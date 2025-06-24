import React, { useState, useCallback } from 'react';
import { FloatingOverlay } from './components/FloatingOverlay';
import './App.css';

// Check if running in Tauri
declare global {
  interface Window {
    __TAURI__?: {
      core: {
        invoke: (cmd: string, args?: any) => Promise<any>;
      };
      window: {
        getCurrent: () => {
          startDragging: () => Promise<void>;
        };
      };
    };
  }
}

const isTauri = window.__TAURI__ !== undefined;

// Dynamic imports for Tauri modules
let tauriInvoke: any = null;

if (isTauri) {
  import('@tauri-apps/api/core').then(module => {
    tauriInvoke = module.invoke;
  });
}

// Groq API configuration - Use environment variable or prompt user
const GROQ_API_KEY = import.meta.env.VITE_GROQ_API_KEY;

if (!GROQ_API_KEY) {
  throw new Error('Missing VITE_GROQ_API_KEY environment variable');
}

// Removed unused MediaRecorderState interface

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [status, setStatus] = useState('Ready');
  const [showSettings, setShowSettings] = useState(false);

  const updateStatus = useCallback((message: string, type: 'info' | 'success' | 'error' | 'processing' = 'info') => {
    setStatus(message);
    
    // Auto-clear status after 3 seconds for non-persistent states
    if (['success', 'error'].includes(type)) {
      setTimeout(() => {
        setStatus('Ready');
      }, 3000);
    }
  }, []);

  const startRecording = useCallback(async () => {
    try {
      updateStatus('Starting recording...', 'processing');
      console.log('🎤 Starting native recording');
      
      if (isTauri && tauriInvoke) {
        // Set API key first
        try {
          await tauriInvoke('set_groq_api_key', { apiKey: GROQ_API_KEY });
          console.log('✅ API key set');
        } catch (e) {
          console.log('⚠️ API key already set:', e);
        }
        
        // Start recording
        const result = await tauriInvoke('start_groq_recording');
        console.log('✅', result);
        
        setIsRecording(true);
        updateStatus('Recording...', 'processing');
      } else {
        console.log('⚠️ Not in Tauri environment');
        updateStatus('Native recording not available', 'error');
      }
      
    } catch (error: any) {
      console.error('❌ Failed to start recording:', error);
      updateStatus(`Recording failed: ${error}`, 'error');
    }
  }, [updateStatus]);

  const stopRecording = useCallback(async () => {
    try {
      if (isTauri && tauriInvoke) {
        updateStatus('Processing...', 'processing');
        console.log('🛑 Stopping recording');
        
        const audioData = await tauriInvoke('stop_groq_recording') as number[];
        console.log(`📤 Got ${audioData.length} bytes of WAV data`);
        
        setIsRecording(false);
        updateStatus('Transcribing...', 'processing');
        
        const transcription = await tauriInvoke('transcribe_audio', { audioData });
        console.log('🎯 Transcription:', transcription);
        console.log('🔄 STATE: Setting isRecording to FALSE and transcript');
        
        setTranscript(transcription);
        
        // BULLETPROOF clipboard for DEV TIER performance
        try {
          // Method 1: Modern Clipboard API
          await navigator.clipboard.writeText(transcription);
          console.log('📋 DEV TIER: Clipboard success via modern API');
          updateStatus('Ready', 'success');
        } catch (error) {
          console.error('❌ Modern clipboard failed:', error);
          
          // Method 2: Fallback textarea method
          try {
            const textArea = document.createElement('textarea');
            textArea.value = transcription;
            textArea.style.position = 'fixed';
            textArea.style.left = '-999999px';
            textArea.style.top = '-999999px';
            document.body.appendChild(textArea);
            textArea.focus();
            textArea.select();
            
            const success = document.execCommand('copy');
            document.body.removeChild(textArea);
            
            if (success) {
              console.log('📋 DEV TIER: Clipboard success via fallback');
              updateStatus('Ready', 'success');
            } else {
              throw new Error('execCommand copy failed');
            }
          } catch (fallbackError) {
            console.error('❌ Fallback clipboard failed:', fallbackError);
            
            // Method 3: Tauri native clipboard if available
            try {
              if (isTauri && tauriInvoke) {
                await tauriInvoke('copy_to_clipboard', { text: transcription });
                console.log('📋 DEV TIER: Clipboard success via Tauri native');
                updateStatus('Ready', 'success');
              } else {
                throw new Error('Tauri not available');
              }
            } catch (tauriError) {
              console.error('❌ All clipboard methods failed:', tauriError);
              // Show the text for manual copy as last resort
              updateStatus(`❌ Clipboard failed - Copy manually: "${transcription.substring(0, 50)}..."`, 'error');
            }
          }
        }
        
      } else {
        console.log('⚠️ Not in Tauri environment');
        updateStatus('Native recording not available', 'error');
      }
    } catch (error: any) {
      console.error('❌ Failed to stop recording:', error);
      setIsRecording(false);
      updateStatus(`Processing failed: ${error}`, 'error');
    }
  }, [updateStatus]);

  // Clean up unused code - all transcription happens natively now

  const copyText = useCallback(async () => {
    if (transcript) {
      try {
        await navigator.clipboard.writeText(transcript);
        updateStatus('Copied to clipboard!', 'success');
      } catch (error) {
        console.error('Failed to copy text:', error);
        updateStatus('Copy failed', 'error');
      }
    }
  }, [transcript, updateStatus]);

  // REMOVED INJECT TEXT - JUST USE COPY TO CLIPBOARD

  // REMOVED SMART PASTE - CAUSED RECURSION LOOP

  // REMOVED UNUSED openSettings

  // Global recording function for Rust backend to call
  React.useEffect(() => {
    // Expose global recording function to window object
    (window as any).startGlobalRecording = () => {
      console.log('🌍 Global Alt+Alt triggered - starting recording!');
      if (!isRecording) {
        startRecording();
      } else {
        stopRecording();
      }
    };
    
    return () => {
      delete (window as any).startGlobalRecording;
    };
  }, [isRecording, startRecording, stopRecording]);

  // Global hotkey handling
  React.useEffect(() => {
    let altDoubleClickTimer: NodeJS.Timeout | null = null;
    let altClickCount = 0;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Alt + Space to toggle recording
      if (event.code === 'Space' && event.altKey) {
        event.preventDefault();
        if (isRecording) {
          stopRecording();
        } else {
          startRecording();
        }
      }

      // Alt key double-press to start recording
      if (event.code === 'AltLeft' || event.code === 'AltRight') {
        altClickCount++;
        
        if (altDoubleClickTimer) {
          clearTimeout(altDoubleClickTimer);
        }
        
        altDoubleClickTimer = setTimeout(() => {
          if (altClickCount >= 2 && !isRecording) {
            console.log('🎤 Alt double-press - starting recording');
            startRecording();
          }
          altClickCount = 0;
        }, 400);
      }

      // Spacebar to stop recording
      if (event.code === 'Space' && isRecording && !event.altKey) {
        event.preventDefault();
        console.log('🛑 Spacebar - stopping recording');
        stopRecording();
      }
      
      // Escape to close settings
      if (event.code === 'Escape' && showSettings) {
        event.preventDefault();
        setShowSettings(false);
      }
      
      // Enter to close settings
      if (event.code === 'Enter' && showSettings) {
        event.preventDefault();
        setShowSettings(false);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isRecording, startRecording, stopRecording, showSettings]);

  return (
    <>
      {/* Just the floating overlay - no background container */}
      <FloatingOverlay
        onStartRecording={startRecording}
        onStopRecording={stopRecording}
        isRecording={isRecording}
        transcript={transcript}
        status={status}
        onCopyText={copyText}
      />

      {/* Settings modal */}
      {showSettings && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
          <div className="glass rounded-xl p-6 max-w-md w-full mx-4 floating-shadow">
            <h2 className="text-white text-xl font-bold mb-4">Settings</h2>
            <p className="text-white/80 text-sm mb-4">Settings panel coming soon...</p>
            <button
              onClick={() => setShowSettings(false)}
              className="w-full bg-blue-500 hover:bg-blue-600 text-white font-medium py-2 px-4 rounded-lg transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      )}
    </>
  );
}

export default App;