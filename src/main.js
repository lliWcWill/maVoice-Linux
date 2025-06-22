// Use global Tauri API (works better for static HTML)
const { invoke } = window.__TAURI__.core;

console.log('Tauri API loaded successfully');

class AquaVoiceApp {
  constructor() {
    this.isRecording = false;
    this.currentTranscript = '';
    this.mediaRecorder = null;
    this.audioChunks = [];
    this.groqApiKey = localStorage.getItem('groq_api_key') || '';
    
    this.initializeElements();
    this.attachEventListeners();
    this.initializeApp();
  }

  initializeElements() {
    this.elements = {
      recordBtn: document.getElementById('record-btn'),
      recordText: document.querySelector('.record-text'),
      recordIcon: document.querySelector('.record-icon'),
      recordingStatus: document.getElementById('recording-status'),
      statusIndicator: document.getElementById('status-indicator'),
      transcriptBox: document.getElementById('transcript-box'),
      injectBtn: document.getElementById('inject-btn'),
      copyBtn: document.getElementById('copy-btn'),
      groqApiKeyInput: document.getElementById('groq-api-key'),
      saveKeyBtn: document.getElementById('save-key-btn'),
      displayServer: document.getElementById('display-server'),
      activeWindow: document.getElementById('active-window')
    };
  }

  attachEventListeners() {
    // Record button
    this.elements.recordBtn.addEventListener('click', () => this.toggleRecording());
    
    // Action buttons
    this.elements.injectBtn.addEventListener('click', () => this.injectText());
    this.elements.copyBtn.addEventListener('click', () => this.copyText());
    
    // Settings
    this.elements.saveKeyBtn.addEventListener('click', () => this.saveApiKey());
    this.elements.groqApiKeyInput.value = this.groqApiKey;
    
    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => this.handleKeyboard(e));
    
    // Update active window info periodically
    setInterval(() => this.updateActiveWindow(), 2000);
  }

  async initializeApp() {
    try {
      // Set API key if available
      if (this.groqApiKey) {
        await invoke('set_groq_api_key', { apiKey: this.groqApiKey });
        this.updateStatus('Ready', 'success');
      } else {
        this.updateStatus('API Key Required', 'warning');
      }
      
      // Detect display server
      await this.detectDisplayServer();
      
      // Update active window
      await this.updateActiveWindow();
      
    } catch (error) {
      console.error('Initialization error:', error);
      this.updateStatus('Initialization Error', 'error');
    }
  }

  async toggleRecording() {
    try {
      if (!this.isRecording) {
        await this.startRecording();
      } else {
        await this.stopRecording();
      }
    } catch (error) {
      console.error('Recording error:', error);
      this.updateStatus('Recording Error', 'error');
    }
  }

  async startRecording() {
    if (!this.groqApiKey) {
      alert('Please set your Groq API key in settings first.');
      return;
    }

    try {
      // Request microphone access
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      
      // Create MediaRecorder with WebM format (same as your extension)
      const options = { mimeType: 'audio/webm' };
      this.mediaRecorder = new MediaRecorder(stream, options);
      this.audioChunks = [];

      // Collect audio data
      this.mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          this.audioChunks.push(event.data);
        }
      };

      // Handle recording stop
      this.mediaRecorder.onstop = async () => {
        const audioBlob = new Blob(this.audioChunks, { type: 'audio/webm' });
        
        // Update UI first
        this.isRecording = false;
        this.elements.recordBtn.classList.remove('recording');
        this.elements.recordText.textContent = 'Start Recording';
        this.elements.recordIcon.textContent = '🔴';
        this.elements.recordingStatus.style.display = 'none';
        
        await this.transcribeAudio(audioBlob);
        
        // Release microphone
        stream.getTracks().forEach(track => track.stop());
      };

      // Start recording
      this.mediaRecorder.start();
      this.isRecording = true;
      
      this.elements.recordBtn.classList.add('recording');
      this.elements.recordText.textContent = 'Stop Recording';
      this.elements.recordIcon.textContent = '⏹️';
      this.elements.recordingStatus.style.display = 'flex';
      
      this.updateStatus('Recording...', 'recording');
      this.showTranscriptPlaceholder();
      
    } catch (error) {
      console.error('Error starting recording:', error);
      this.updateStatus('Microphone Error', 'error');
    }
  }

  async stopRecording() {
    if (this.mediaRecorder && this.isRecording) {
      this.updateStatus('Processing...', 'processing');
      this.mediaRecorder.stop();
      // UI updates happen in onstop handler
    }
  }

  async transcribeAudio(audioBlob) {
    if (!audioBlob || audioBlob.size === 0) {
      this.updateStatus('No audio data', 'error');
      return;
    }

    try {
      this.updateStatus('Transcribing...', 'processing');
      
      // Convert blob to array buffer then to array for Rust
      const arrayBuffer = await audioBlob.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);
      const webm_data = Array.from(uint8Array);
      
      console.log(`🔥 Sending ${webm_data.length} bytes of WebM data to Rust backend`);
      
      // Use Rust backend to process WebM and send to Groq
      const transcript = await invoke('process_webm_audio', {
        webmData: webm_data
      });
      
      this.currentTranscript = transcript.trim();
      this.showTranscript(this.currentTranscript);
      this.enableActionButtons();
      this.updateStatus('Transcribed', 'success');
      
    } catch (error) {
      console.error('Transcription error:', error);
      this.updateStatus('Transcription Failed', 'error');
      this.showTranscript(`Error: ${error}`);
    }
  }

  async injectText() {
    if (!this.currentTranscript) return;
    
    console.log('🔥 injectText called with:', this.currentTranscript);
    
    try {
      const result = await invoke('inject_text', { text: this.currentTranscript });
      console.log('✅ Injection result:', result);
      this.updateStatus('Text Injected', 'success');
    } catch (error) {
      console.error('❌ Injection error:', error);
      this.updateStatus('Injection Failed', 'error');
    }
  }

  async copyText() {
    if (!this.currentTranscript) return;
    
    try {
      await navigator.clipboard.writeText(this.currentTranscript);
      this.updateStatus('Copied to Clipboard', 'success');
    } catch (error) {
      console.error('Copy error:', error);
      this.updateStatus('Copy Failed', 'error');
    }
  }

  async saveApiKey() {
    const apiKey = this.elements.groqApiKeyInput.value.trim();
    if (!apiKey) {
      alert('Please enter a valid API key.');
      return;
    }
    
    try {
      await invoke('set_groq_api_key', { apiKey });
      this.groqApiKey = apiKey;
      localStorage.setItem('groq_api_key', apiKey);
      this.updateStatus('API Key Saved', 'success');
    } catch (error) {
      console.error('API key error:', error);
      this.updateStatus('API Key Error', 'error');
    }
  }

  async detectDisplayServer() {
    try {
      const windowInfo = await invoke('get_active_window_info');
      if (windowInfo.title.includes('Wayland')) {
        this.elements.displayServer.textContent = 'Wayland';
      } else {
        this.elements.displayServer.textContent = 'X11';
      }
    } catch (error) {
      this.elements.displayServer.textContent = 'Unknown';
    }
  }

  async updateActiveWindow() {
    try {
      const windowInfo = await invoke('get_active_window_info');
      const displayText = windowInfo.title.length > 30 
        ? windowInfo.title.substring(0, 30) + '...'
        : windowInfo.title;
      this.elements.activeWindow.textContent = displayText;
    } catch (error) {
      this.elements.activeWindow.textContent = 'Unknown';
    }
  }

  handleKeyboard(event) {
    // Space key for recording toggle
    if (event.code === 'Space' && !event.target.matches('input')) {
      event.preventDefault();
      this.toggleRecording();
    }
    
    // Enter key for text injection
    if (event.code === 'Enter' && this.currentTranscript) {
      event.preventDefault();
      this.injectText();
    }
    
    // Ctrl+C for copy
    if (event.ctrlKey && event.code === 'KeyC' && this.currentTranscript) {
      event.preventDefault();
      this.copyText();
    }
  }

  showTranscriptPlaceholder() {
    this.elements.transcriptBox.innerHTML = '<div class="transcript-placeholder">Listening...</div>';
    this.disableActionButtons();
  }

  showTranscript(text) {
    this.elements.transcriptBox.innerHTML = `<div class="transcript-text">${text}</div>`;
  }

  enableActionButtons() {
    this.elements.injectBtn.disabled = false;
    this.elements.copyBtn.disabled = false;
  }

  disableActionButtons() {
    this.elements.injectBtn.disabled = true;
    this.elements.copyBtn.disabled = true;
  }

  updateStatus(message, type = 'info') {
    this.elements.statusIndicator.textContent = message;
    this.elements.statusIndicator.className = 'status-indicator';
    
    switch (type) {
      case 'success':
        this.elements.statusIndicator.style.background = 'var(--secondary-color)';
        break;
      case 'error':
        this.elements.statusIndicator.style.background = 'var(--danger-color)';
        break;
      case 'warning':
        this.elements.statusIndicator.style.background = 'var(--warning-color)';
        break;
      case 'processing':
      case 'recording':
        this.elements.statusIndicator.style.background = 'var(--primary-color)';
        break;
      default:
        this.elements.statusIndicator.style.background = 'var(--secondary-color)';
    }
    
    // Auto-clear status after 3 seconds for non-persistent states
    if (['success', 'error'].includes(type)) {
      setTimeout(() => {
        if (this.elements.statusIndicator.textContent === message) {
          this.updateStatus('Ready', 'info');
        }
      }, 3000);
    }
  }
}

// Initialize app when DOM is loaded
window.addEventListener("DOMContentLoaded", () => {
  new AquaVoiceApp();
});