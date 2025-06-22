use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, Sample, SampleFormat, Stream, StreamConfig, SampleRate,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

pub struct GroqRecorder {
    host: Host,
    input_device: Device,
    config: StreamConfig,
    stream: Option<Stream>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    sample_sender: Option<Sender<f32>>,
    sample_receiver: Option<Receiver<f32>>,
}

impl GroqRecorder {
    pub fn new() -> Result<Self, String> {
        println!("🎤 Initializing Groq-compatible audio recorder");
        
        let host = cpal::default_host();
        println!("🔧 Audio host: {}", host.id().name());
        
        let input_device = host
            .default_input_device()
            .ok_or("No input device available")?;
        
        println!("🎧 Using device: {}", input_device.name().unwrap_or("Unknown".to_string()));
        
        // Configure for Groq requirements: 16KHz mono
        let config = StreamConfig {
            channels: 1,                    // Mono - exactly what Groq expects
            sample_rate: SampleRate(16000), // 16KHz - Groq's native format
            buffer_size: cpal::BufferSize::Fixed(1024),
        };
        
        // Verify device supports this configuration
        match input_device.supported_input_configs() {
            Ok(configs) => {
                let supported = configs
                    .filter(|c| c.channels() == 1 && c.min_sample_rate() <= SampleRate(16000) && c.max_sample_rate() >= SampleRate(16000))
                    .next();
                
                if supported.is_none() {
                    return Err("Device doesn't support Groq format (16KHz mono)".to_string());
                }
                println!("✅ Device supports Groq format");
            }
            Err(e) => {
                println!("⚠️ Couldn't verify device capabilities: {}", e);
            }
        }
        
        let audio_data = Arc::new(Mutex::new(Vec::new()));
        let (sample_sender, sample_receiver) = unbounded();
        
        Ok(Self {
            host,
            input_device,
            config,
            stream: None,
            audio_data,
            sample_sender: Some(sample_sender),
            sample_receiver: Some(sample_receiver),
        })
    }
    
    pub fn start_recording(&mut self) -> Result<String, String> {
        if self.stream.is_some() {
            return Err("Already recording".to_string());
        }
        
        println!("🚀 Starting recording with Groq specs: 16KHz mono");
        
        // Clear previous audio data
        if let Ok(mut data) = self.audio_data.lock() {
            data.clear();
        }
        
        let audio_data = self.audio_data.clone();
        let sender = self.sample_sender.as_ref().unwrap().clone();
        
        let stream = self.input_device
            .build_input_stream(
                &self.config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Send samples for processing
                    for &sample in data {
                        let _ = sender.send(sample);
                    }
                    
                    // Store in audio buffer
                    if let Ok(mut buffer) = audio_data.lock() {
                        buffer.extend_from_slice(data);
                    }
                },
                |err| {
                    eprintln!("❌ Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;
        
        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;
        self.stream = Some(stream);
        
        println!("✅ Recording started successfully");
        Ok("Recording started".to_string())
    }
    
    pub fn stop_recording(&mut self) -> Result<Vec<u8>, String> {
        if self.stream.is_none() {
            return Err("Not recording".to_string());
        }
        
        println!("🛑 Stopping recording and generating WAV");
        
        // Stop the stream
        self.stream = None;
        
        // Get the recorded audio data
        let audio_samples = if let Ok(data) = self.audio_data.lock() {
            data.clone()
        } else {
            return Err("Failed to access recorded data".to_string());
        };
        
        if audio_samples.is_empty() {
            return Err("No audio data recorded".to_string());
        }
        
        println!("📊 Processing {} samples ({:.2}s duration)", 
                 audio_samples.len(), 
                 audio_samples.len() as f32 / 16000.0);
        
        // Generate WAV file in memory with Groq specifications
        let mut wav_buffer = Vec::new();
        {
            let spec = WavSpec {
                channels: 1,        // Mono
                sample_rate: 16000, // 16KHz
                bits_per_sample: 16, // 16-bit
                sample_format: hound::SampleFormat::Int,
            };
            
            let mut writer = WavWriter::new(Cursor::new(&mut wav_buffer), spec)
                .map_err(|e| format!("Failed to create WAV writer: {}", e))?;
            
            // Convert f32 samples to i16 and write
            for sample in audio_samples.iter() {
                let sample_i16 = (*sample * i16::MAX as f32) as i16;
                writer.write_sample(sample_i16)
                    .map_err(|e| format!("Failed to write sample: {}", e))?;
            }
            
            writer.finalize()
                .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
        }
        
        // Audio quality analysis
        let duration = audio_samples.len() as f32 / 16000.0;
        let peak_amplitude = audio_samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let rms_level = (audio_samples.iter().map(|s| s * s).sum::<f32>() / audio_samples.len() as f32).sqrt();
        
        println!("🔍 Audio Quality Debug:");
        println!("  Duration: {:.2}s", duration);
        println!("  Sample rate: 16000 Hz");
        println!("  Channels: 1 (mono)");
        println!("  Peak amplitude: {:.3}", peak_amplitude);  
        println!("  RMS level: {:.3}", rms_level);
        println!("  WAV file size: {:.2}KB", wav_buffer.len() as f32 / 1024.0);
        
        // Quality warnings
        if peak_amplitude > 0.95 {
            println!("⚠️ Audio may be clipping (peak: {:.3})", peak_amplitude);
        } else if peak_amplitude < 0.01 {
            println!("⚠️ Audio level very low (peak: {:.3})", peak_amplitude);
        } else {
            println!("✅ Audio levels look good for speech recognition");
        }
        
        println!("✅ Generated {:.2}KB WAV file for Groq", wav_buffer.len() as f32 / 1024.0);
        
        Ok(wav_buffer)
    }
    
    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }
}