// Mock audio recorder for WSL2 testing
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

pub struct MockGroqRecorder {
    recording: Arc<Mutex<bool>>,
}

impl MockGroqRecorder {
    pub fn new() -> Result<Self, String> {
        println!("🎤 Using MOCK audio recorder for WSL2 testing");
        Ok(Self {
            recording: Arc::new(Mutex::new(false)),
        })
    }

    pub fn start_recording(&self) -> Result<(), String> {
        let mut recording = self.recording.lock().unwrap();
        *recording = true;
        println!("🔴 Mock recording started");
        Ok(())
    }

    pub async fn stop_recording(&self) -> Result<Vec<u8>, String> {
        let mut recording = self.recording.lock().unwrap();
        *recording = false;
        println!("⏹️ Mock recording stopped");
        
        // Simulate processing delay
        sleep(Duration::from_millis(500)).await;
        
        // Return a minimal valid WAV file (silent audio)
        // WAV header for 1 second of silence at 16kHz mono
        let mut wav_data = vec![
            // RIFF header
            b'R', b'I', b'F', b'F',
            0x24, 0x7D, 0x00, 0x00, // File size - 8
            b'W', b'A', b'V', b'E',
            // fmt chunk
            b'f', b'm', b't', b' ',
            0x10, 0x00, 0x00, 0x00, // Subchunk size (16)
            0x01, 0x00, // Audio format (PCM)
            0x01, 0x00, // Number of channels (1)
            0x80, 0x3E, 0x00, 0x00, // Sample rate (16000)
            0x00, 0x7D, 0x00, 0x00, // Byte rate
            0x02, 0x00, // Block align
            0x10, 0x00, // Bits per sample (16)
            // data chunk
            b'd', b'a', b't', b'a',
            0x00, 0x7D, 0x00, 0x00, // Data size
        ];
        
        // Add 1 second of silence (16000 samples * 2 bytes per sample)
        wav_data.extend(vec![0u8; 32000]);
        
        Ok(wav_data)
    }

    pub async fn get_audio_level(&self) -> f32 {
        // Return random levels for visual feedback
        if *self.recording.lock().unwrap() {
            rand::random::<f32>() * 0.5
        } else {
            0.0
        }
    }
}

// Re-export as GroqRecorder for compatibility
pub type GroqRecorder = MockGroqRecorder;