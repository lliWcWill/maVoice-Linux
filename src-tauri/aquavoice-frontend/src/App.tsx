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

// Groq API configuration - Use environment variable or localStorage
const getApiKey = () => {
  const envKey = import.meta.env.VITE_GROQ_API_KEY;
  const storedKey = localStorage.getItem('groq_api_key');
  return storedKey || envKey || '';
};

const getSettings = () => {
  const stored = localStorage.getItem('groq_settings');
  return stored ? JSON.parse(stored) : {
    model: 'whisper-large-v3-turbo',
    language: 'en',
    dictionary: '',
    temperature: 0,
    responseFormat: 'json'
  };
};

// const GROQ_API_KEY = getApiKey(); // Removed - using getApiKey() directly

// Removed unused MediaRecorderState interface

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [status, setStatus] = useState('Ready');
  const [showSettings, setShowSettings] = useState(!getApiKey()); // Show settings if no API key
  const [apiKey, setApiKey] = useState(getApiKey());
  const [apiKeyInput, setApiKeyInput] = useState(getApiKey());
  const [settings, setSettings] = useState(getSettings());
  const [tempSettings, setTempSettings] = useState(getSettings());

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
      // Check API key before starting
      const currentApiKey = getApiKey();
      if (!currentApiKey) {
        setShowSettings(true);
        updateStatus('Please set your Groq API key', 'error');
        return;
      }
      
      updateStatus('Starting recording...', 'processing');
      console.log('🎤 Starting native recording');
      
      if (isTauri && tauriInvoke) {
        // Set API key first
        try {
          await tauriInvoke('set_groq_api_key', { apiKey: currentApiKey });
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
      
      // Ctrl+, to open settings
      if (event.code === 'Comma' && event.ctrlKey) {
        event.preventDefault();
        setShowSettings(true);
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
        onOpenSettings={() => setShowSettings(true)}
      />

      {/* Settings modal */}
      {showSettings && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
          <div className="glass rounded-xl p-6 max-w-lg w-full mx-4 floating-shadow max-h-[90vh] overflow-y-auto">
            <h2 className="text-white text-xl font-bold mb-4">⚡ maVoice Settings</h2>
            
            {/* API Key */}
            <div className="mb-4">
              <label className="block text-white/90 text-sm font-medium mb-2">
                Groq API Key
              </label>
              <input
                type="password"
                value={apiKeyInput}
                onChange={(e) => setApiKeyInput(e.target.value)}
                placeholder="gsk_..."
                className="w-full bg-white/10 border border-white/20 rounded-lg px-3 py-2 text-white placeholder-white/50 focus:outline-none focus:border-blue-400 focus:ring-1 focus:ring-blue-400"
              />
              <p className="text-white/60 text-xs mt-1">
                Get your API key from <a href="https://console.groq.com" target="_blank" rel="noopener noreferrer" className="text-blue-400 hover:underline">console.groq.com</a>
              </p>
            </div>
            
            {/* Model Selection */}
            <div className="mb-4">
              <label className="block text-white/90 text-sm font-medium mb-2">
                Whisper Model
              </label>
              <select
                value={tempSettings.model}
                onChange={(e) => setTempSettings({...tempSettings, model: e.target.value})}
                className="w-full bg-white/10 border border-white/20 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-400 focus:ring-1 focus:ring-blue-400"
              >
                <option value="whisper-large-v3-turbo">Whisper Large v3 Turbo (Fastest)</option>
                <option value="whisper-large-v3">Whisper Large v3 (Balanced)</option>
                <option value="distil-whisper-large-v3-en">Distil Whisper Large v3 EN (English Only)</option>
              </select>
              <p className="text-white/60 text-xs mt-1">
                Turbo is fastest, v3 is most accurate, Distil is optimized for English
              </p>
            </div>
            
            {/* Language */}
            <div className="mb-4">
              <label className="block text-white/90 text-sm font-medium mb-2">
                Language (ISO-639-1)
              </label>
              <input
                type="text"
                value={tempSettings.language}
                onChange={(e) => setTempSettings({...tempSettings, language: e.target.value})}
                placeholder="en, es, fr, de..."
                className="w-full bg-white/10 border border-white/20 rounded-lg px-3 py-2 text-white placeholder-white/50 focus:outline-none focus:border-blue-400 focus:ring-1 focus:ring-blue-400"
              />
              <p className="text-white/60 text-xs mt-1">
                Language code improves accuracy (en=English, es=Spanish, etc.)
              </p>
            </div>
            
            {/* Dictionary/Prompt */}
            <div className="mb-4">
              <label className="block text-white/90 text-sm font-medium mb-2">
                Dictionary / Custom Prompt
              </label>
              <textarea
                value={tempSettings.dictionary}
                onChange={(e) => setTempSettings({...tempSettings, dictionary: e.target.value})}
                placeholder="Add custom words, names, or style instructions...\nExample: John Smith, API key, JavaScript"
                rows={3}
                className="w-full bg-white/10 border border-white/20 rounded-lg px-3 py-2 text-white placeholder-white/50 focus:outline-none focus:border-blue-400 focus:ring-1 focus:ring-blue-400 resize-none"
              />
              <p className="text-white/60 text-xs mt-1">
                Help Whisper spell names, technical terms, or set transcription style (max 224 tokens)
              </p>
            </div>
            
            {/* Temperature */}
            <div className="mb-4">
              <label className="block text-white/90 text-sm font-medium mb-2">
                Temperature: {tempSettings.temperature}
              </label>
              <input
                type="range"
                min="0"
                max="1"
                step="0.1"
                value={tempSettings.temperature}
                onChange={(e) => setTempSettings({...tempSettings, temperature: parseFloat(e.target.value)})}
                className="w-full"
              />
              <div className="flex justify-between text-white/60 text-xs mt-1">
                <span>Deterministic (0)</span>
                <span>Creative (1)</span>
              </div>
            </div>
            
            {/* Response Format */}
            <div className="mb-6">
              <label className="block text-white/90 text-sm font-medium mb-2">
                Response Format
              </label>
              <select
                value={tempSettings.responseFormat}
                onChange={(e) => setTempSettings({...tempSettings, responseFormat: e.target.value})}
                className="w-full bg-white/10 border border-white/20 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-400 focus:ring-1 focus:ring-blue-400"
              >
                <option value="json">JSON (Default)</option>
                <option value="text">Text Only</option>
                <option value="verbose_json">Verbose JSON (with timestamps)</option>
              </select>
            </div>
            
            {/* Action Buttons */}
            <div className="flex gap-2">
              <button
                onClick={() => {
                  // Save API key
                  if (apiKeyInput.trim()) {
                    localStorage.setItem('groq_api_key', apiKeyInput.trim());
                    setApiKey(apiKeyInput.trim());
                  }
                  
                  // Save settings
                  localStorage.setItem('groq_settings', JSON.stringify(tempSettings));
                  setSettings(tempSettings);
                  
                  updateStatus('Settings saved!', 'success');
                  setShowSettings(false);
                }}
                className="flex-1 bg-blue-500 hover:bg-blue-600 text-white font-medium py-2 px-4 rounded-lg transition-colors"
              >
                Save Settings
              </button>
              <button
                onClick={() => {
                  setTempSettings(settings);
                  setApiKeyInput(apiKey);
                  setShowSettings(false);
                }}
                className="flex-1 bg-gray-600 hover:bg-gray-700 text-white font-medium py-2 px-4 rounded-lg transition-colors"
              >
                Cancel
              </button>
            </div>
            
            {/* Keyboard Shortcuts */}
            <div className="mt-4 pt-4 border-t border-white/20">
              <p className="text-white/60 text-xs text-center">
                🎯 Shortcuts: Ctrl+, (settings) • Alt+Space (record) • Double Alt (quick record)
              </p>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export default App;