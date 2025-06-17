use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig, Sample, FromSample, SizedSample};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct AudioRecorder {
    host: Host,
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
    recording_stream: Option<Stream>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<Mutex<bool>>,
}

impl AudioRecorder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        // Get the supported configuration first to access sample format
        let supported_config = device.default_input_config()?;
        let sample_format = supported_config.sample_format();
        let sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        // Convert to StreamConfig
        let stream_config = StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(AudioRecorder {
            host,
            device,
            config: stream_config,
            sample_format,
            recording_stream: None,
            audio_data: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(Mutex::new(false)),
        })
    }

    pub fn start_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let audio_data = self.audio_data.clone();
        let is_recording = self.is_recording.clone();
        
        {
            let mut recording = is_recording.lock().unwrap();
            *recording = true;
        }

        // Clear previous audio data
        {
            let mut data = audio_data.lock().unwrap();
            data.clear();
        }

        let stream = match self.sample_format {
            SampleFormat::F32 => self.build_input_stream::<f32>(audio_data, is_recording)?,
            SampleFormat::I16 => self.build_input_stream::<i16>(audio_data, is_recording)?,
            SampleFormat::U16 => self.build_input_stream::<u16>(audio_data, is_recording)?,
            _ => return Err("Unsupported sample format".into()),
        };

        stream.play()?;
        self.recording_stream = Some(stream);

        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        {
            let mut recording = self.is_recording.lock().unwrap();
            *recording = false;
        }

        if let Some(stream) = self.recording_stream.take() {
            drop(stream);
        }

        // Wait a moment for the stream to finish
        std::thread::sleep(Duration::from_millis(100));

        let data = self.audio_data.lock().unwrap();
        Ok(data.clone())
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }

    fn build_input_stream<T>(
        &self,
        audio_data: Arc<Mutex<Vec<f32>>>,
        is_recording: Arc<Mutex<bool>>,
    ) -> Result<Stream, Box<dyn std::error::Error>>
    where
        T: Sample + SizedSample + Send + 'static,
        f32: FromSample<T>,
    {
        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let recording = *is_recording.lock().unwrap();
                if recording {
                    let mut audio_buffer = audio_data.lock().unwrap();
                    for &sample in data {
                        audio_buffer.push(sample.to_sample());
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        Ok(stream)
    }

    pub fn save_to_wav(&self, filename: &str, audio_data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        let spec = hound::WavSpec {
            channels: self.config.channels,
            sample_rate: self.config.sample_rate.0,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(filename, spec)?;
        for sample in audio_data {
            writer.write_sample(*sample)?;
        }
        writer.finalize()?;

        Ok(())
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new().expect("Failed to create AudioRecorder")
    }
}