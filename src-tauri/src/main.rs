// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod audio;
mod system;
mod webm;

use tauri::State;
use std::sync::{Arc, Mutex};
use api::GroqClient;
use audio::GroqRecorder;
use system::{TextInjector, WindowInfo};
use webm::WebMProcessor;

// Simplified application state
pub struct AppState {
    pub groq_client: Arc<Mutex<Option<GroqClient>>>,
    pub groq_recorder: Arc<Mutex<GroqRecorder>>,
    pub text_injector: Arc<Mutex<TextInjector>>,
    pub webm_processor: Arc<Mutex<WebMProcessor>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello {}, you've been greeted from Rust!", name)
}

// Groq-compatible native audio recording commands

#[tauri::command]
async fn start_groq_recording(state: State<'_, AppState>) -> Result<String, String> {
    println!("🚀 Starting Groq-compatible recording");
    
    let mut recorder = state.groq_recorder.lock()
        .map_err(|e| format!("Recorder lock error: {}", e))?;
    
    recorder.start_recording()?;
    Ok("Recording started with Groq-optimized settings (16KHz mono)".to_string())
}

#[tauri::command]
async fn stop_groq_recording(state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    println!("📤 Stopping recording and preparing for Groq API");
    
    let mut recorder = state.groq_recorder.lock()
        .map_err(|e| format!("Recorder lock error: {}", e))?;
    
    let wav_data = recorder.stop_recording()?;
    
    // Validate minimum length for Groq (0.01 seconds minimum)
    if wav_data.len() < 1000 { // Rough estimate for very short audio
        return Err("Recording too short (minimum 0.01 seconds)".to_string());
    }
    
    println!("✅ Generated {:.2}KB WAV file for Groq", wav_data.len() as f32 / 1024.0);
    Ok(wav_data)
}

#[tauri::command]
async fn is_recording(state: State<'_, AppState>) -> Result<bool, String> {
    let recorder = state.groq_recorder.lock()
        .map_err(|e| format!("Recorder lock error: {}", e))?;
    
    Ok(recorder.is_recording())
}

#[tauri::command]
async fn set_groq_api_key(state: State<'_, AppState>, api_key: String) -> Result<String, String> {
    let groq_client = state.groq_client.clone();
    let mut client_guard = groq_client.lock().map_err(|e| e.to_string())?;
    
    *client_guard = Some(GroqClient::new(api_key));
    Ok("Groq API key set successfully".to_string())
}

#[tauri::command]
async fn process_webm_audio(
    state: State<'_, AppState>,
    webm_data: Vec<u8>,
) -> Result<String, String> {
    println!("🔥 process_webm_audio called with {} bytes", webm_data.len());
    
    // Get Groq client first
    let client = {
        let groq_client = state.groq_client.clone();
        let client_guard = groq_client.lock().map_err(|e| e.to_string())?;
        client_guard.as_ref().ok_or("Groq API key not set")?.clone()
    };
    
    // Send WebM directly to Groq (it supports WebM format natively)
    println!("🔥 Sending WebM directly to Groq API (bypassing local processing)");
    
    match client.transcribe_audio_bytes(&webm_data, "recording.webm", Some("whisper-large-v3-turbo"), Some("en")).await {
        Ok(transcription) => {
            println!("✅ Groq transcription successful: '{}'", transcription);
            Ok(transcription)
        }
        Err(e) => {
            println!("❌ Groq transcription failed: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn inject_text(state: State<'_, AppState>, text: String) -> Result<String, String> {
    println!("🔥 inject_text called with text: '{}'", text);
    
    let text_injector = state.text_injector.clone();
    let injector_guard = text_injector.lock().map_err(|e| e.to_string())?;
    
    println!("🔥 About to inject text using backend: {:?}", injector_guard.backend());
    
    match injector_guard.inject_text(&text) {
        Ok(_) => {
            println!("✅ Text injection successful");
            Ok("Text injected successfully".to_string())
        }
        Err(e) => {
            println!("❌ Text injection failed: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn get_active_window_info(state: State<'_, AppState>) -> Result<WindowInfo, String> {
    let text_injector = state.text_injector.clone();
    let injector_guard = text_injector.lock().map_err(|e| e.to_string())?;
    
    let window_info = injector_guard.get_active_window_info().map_err(|e| e.to_string())?;
    Ok(window_info)
}

// Audio thread no longer needed - using browser MediaRecorder

// Apply Linux graphics fix for blank screen issue
fn apply_graphics_fix() {
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    apply_graphics_fix();
    let groq_client = Arc::new(Mutex::new(None));
    
    let groq_recorder = Arc::new(Mutex::new(
        GroqRecorder::new().expect("Failed to initialize Groq recorder")
    ));
    
    let text_injector = Arc::new(Mutex::new(
        TextInjector::new().expect("Failed to initialize text injector")
    ));
    
    let webm_processor = Arc::new(Mutex::new(
        WebMProcessor::new()
    ));

    let app_state = AppState {
        groq_client,
        groq_recorder,
        text_injector,
        webm_processor,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_log::Builder::new()
            .target(tauri_plugin_log::Target::new(
                tauri_plugin_log::TargetKind::Stdout,
            ))
            .level(log::LevelFilter::Debug)
            .build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            start_groq_recording,
            stop_groq_recording,
            is_recording,
            set_groq_api_key,
            process_webm_audio,
            inject_text,
            get_active_window_info
        ])
        .setup(|_app| {
            // NO devtools auto-open - clean desktop app experience
            println!("🚀 AquaVoice starting up...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}