use cpal::{traits::*, Device, StreamConfig, SampleRate, Stream, SampleFormat};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::io::Cursor;
use crossbeam_channel::{unbounded, Receiver, Sender};

pub struct GroqRecorder {
    device: Device,
    config: StreamConfig,
    stream: Option<Stream>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    sample_sender: Sender<f32>,
    _sample_receiver: Receiver<f32>,
}

impl GroqRecorder {
    pub fn new() -> Result<Self, String> {
        println!("🎤 Initializing Groq-compatible audio recorder");

        let host = cpal::default_host();
        println!("🔧 Audio host: {}", host.id().name());

        let input_device = host
            .default_input_device()
            .ok_or("No input device available")?;
        println!("🎧 Using device: {}", input_device.name().unwrap_or_default());

        // prefer 16 kHz mono; fallback to device default
        let mut config = StreamConfig {
            channels: 1,
            sample_rate: SampleRate(16_000),
            buffer_size: cpal::BufferSize::Default,
        };

        let supports_16k = input_device
            .supported_input_configs()
            .map(|mut it| {
                it.any(|c| {
                    c.channels() == 1
                        && c.min_sample_rate() <= SampleRate(16_000)
                        && c.max_sample_rate() >= SampleRate(16_000)
                })
            })
            .unwrap_or(false);

        if !supports_16k {
            println!("⚠️ 16 kHz not supported – using device default rate");
            let def_cfg = input_device
                .default_input_config()
                .map_err(|e| e.to_string())?;
            config = def_cfg.into();
            config.channels = 1;
        }

        println!(
            "📐 Input config → {} Hz, {} channel(s)",
            config.sample_rate.0, config.channels
        );

        let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let (tx, rx) = unbounded();

        Ok(Self {
            device: input_device,
            config,
            stream: None,
            audio_buffer,
            sample_sender: tx,
            _sample_receiver: rx,
        })
    }
    
    pub fn start_recording(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Err("Already recording".into());
        }

        println!("🚀 Starting recording …");
        self.audio_buffer.lock().unwrap().clear();

        let audio_buf = self.audio_buffer.clone();
        let tx = self.sample_sender.clone();

        let sample_format = self
            .device
            .default_input_config()
            .map_err(|e| e.to_string())?
            .sample_format();

        let err_fn = |err| eprintln!("❌ Stream error: {err}");

        self.stream = Some(match sample_format {
            SampleFormat::F32 => {
                self.device
                    .build_input_stream(
                        &self.config,
                        move |data: &[f32], _| {
                            for &s in data {
                                let _ = tx.send(s);
                            }
                            audio_buf.lock().unwrap().extend_from_slice(data);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| e.to_string())?
            }
            SampleFormat::I16 => {
                self.device
                    .build_input_stream(
                        &self.config,
                        move |data: &[i16], _| {
                            for &s in data {
                                let f = s as f32 / i16::MAX as f32;
                                let _ = tx.send(f);
                                audio_buf.lock().unwrap().push(f);
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| e.to_string())?
            }
            SampleFormat::U16 => {
                self.device
                    .build_input_stream(
                        &self.config,
                        move |data: &[u16], _| {
                            for &s in data {
                                let f = (s as f32 / u16::MAX as f32) * 2.0 - 1.0;
                                let _ = tx.send(f);
                                audio_buf.lock().unwrap().push(f);
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| e.to_string())?
            }
            _ => return Err("Unsupported sample format".into()),
        });

        self.stream
            .as_ref()
            .unwrap()
            .play()
            .map_err(|e| e.to_string())?;

        println!("✅ Recording started successfully");
        Ok(())
    }
    
    pub fn stop_recording(&mut self) -> Result<Vec<u8>, String> {
        if self.stream.is_none() {
            return Err("Not recording".into());
        }
        println!("🛑 Stopping recording and generating WAV");
        self.stream.take(); // drop = stop

        let samples = self.audio_buffer.lock().unwrap().clone();
        if samples.is_empty() {
            return Err("No audio captured".into());
        }

        let mut wav_bytes = Vec::<u8>::new();
        {
            let spec = WavSpec {
                channels: 1,
                sample_rate: self.config.sample_rate.0,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer =
                WavWriter::new(Cursor::new(&mut wav_bytes), spec)
                    .map_err(|e| e.to_string())?;

            for &s in &samples {
                let s16 = (s * i16::MAX as f32)
                    .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                writer.write_sample(s16).map_err(|e| e.to_string())?;
            }
            writer.finalize().unwrap();
        }

        println!(
            "✅ Generated {:.1} KB WAV ({} samples @ {} Hz)",
            wav_bytes.len() as f32 / 1024.0,
            samples.len(),
            self.config.sample_rate.0
        );
        Ok(wav_bytes)
    }
    
    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }

    // NEW: Get real-time audio levels for visualization
    pub fn get_audio_levels(&self) -> [f32; 4] {
        if !self.is_recording() {
            return [0.0, 0.0, 0.0, 0.0]; // Silent when not recording
        }

        let buffer = self.audio_buffer.lock().unwrap();
        let samples = &*buffer;
        
        // Use last 1024 samples (about 64ms at 16kHz) for real-time response
        let recent_samples: Vec<f32> = if samples.len() > 1024 {
            samples[samples.len() - 1024..].to_vec()
        } else {
            samples.clone()
        };

        if recent_samples.is_empty() {
            return [0.0, 0.0, 0.0, 0.0];
        }

        // Calculate RMS (Root Mean Square) for overall volume
        let rms: f32 = (recent_samples.iter()
            .map(|&x| x * x)
            .sum::<f32>() / recent_samples.len() as f32)
            .sqrt();

        // Simulate 4 frequency bands by analyzing different parts of the signal
        // This is a simplified approach - for real frequency analysis you'd need FFT
        let chunk_size = recent_samples.len() / 4;
        let mut levels = [0.0f32; 4];
        
        for i in 0..4 {
            let start = i * chunk_size;
            let end = if i == 3 { recent_samples.len() } else { (i + 1) * chunk_size };
            
            if start < recent_samples.len() {
                let chunk = &recent_samples[start..end];
                let chunk_rms: f32 = (chunk.iter()
                    .map(|&x| x * x)
                    .sum::<f32>() / chunk.len() as f32)
                    .sqrt();
                
                // Normalize and amplify for better visualization (0.0 to 1.0)
                levels[i] = (chunk_rms * 10.0).min(1.0);
            }
        }

        // Add some variation based on overall RMS to make it more responsive
        let boost = rms * 5.0;
        for level in &mut levels {
            *level = (*level + boost).min(1.0);
        }

        levels
    }
}