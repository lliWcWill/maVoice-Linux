//! Audio capture → in-memory WAV encoder (16-bit mono) for Groq STT.

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, SampleFormat, Stream, StreamConfig, SampleRate,
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
    sample_sender: Sender<f32>,
    _sample_receiver: Receiver<f32>,
}

impl GroqRecorder {
    // ───────────────────────────────────────────────────────────────────
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

        let audio_data = Arc::new(Mutex::new(Vec::<f32>::new()));
        let (tx, rx) = unbounded();

        Ok(Self {
            host,
            input_device,
            config,
            stream: None,
            audio_data,
            sample_sender: tx,
            _sample_receiver: rx,
        })
    }

    // ───────────────────────────────────────────────────────────────────
    pub fn start_recording(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Err("Already recording".into());
        }

        println!("🚀 Starting recording …");
        self.audio_data.lock().unwrap().clear();

        let audio_buf = self.audio_data.clone();
        let tx = self.sample_sender.clone();

        let sample_format = self
            .input_device
            .default_input_config()
            .map_err(|e| e.to_string())?
            .sample_format();

        let err_fn = |err| eprintln!("❌ Stream error: {err}");

        self.stream = Some(match sample_format {
            SampleFormat::F32 => {
                self.input_device
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
                self.input_device
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
                self.input_device
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

    // ───────────────────────────────────────────────────────────────────
    pub fn stop_recording(&mut self) -> Result<Vec<u8>, String> {
        if self.stream.is_none() {
            return Err("Not recording".into());
        }
        println!("🛑 Stopping recording and generating WAV");
        self.stream.take(); // drop = stop

        let samples = self.audio_data.lock().unwrap().clone();
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
}
