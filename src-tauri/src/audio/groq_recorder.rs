use cpal::{traits::*, Device, StreamConfig, SampleRate, Stream};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::io::Cursor;

pub struct GroqRecorder {
    device: Device,
    stream: Option<Stream>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<Mutex<bool>>,
}

impl GroqRecorder {
    pub fn new() -> Result<Self, String> {
        println!("🎤 Initializing Groq-compatible audio recorder");
        
        let host = cpal::default_host();
        println!("🔧 Audio host: {}", host.id().name());
        
        let device = host.default_input_device()
            .ok_or("No microphone found")?;
        
        println!("🎧 Using device: {}", device.name().unwrap_or_else(|_| "Unknown".to_string()));
        
        // Test Groq-compatible configuration
        let _config = StreamConfig {
            channels: 1,                    // ✅ Mono - Groq requirement
            sample_rate: SampleRate(16000), // ✅ 16KHz - Groq's native format
            buffer_size: cpal::BufferSize::Fixed(1024),
        };
        
        // Verify device supports Groq format
        let supported_configs = device.supported_input_configs()
            .map_err(|e| format!("Cannot query device configs: {}", e))?;
        
        let groq_compatible = supported_configs
            .filter(|c| {
                c.channels() == 1 && 
                c.min_sample_rate() <= SampleRate(16000) &&
                c.max_sample_rate() >= SampleRate(16000)
            })
            .next()
            .ok_or("Device doesn't support 16KHz mono recording")?;
        
        println!("✅ Device supports Groq format: {:?}", groq_compatible);
        
        Ok(Self {
            device,
            stream: None,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(Mutex::new(false)),
        })
    }
    
    pub fn start_recording(&mut self) -> Result<(), String> {
        println!("🚀 Starting recording with Groq specs: 16KHz mono");
        
        let config = StreamConfig {
            channels: 1,                    // Mono
            sample_rate: SampleRate(16000), // 16KHz
            buffer_size: cpal::BufferSize::Fixed(1024),
        };
        
        // Clear previous recording
        {
            let mut buffer = self.audio_buffer.lock()
                .map_err(|e| format!("Buffer lock error: {}", e))?;
            buffer.clear();
        }
        
        let audio_buffer = Arc::clone(&self.audio_buffer);
        let is_recording = Arc::clone(&self.is_recording);
        
        let stream = self.device.build_input_stream(
            &config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                if let Ok(recording) = is_recording.lock() {
                    if *recording {
                        if let Ok(mut buffer) = audio_buffer.lock() {
                            buffer.extend_from_slice(data);
                            
                            // Log progress every second
                            if buffer.len() % 16000 == 0 && buffer.len() > 0 {
                                println!("📊 Recorded {} seconds", buffer.len() / 16000);
                            }
                        }
                    }
                }
            },
            |err| eprintln!("❌ Audio stream error: {}", err),
            None,
        ).map_err(|e| format!("Failed to build input stream: {}", e))?;
        
        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;
        
        // Set recording flag
        {
            let mut recording = self.is_recording.lock()
                .map_err(|e| format!("Recording flag lock error: {}", e))?;
            *recording = true;
        }
        
        self.stream = Some(stream);
        
        println!("✅ Recording started successfully");
        Ok(())
    }
    
    pub fn stop_recording(&mut self) -> Result<Vec<u8>, String> {
        println!("🛑 Stopping recording and generating WAV");
        
        // Stop recording
        {
            let mut recording = self.is_recording.lock()
                .map_err(|e| format!("Recording flag lock error: {}", e))?;
            *recording = false;
        }
        
        // Drop the stream to stop audio capture
        self.stream.take();
        
        // Get the recorded audio data
        let audio_data = {
            let mut buffer = self.audio_buffer.lock()
                .map_err(|e| format!("Buffer lock error: {}", e))?;
            let data = buffer.clone();
            buffer.clear();
            data
        };
        
        if audio_data.is_empty() {
            return Err("No audio data recorded".to_string());
        }
        
        let duration = audio_data.len() as f32 / 16000.0;
        println!("📊 Processing {} samples ({:.2}s duration)", audio_data.len(), duration);
        
        // Check minimum duration for Groq (0.01 seconds minimum)
        if duration < 0.01 {
            return Err("Recording too short (minimum 0.01 seconds)".to_string());
        }
        
        // Generate WAV with Groq's exact specifications
        let wav_data = self.generate_groq_wav(&audio_data)?;
        
        println!("✅ Generated {:.2}KB WAV for Groq", wav_data.len() as f32 / 1024.0);
        Ok(wav_data)
    }
    
    fn generate_groq_wav(&self, samples: &[f32]) -> Result<Vec<u8>, String> {
        // Groq's exact WAV specifications
        let spec = WavSpec {
            channels: 1,                           // Mono
            sample_rate: 16000,                    // 16KHz
            bits_per_sample: 16,                   // 16-bit
            sample_format: hound::SampleFormat::Int,
        };
        
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec)
                .map_err(|e| format!("WAV writer error: {}", e))?;
            
            // Convert f32 to i16 with proper scaling and clamping
            for &sample in samples {
                // Clamp to [-1.0, 1.0] to prevent clipping
                let clamped = sample.max(-1.0).min(1.0);
                // Convert to 16-bit signed integer
                let sample_i16 = (clamped * i16::MAX as f32) as i16;
                writer.write_sample(sample_i16)
                    .map_err(|e| format!("WAV write error: {}", e))?;
            }
            
            writer.finalize()
                .map_err(|e| format!("WAV finalize error: {}", e))?;
        }
        
        let wav_data = cursor.into_inner();
        
        // Debug audio quality
        self.debug_audio_quality(samples, &wav_data);
        
        Ok(wav_data)
    }
    
    fn debug_audio_quality(&self, samples: &[f32], wav_data: &[u8]) {
        let peak_amplitude = samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        
        println!("🔍 Audio Quality Debug:");
        println!("  Duration: {:.2}s", samples.len() as f32 / 16000.0);
        println!("  Sample rate: 16000 Hz");
        println!("  Channels: 1 (mono)");
        println!("  Peak amplitude: {:.3}", peak_amplitude);
        println!("  RMS level: {:.3}", rms);
        println!("  WAV file size: {:.2}KB", wav_data.len() as f32 / 1024.0);
        
        // Quality warnings
        if peak_amplitude < 0.01 {
            println!("⚠️  WARNING: Very low audio level - microphone might be muted");
        } else if peak_amplitude > 0.95 {
            println!("⚠️  WARNING: Audio clipping detected - reduce input level");
        } else {
            println!("✅ Audio levels look good for speech recognition");
        }
    }
    
    pub fn is_recording(&self) -> bool {
        self.is_recording.lock().map(|r| *r).unwrap_or(false)
    }
}