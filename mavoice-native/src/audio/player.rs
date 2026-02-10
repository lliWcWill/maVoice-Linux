use cpal::traits::*;
use cpal::{Device, SampleRate, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Audio player for Gemini Live PCM output (24kHz mono s16le).
///
/// Uses a shared ring buffer: the main thread enqueues decoded PCM data,
/// and the cpal output callback drains it to the speakers.
pub struct AudioPlayer {
    _stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    /// Recent output samples for visualization (last 1024)
    recent_output: Arc<Mutex<Vec<f32>>>,
    playing: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, String> {
        log::info!("Initializing audio player for Gemini output");

        let host = cpal::default_host();
        let output_device = host
            .default_output_device()
            .ok_or("No output device available")?;
        log::info!(
            "Output device: {}",
            output_device.name().unwrap_or_default()
        );

        // Try 24kHz mono first (Gemini's native output rate), fallback to device default
        let (config, needs_resample) = Self::pick_output_config(&output_device)?;

        log::info!(
            "Output config: {} Hz, {} channel(s), resample={}",
            config.sample_rate.0,
            config.channels,
            needs_resample
        );

        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(48000)));
        let recent_output = Arc::new(Mutex::new(Vec::<f32>::with_capacity(2048)));
        let playing = Arc::new(AtomicBool::new(false));

        let buf_clone = buffer.clone();
        let recent_clone = recent_output.clone();
        let playing_clone = playing.clone();
        let out_channels = config.channels as usize;

        let stream = output_device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    let mut buf = buf_clone.lock().unwrap();
                    let mono_samples_needed = data.len() / out_channels;

                    if buf.is_empty() {
                        // Silence
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                        playing_clone.store(false, Ordering::Relaxed);
                        return;
                    }

                    playing_clone.store(true, Ordering::Relaxed);

                    let available = buf.len().min(mono_samples_needed);
                    let drained: Vec<f32> = buf.drain(..available).collect();

                    // Write to output (duplicate mono to all channels)
                    let mut src_idx = 0;
                    for frame in data.chunks_mut(out_channels) {
                        let sample = if src_idx < drained.len() {
                            drained[src_idx]
                        } else {
                            0.0
                        };
                        for ch in frame.iter_mut() {
                            *ch = sample;
                        }
                        src_idx += 1;
                    }

                    // Track recent output for visualization
                    let mut recent = recent_clone.lock().unwrap();
                    recent.extend_from_slice(&drained);
                    if recent.len() > 2048 {
                        let excess = recent.len() - 2048;
                        recent.drain(..excess);
                    }
                },
                |err| log::error!("Output stream error: {err}"),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        log::info!("Audio player started");

        Ok(Self {
            _stream: stream,
            buffer,
            recent_output,
            playing,
        })
    }

    /// Pick output config: prefer 24kHz mono, fall back to device default.
    fn pick_output_config(device: &Device) -> Result<(StreamConfig, bool), String> {
        let supports_24k = device
            .supported_output_configs()
            .map(|mut it| {
                it.any(|c| {
                    c.min_sample_rate() <= SampleRate(24_000)
                        && c.max_sample_rate() >= SampleRate(24_000)
                })
            })
            .unwrap_or(false);

        if supports_24k {
            Ok((
                StreamConfig {
                    channels: 1,
                    sample_rate: SampleRate(24_000),
                    buffer_size: cpal::BufferSize::Default,
                },
                false,
            ))
        } else {
            // Use device default — we'll need to resample
            let def = device.default_output_config().map_err(|e| e.to_string())?;
            log::warn!(
                "24kHz not supported, using device default: {} Hz, {} ch",
                def.sample_rate().0,
                def.channels()
            );
            let config: StreamConfig = def.into();
            Ok((config, true))
        }
    }

    /// Enqueue raw PCM data from Gemini (24kHz mono s16le bytes).
    /// Thread-safe — can be called from any thread.
    pub fn enqueue(&self, pcm_24khz_s16le: &[u8]) {
        // Convert s16le bytes → f32 samples
        let samples: Vec<f32> = pcm_24khz_s16le
            .chunks_exact(2)
            .map(|chunk| {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                sample as f32 / i16::MAX as f32
            })
            .collect();

        let mut buf = self.buffer.lock().unwrap();
        buf.extend_from_slice(&samples);
    }

    /// Flush the playback buffer (for barge-in interruption).
    pub fn clear(&self) {
        self.buffer.lock().unwrap().clear();
        self.recent_output.lock().unwrap().clear();
    }

    /// Whether audio is currently being played.
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    /// Get 4-band audio levels from recent output for visualization.
    /// Same algorithm as GroqRecorder::get_audio_levels().
    pub fn get_output_levels(&self) -> [f32; 4] {
        let recent = self.recent_output.lock().unwrap();
        if recent.is_empty() {
            return [0.0; 4];
        }

        // Use last 1024 samples
        let samples = if recent.len() > 1024 {
            &recent[recent.len() - 1024..]
        } else {
            &recent[..]
        };

        let rms: f32 = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();

        let chunk_size = samples.len() / 4;
        let mut levels = [0.0f32; 4];

        for i in 0..4 {
            let start = i * chunk_size;
            let end = if i == 3 {
                samples.len()
            } else {
                (i + 1) * chunk_size
            };

            if start < samples.len() {
                let chunk = &samples[start..end];
                let chunk_rms: f32 =
                    (chunk.iter().map(|&x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
                levels[i] = (chunk_rms * 10.0).min(1.0);
            }
        }

        let boost = rms * 7.0;
        for level in &mut levels {
            *level = (*level + boost).min(1.0);
        }

        levels
    }
}
