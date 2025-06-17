# CPAL Audio Recording Examples and Documentation

This document provides comprehensive examples and documentation for working with CPAL (Cross-Platform Audio Library) in Rust, specifically focusing on proper StreamConfig creation, SampleFormat handling, and input stream building.

## Current CPAL Version: 0.15.3

## Key Concepts

1. **Host**: Provides access to audio devices on the system
2. **Device**: An audio device with input/output streams
3. **SupportedStreamConfig**: Configuration supported by a device (includes sample format)
4. **StreamConfig**: Configuration used to build streams (doesn't include sample format)
5. **Stream**: An open flow of audio data

## 1. Basic Input Stream Setup

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig, Sample, FromSample, SizedSample};

fn create_input_stream() -> Result<(), Box<dyn std::error::Error>> {
    // Get the default host
    let host = cpal::default_host();
    
    // Get the default input device
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;
    
    // Get the default input configuration (this includes sample format)
    let supported_config = device.default_input_config()?;
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();
    
    // Build stream based on sample format
    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(&device, &config)?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config)?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config)?,
        _ => return Err("Unsupported sample format".into()),
    };
    
    stream.play()?;
    
    // Keep the stream alive
    std::thread::sleep(std::time::Duration::from_secs(5));
    
    Ok(())
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: Sample + SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            // Process audio data here
            for &sample in data {
                let float_sample: f32 = sample.to_sample();
                // Do something with the sample
                println!("Sample: {}", float_sample);
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None, // Timeout
    )?;
    
    Ok(stream)
}
```

## 2. Proper StreamConfig Creation

The key insight is that you need to get the `SupportedStreamConfig` first to access the sample format, then convert it to `StreamConfig`:

```rust
// CORRECT: Get supported config first
let supported_config = device.default_input_config()?;
let sample_format = supported_config.sample_format(); // Available here
let config: StreamConfig = supported_config.into(); // Convert to StreamConfig

// INCORRECT: StreamConfig doesn't have sample_format() method
// let config = StreamConfig { /* ... */ };
// let sample_format = config.sample_format(); // This doesn't exist!
```

## 3. Manual StreamConfig Construction

You can manually construct a StreamConfig, but you need to ensure it's supported:

```rust
use cpal::{StreamConfig, SampleRate, BufferSize};

let config = StreamConfig {
    channels: 1,
    sample_rate: SampleRate(44100),
    buffer_size: BufferSize::Default,
};

// But you still need to determine the sample format separately
// by querying the device's supported configurations
let mut supported_configs = device.supported_input_configs()?;
let supported_config = supported_configs.next()
    .ok_or("No supported input configs")?;
let sample_format = supported_config.sample_format();
```

## 4. Handling Different Sample Formats

```rust
use std::sync::{Arc, Mutex};

struct AudioProcessor {
    buffer: Arc<Mutex<Vec<f32>>>,
}

impl AudioProcessor {
    fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn create_stream(&self, device: &cpal::Device) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
        let supported_config = device.default_input_config()?;
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();
        
        match sample_format {
            SampleFormat::F32 => self.build_input_stream::<f32>(device, &config),
            SampleFormat::I16 => self.build_input_stream::<i16>(device, &config),
            SampleFormat::U16 => self.build_input_stream::<u16>(device, &config),
            _ => Err("Unsupported sample format".into()),
        }
    }
    
    fn build_input_stream<T>(
        &self,
        device: &cpal::Device,
        config: &StreamConfig,
    ) -> Result<cpal::Stream, Box<dyn std::error::Error>>
    where
        T: Sample + SizedSample + Send + 'static,
        f32: FromSample<T>,
    {
        let buffer = self.buffer.clone();
        
        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut buffer_guard = buffer.lock().unwrap();
                for &sample in data {
                    buffer_guard.push(sample.to_sample());
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;
        
        Ok(stream)
    }
}
```

## 5. Working with Specific Devices

```rust
fn list_and_select_device() -> Result<cpal::Device, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    
    // List all input devices
    let devices = host.input_devices()?;
    
    for (index, device) in devices.enumerate() {
        println!("Device {}: {}", index, device.name()?);
        
        // Check supported configurations
        let configs = device.supported_input_configs()?;
        for config in configs {
            println!("  Config: {:?}", config);
        }
    }
    
    // Get default device
    let device = host
        .default_input_device()
        .ok_or("No default input device")?;
    
    Ok(device)
}
```

## 6. Latency and Buffer Management

```rust
use cpal::BufferSize;

fn create_low_latency_stream() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    let supported_config = device.default_input_config()?;
    
    // Create config with specific buffer size
    let config = StreamConfig {
        channels: supported_config.channels(),
        sample_rate: supported_config.sample_rate(),
        buffer_size: BufferSize::Fixed(256), // Small buffer for low latency
    };
    
    let sample_format = supported_config.sample_format();
    
    // Continue with stream creation...
    
    Ok(())
}
```

## 7. Error Handling Best Practices

```rust
#[derive(Debug)]
enum AudioError {
    DeviceNotAvailable,
    ConfigNotSupported,
    StreamBuildFailed(cpal::BuildStreamError),
    PlayError(cpal::PlayStreamError),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AudioError::DeviceNotAvailable => write!(f, "No audio input device available"),
            AudioError::ConfigNotSupported => write!(f, "Audio configuration not supported"),
            AudioError::StreamBuildFailed(e) => write!(f, "Failed to build audio stream: {}", e),
            AudioError::PlayError(e) => write!(f, "Failed to start audio stream: {}", e),
        }
    }
}

impl std::error::Error for AudioError {}

fn robust_audio_setup() -> Result<cpal::Stream, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioError::DeviceNotAvailable)?;
    
    let supported_config = device
        .default_input_config()
        .map_err(|_| AudioError::ConfigNotSupported)?;
    
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();
    
    let stream = match sample_format {
        SampleFormat::F32 => build_typed_stream::<f32>(&device, &config),
        SampleFormat::I16 => build_typed_stream::<i16>(&device, &config),
        SampleFormat::U16 => build_typed_stream::<u16>(&device, &config),
        _ => Err(AudioError::ConfigNotSupported),
    }?;
    
    stream.play().map_err(AudioError::PlayError)?;
    
    Ok(stream)
}

fn build_typed_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
) -> Result<cpal::Stream, AudioError>
where
    T: Sample + SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |_data: &[T], _: &cpal::InputCallbackInfo| {
                // Process audio
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        )
        .map_err(AudioError::StreamBuildFailed)
}
```

## 8. Thread Safety Considerations

CPAL streams are not `Send` or `Sync` by default, which can cause issues in async contexts or when sharing between threads. Here are solutions:

### Solution 1: Keep streams on the main thread
```rust
// Instead of storing the stream in a shared state,
// manage it locally and communicate via channels
use std::sync::mpsc;

enum AudioCommand {
    Start,
    Stop,
    GetData,
}

fn audio_thread() {
    let (tx, rx) = mpsc::channel();
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    // ... create stream locally
    
    for command in rx {
        match command {
            AudioCommand::Start => {
                // stream.play()
            },
            AudioCommand::Stop => {
                // stop stream
            },
            AudioCommand::GetData => {
                // send data back
            },
        }
    }
}
```

### Solution 2: Use a wrapper that manages thread safety
```rust
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};

pub struct ThreadSafeAudioRecorder {
    command_sender: Sender<AudioCommand>,
    data_receiver: Receiver<Vec<f32>>,
}

impl ThreadSafeAudioRecorder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (data_tx, data_rx) = mpsc::channel();
        
        std::thread::spawn(move || {
            // Create CPAL stream in this thread
            let host = cpal::default_host();
            let device = host.default_input_device().unwrap();
            // ... handle commands
        });
        
        Ok(Self {
            command_sender: cmd_tx,
            data_receiver: data_rx,
        })
    }
    
    pub fn start_recording(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.command_sender.send(AudioCommand::Start)?;
        Ok(())
    }
}
```

## 9. Platform-Specific Considerations

### Linux (ALSA/JACK)
```rust
#[cfg(target_os = "linux")]
fn linux_specific_setup() {
    // ALSA is the default on Linux
    let host = cpal::default_host();
    
    // For JACK support, you can use:
    // let host = cpal::host_from_id(cpal::HostId::Jack).unwrap();
}
```

### Windows (WASAPI/ASIO)
```rust
#[cfg(target_os = "windows")]
fn windows_specific_setup() {
    // WASAPI is the default on Windows
    let host = cpal::default_host();
    
    // For ASIO support (requires additional setup):
    // let host = cpal::host_from_id(cpal::HostId::Asio).unwrap();
}
```

### macOS (CoreAudio)
```rust
#[cfg(target_os = "macos")]
fn macos_specific_setup() {
    // CoreAudio is the default on macOS
    let host = cpal::default_host();
    
    // Note: macOS typically uses f32 samples
}
```

## Common Issues and Solutions

### Issue 1: "The trait bound `T: SizedSample` is not satisfied"
**Solution**: Add `SizedSample` to your trait bounds:
```rust
where
    T: Sample + SizedSample + Send + 'static,
    f32: FromSample<T>,
```

### Issue 2: "StreamConfig doesn't have sample_format() method"
**Solution**: Get sample format from SupportedStreamConfig before converting:
```rust
let supported_config = device.default_input_config()?;
let sample_format = supported_config.sample_format(); // Get it here
let config: StreamConfig = supported_config.into(); // Then convert
```

### Issue 3: "Stream is not Send/Sync"
**Solution**: Keep streams on a single thread and use message passing:
```rust
// Don't store Stream in shared state
// Instead, manage streams in dedicated threads
```

### Issue 4: "No default input device"
**Solution**: Check available devices and handle the case gracefully:
```rust
let device = host
    .default_input_device()
    .or_else(|| host.input_devices().ok()?.next())
    .ok_or("No input devices available")?;
```

## Best Practices

1. **Always query device capabilities before creating streams**
2. **Handle all supported sample formats (F32, I16, U16)**
3. **Use proper error handling with custom error types**
4. **Keep streams on dedicated threads for async applications**
5. **Test on multiple platforms if targeting cross-platform support**
6. **Use appropriate buffer sizes for your latency requirements**
7. **Properly handle device disconnection and reconnection**

## Example: Complete Working Audio Recorder

Here's a complete, working example that demonstrates all the concepts:

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig, Sample, FromSample, SizedSample};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct AudioRecorder {
    command_sender: Sender<RecorderCommand>,
    data_receiver: Arc<Mutex<Receiver<Vec<f32>>>>,
}

enum RecorderCommand {
    Start,
    Stop,
    IsRecording(Sender<bool>),
}

impl AudioRecorder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (data_tx, data_rx) = mpsc::channel();
        
        thread::spawn(move || {
            let mut recorder = AudioRecorderThread::new(data_tx).unwrap();
            
            for command in cmd_rx {
                match command {
                    RecorderCommand::Start => {
                        let _ = recorder.start();
                    },
                    RecorderCommand::Stop => {
                        let _ = recorder.stop();
                    },
                    RecorderCommand::IsRecording(response_tx) => {
                        let _ = response_tx.send(recorder.is_recording());
                    },
                }
            }
        });
        
        Ok(Self {
            command_sender: cmd_tx,
            data_receiver: Arc::new(Mutex::new(data_rx)),
        })
    }
    
    pub fn start_recording(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.command_sender.send(RecorderCommand::Start)?;
        Ok(())
    }
    
    pub fn stop_recording(&self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        self.command_sender.send(RecorderCommand::Stop)?;
        let receiver = self.data_receiver.lock().unwrap();
        let data = receiver.recv()?;
        Ok(data)
    }
}

struct AudioRecorderThread {
    host: cpal::Host,
    device: cpal::Device,
    config: StreamConfig,
    sample_format: SampleFormat,
    stream: Option<cpal::Stream>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<Mutex<bool>>,
    data_sender: Sender<Vec<f32>>,
}

impl AudioRecorderThread {
    fn new(data_sender: Sender<Vec<f32>>) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;
        
        let supported_config = device.default_input_config()?;
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();
        
        Ok(Self {
            host,
            device,
            config,
            sample_format,
            stream: None,
            audio_data: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(Mutex::new(false)),
            data_sender,
        })
    }
    
    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let audio_data = self.audio_data.clone();
        let is_recording = self.is_recording.clone();
        
        {
            let mut recording = is_recording.lock().unwrap();
            *recording = true;
        }
        
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
        self.stream = Some(stream);
        
        Ok(())
    }
    
    fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut recording = self.is_recording.lock().unwrap();
            *recording = false;
        }
        
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        
        thread::sleep(std::time::Duration::from_millis(100));
        
        let data = self.audio_data.lock().unwrap().clone();
        self.data_sender.send(data)?;
        
        Ok(())
    }
    
    fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }
    
    fn build_input_stream<T>(
        &self,
        audio_data: Arc<Mutex<Vec<f32>>>,
        is_recording: Arc<Mutex<bool>>,
    ) -> Result<cpal::Stream, Box<dyn std::error::Error>>
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
}
```

This comprehensive guide should help you understand the proper CPAL API usage and resolve the compilation errors in your audio recording application.