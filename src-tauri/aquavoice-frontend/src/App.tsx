import React, { useState, useCallback, useRef } from 'react';
import { FloatingOverlay } from './components/FloatingOverlay';
import './App.css';

// Check if running in Tauri
declare global {
  interface Window {
    __TAURI__?: any;
  }
}

const isTauri = window.__TAURI__ !== undefined;

// Dynamic imports for Tauri modules
let tauriHttp: any = null;
let tauriInvoke: any = null;
let tauriLog: any = null;

if (isTauri) {
  import('@tauri-apps/plugin-http').then(module => {
    tauriHttp = module;
  });
  import('@tauri-apps/api/core').then(module => {
    tauriInvoke = module.invoke;
  });
  import('@tauri-apps/plugin-log').then(module => {
    tauriLog = module;
    // Set up console redirection to Tauri logs
    const { attachConsole } = module;
    attachConsole().then(() => {
      console.log('✅ Tauri console logging attached');
    });
  });
}

// Groq API configuration
const GROQ_API_KEY = import.meta.env.VITE_GROQ_API_KEY || 'your-groq-api-key-here';
const GROQ_API_URL = 'https://api.groq.com/openai/v1/audio/transcriptions';

interface MediaRecorderState {
  recorder: MediaRecorder | null;
  chunks: Blob[];
  stream: MediaStream | null;
}

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [status, setStatus] = useState('Ready');
  const [showSettings, setShowSettings] = useState(false);
  
  const mediaRecorderRef = useRef<MediaRecorderState>({
    recorder: null,
    chunks: [],
    stream: null,
  });

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
      updateStatus('Starting Groq recording...', 'processing');
      console.log('🎤 Starting Groq-compatible recording (16KHz mono)');
      
      if (isTauri && tauriInvoke) {
        // Use native Groq-optimized recording
        const result = await tauriInvoke('start_groq_recording');
        console.log('✅', result);
        
        setIsRecording(true);
        updateStatus('Recording (16KHz mono)...', 'processing');
      } else {
        // Fallback to WebRTC for web version (for testing)
        console.log('🌐 Using WebRTC fallback for web version');
        
        const stream = await navigator.mediaDevices.getUserMedia({ 
          audio: {
            sampleRate: 16000,  // Match Groq's preferred rate
            channelCount: 1,    // Mono
            echoCancellation: true,
            noiseSuppression: true,
          } 
        });
        
        const recorder = new MediaRecorder(stream, { 
          mimeType: 'audio/webm;codecs=opus',
          audioBitsPerSecond: 128000,
        });
        
        const chunks: Blob[] = [];
        
        recorder.ondataavailable = (event) => {
          if (event.data.size > 0) {
            chunks.push(event.data);
          }
        };
        
        recorder.onstop = async () => {
          const audioBlob = new Blob(chunks, { type: 'audio/webm' });
          await transcribeWithGroq(audioBlob);
          
          stream.getTracks().forEach(track => track.stop());
          mediaRecorderRef.current = { recorder: null, chunks: [], stream: null };
        };
        
        recorder.start();
        mediaRecorderRef.current = { recorder, chunks, stream };
        
        setIsRecording(true);
        updateStatus('Recording (WebRTC)...', 'processing');
      }
      
    } catch (error: any) {
      console.error('❌ Failed to start recording:', error);
      updateStatus(`Recording failed: ${error}`, 'error');
    }
  }, [updateStatus]);

  const stopRecording = useCallback(async () => {
    try {
      if (isTauri && tauriInvoke) {
        // Use native Groq recording
        updateStatus('Processing audio...', 'processing');
        console.log('🛑 Stopping Groq recording');
        
        const audioData = await tauriInvoke('stop_groq_recording') as number[];
        const audioBytes = new Uint8Array(audioData);
        
        console.log(`📤 Got ${audioBytes.length} bytes of WAV data`);
        
        setIsRecording(false);
        await transcribeWithGroq(new Blob([audioBytes], { type: 'audio/wav' }));
        
      } else {
        // WebRTC fallback
        const { recorder } = mediaRecorderRef.current;
        
        if (recorder && recorder.state === 'recording') {
          updateStatus('Processing...', 'processing');
          recorder.stop();
          setIsRecording(false);
        }
      }
    } catch (error: any) {
      console.error('❌ Failed to stop recording:', error);
      setIsRecording(false);
      updateStatus(`Stop failed: ${error}`, 'error');
    }
  }, [updateStatus]);

  const transcribeWithGroq = useCallback(async (audioBlob: Blob) => {
    try {
      updateStatus('Transcribing with Groq...', 'processing');
      
      const audioType = audioBlob.type.includes('wav') ? 'WAV' : 'WebM';
      console.log(`📤 Sending ${(audioBlob.size / 1024).toFixed(2)}KB ${audioType} to Groq`);
      
      // Create FormData for Groq API with optimized parameters
      const formData = new FormData();
      const filename = audioBlob.type.includes('wav') ? 'recording.wav' : 'recording.webm';
      formData.append('file', audioBlob, filename);
      formData.append('model', 'whisper-large-v3-turbo');
      formData.append('response_format', 'verbose_json');
      formData.append('timestamp_granularities', '["word", "segment"]');
      formData.append('language', 'en');
      formData.append('temperature', '0.0');
      
      let response: Response;
      
      if (isTauri && tauriHttp) {
        // Use Tauri HTTP client for desktop app
        const { fetch: tauriFetch } = tauriHttp;
        response = await tauriFetch(GROQ_API_URL, {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${GROQ_API_KEY}`,
          },
          body: formData,
        });
      } else {
        // Use regular fetch for web version
        response = await fetch(GROQ_API_URL, {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${GROQ_API_KEY}`,
          },
          body: formData,
        });
      }

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error?.message || `HTTP ${response.status}: ${response.statusText}`);
      }

      const result = await response.json();
      
      // ✅ GROQ QUALITY MONITORING
      if (result.segments && result.segments.length > 0) {
        const segment = result.segments[0];
        const confidence = segment.avg_logprob || 0;
        const speechProbability = 1 - (segment.no_speech_prob || 0);
        const compressionRatio = segment.compression_ratio || 0;
        
        console.log('📊 Groq Quality Metrics:', {
          confidence: confidence.toFixed(3),
          speech_probability: speechProbability.toFixed(3),
          compression_ratio: compressionRatio.toFixed(3),
          duration: `${segment.end - segment.start}s`,
          format: audioType
        });
        
        // Quality assessment based on Groq docs
        if (confidence < -0.5) {
          console.warn('⚠️ Low confidence transcription detected');
        }
        if (speechProbability < 0.8) {
          console.warn('⚠️ Low speech probability detected');
        }
        if (compressionRatio < 1.0 || compressionRatio > 3.0) {
          console.warn('⚠️ Unusual compression ratio detected');
        }
        
        // Log quality status
        if (confidence > -0.2 && speechProbability > 0.95) {
          console.log('✅ Excellent transcription quality');
        } else if (confidence > -0.5 && speechProbability > 0.8) {
          console.log('✅ Good transcription quality');
        } else {
          console.log('⚠️ Fair transcription quality');
        }
      }
      
      const transcribedText = result.text || '';
      console.log('🎯 === TRANSCRIPTION RESULT ===');
      console.log('📝 Text:', transcribedText);
      console.log('📊 Length:', transcribedText.length, 'characters');
      console.log('🎯 ==========================')
      
      setTranscript(transcribedText.trim());
      updateStatus('Transcribed!', 'success');
      
    } catch (error: any) {
      console.error('❌ Groq transcription failed:', error);
      updateStatus('Transcription failed', 'error');
    }
  }, [updateStatus]);

  // Keep the old function for WebRTC fallback
  const transcribeAudio = transcribeWithGroq;

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

  const injectText = useCallback(async () => {
    if (transcript) {
      try {
        if (isTauri && tauriInvoke) {
          // Use Tauri command to inject text
          await tauriInvoke('inject_text', { text: transcript });
          updateStatus('Text injected!', 'success');
        } else {
          // Fallback for web version
          console.log('🔥 Would inject text:', transcript);
          updateStatus('Text injection not available in web mode', 'error');
        }
      } catch (error) {
        console.error('Failed to inject text:', error);
        updateStatus('Injection failed', 'error');
      }
    }
  }, [transcript, updateStatus]);

  const openSettings = useCallback(() => {
    setShowSettings(true);
  }, []);

  // Global hotkey handling
  React.useEffect(() => {
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
        onInjectText={injectText}
        onCopyText={copyText}
        onOpenSettings={openSettings}
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