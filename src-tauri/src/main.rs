// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod api;
mod system;

use tauri::{Manager, State};
use std::sync::{Arc, Mutex, mpsc};
use api::GroqClient;
use system::{TextInjector, WindowInfo};

// Audio control messages for thread communication
#[derive(Debug)]
pub enum AudioCommand {
    StartRecording,
    StopRecording,
    IsRecording,
    Exit,
}

#[derive(Debug)]
pub enum AudioResponse {
    RecordingStarted,
    RecordingStopped(Vec<f32>),
    IsRecording(bool),
    Error(String),
}

// Simplified application state without storing AudioRecorder directly
pub struct AppState {
    pub groq_client: Arc<Mutex<Option<GroqClient>>>,
    pub text_injector: Arc<Mutex<TextInjector>>,
    pub audio_tx: Arc<Mutex<mpsc::Sender<AudioCommand>>>,
    pub audio_rx: Arc<Mutex<mpsc::Receiver<AudioResponse>>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello {}, you've been greeted from Rust!", name)
}

#[tauri::command]
async fn start_recording(state: State<'_, AppState>) -> Result<String, String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    let rx = state.audio_rx.lock().map_err(|e| e.to_string())?;
    
    tx.send(AudioCommand::StartRecording).map_err(|e| e.to_string())?;
    
    match rx.recv().map_err(|e| e.to_string())? {
        AudioResponse::RecordingStarted => Ok("Recording started".to_string()),
        AudioResponse::Error(err) => Err(err),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn stop_recording(state: State<'_, AppState>) -> Result<Vec<f32>, String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    let rx = state.audio_rx.lock().map_err(|e| e.to_string())?;
    
    tx.send(AudioCommand::StopRecording).map_err(|e| e.to_string())?;
    
    match rx.recv().map_err(|e| e.to_string())? {
        AudioResponse::RecordingStopped(data) => Ok(data),
        AudioResponse::Error(err) => Err(err),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn is_recording(state: State<'_, AppState>) -> Result<bool, String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    let rx = state.audio_rx.lock().map_err(|e| e.to_string())?;
    
    tx.send(AudioCommand::IsRecording).map_err(|e| e.to_string())?;
    
    match rx.recv().map_err(|e| e.to_string())? {
        AudioResponse::IsRecording(recording) => Ok(recording),
        AudioResponse::Error(err) => Err(err),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn set_groq_api_key(state: State<'_, AppState>, api_key: String) -> Result<String, String> {
    let groq_client = state.groq_client.clone();
    let mut client_guard = groq_client.lock().map_err(|e| e.to_string())?;
    
    *client_guard = Some(GroqClient::new(api_key));
    Ok("Groq API key set successfully".to_string())
}

#[tauri::command]
async fn transcribe_audio(
    state: State<'_, AppState>,
    audio_data: Vec<f32>,
    sample_rate: u32,
    channels: u16,
) -> Result<String, String> {
    // Clone the client outside the guard to avoid holding the lock across await
    let client = {
        let groq_client = state.groq_client.clone();
        let client_guard = groq_client.lock().map_err(|e| e.to_string())?;
        client_guard.as_ref().ok_or("Groq API key not set")?.clone()
    };
    
    // Convert f32 audio data to WAV bytes using hound
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| e.to_string())?;
        
        for sample in &audio_data {
            writer.write_sample(*sample).map_err(|e| e.to_string())?;
        }
        
        writer.finalize().map_err(|e| e.to_string())?;
    }
    let wav_bytes = cursor.into_inner();
    
    // Transcribe using Groq
    let transcription = client
        .transcribe_audio_bytes(&wav_bytes, "audio.wav", None, None)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(transcription)
}

#[tauri::command]
async fn inject_text(state: State<'_, AppState>, text: String) -> Result<String, String> {
    let text_injector = state.text_injector.clone();
    let injector_guard = text_injector.lock().map_err(|e| e.to_string())?;
    
    injector_guard.inject_text(&text).map_err(|e| e.to_string())?;
    Ok("Text injected successfully".to_string())
}

#[tauri::command]
async fn get_active_window_info(state: State<'_, AppState>) -> Result<WindowInfo, String> {
    let text_injector = state.text_injector.clone();
    let injector_guard = text_injector.lock().map_err(|e| e.to_string())?;
    
    let window_info = injector_guard.get_active_window_info().map_err(|e| e.to_string())?;
    Ok(window_info)
}

fn create_audio_thread() -> (mpsc::Sender<AudioCommand>, mpsc::Receiver<AudioResponse>) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (resp_tx, resp_rx) = mpsc::channel();
    
    std::thread::spawn(move || {
        use audio::AudioRecorder;
        let mut recorder: Option<AudioRecorder> = None;
        
        loop {
            match cmd_rx.recv() {
                Ok(AudioCommand::StartRecording) => {
                    if recorder.is_some() {
                        let _ = resp_tx.send(AudioResponse::Error("Already recording".to_string()));
                        continue;
                    }
                    
                    match AudioRecorder::new() {
                        Ok(mut new_recorder) => {
                            match new_recorder.start_recording() {
                                Ok(_) => {
                                    recorder = Some(new_recorder);
                                    let _ = resp_tx.send(AudioResponse::RecordingStarted);
                                }
                                Err(e) => {
                                    let _ = resp_tx.send(AudioResponse::Error(e.to_string()));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = resp_tx.send(AudioResponse::Error(e.to_string()));
                        }
                    }
                }
                Ok(AudioCommand::StopRecording) => {
                    if let Some(mut rec) = recorder.take() {
                        match rec.stop_recording() {
                            Ok(data) => {
                                let _ = resp_tx.send(AudioResponse::RecordingStopped(data));
                            }
                            Err(e) => {
                                let _ = resp_tx.send(AudioResponse::Error(e.to_string()));
                            }
                        }
                    } else {
                        let _ = resp_tx.send(AudioResponse::RecordingStopped(Vec::new()));
                    }
                }
                Ok(AudioCommand::IsRecording) => {
                    let is_recording = recorder.as_ref().map_or(false, |r| r.is_recording());
                    let _ = resp_tx.send(AudioResponse::IsRecording(is_recording));
                }
                Ok(AudioCommand::Exit) => break,
                Err(_) => break,
            }
        }
    });
    
    (cmd_tx, resp_rx)
}

// Apply Linux graphics fix for blank screen issue
fn apply_graphics_fix() {
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    apply_graphics_fix();
    let groq_client = Arc::new(Mutex::new(None));
    
    let text_injector = Arc::new(Mutex::new(
        TextInjector::new().expect("Failed to initialize text injector")
    ));
    
    let (audio_tx, audio_rx) = create_audio_thread();

    let app_state = AppState {
        groq_client,
        text_injector,
        audio_tx: Arc::new(Mutex::new(audio_tx)),
        audio_rx: Arc::new(Mutex::new(audio_rx)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            start_recording,
            stop_recording,
            is_recording,
            set_groq_api_key,
            transcribe_audio,
            inject_text,
            get_active_window_info
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}