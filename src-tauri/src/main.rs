// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod audio;
mod system;
mod webm;

use tauri::{State, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
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
async fn get_audio_levels(state: State<'_, AppState>) -> Result<[f32; 4], String> {
    let recorder = state.groq_recorder.lock()
        .map_err(|e| format!("Recorder lock error: {}", e))?;
    
    Ok(recorder.get_audio_levels())
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
    audio_data: Vec<u8>,
) -> Result<String, String> {
    println!("📤 Sending {:.2}KB WAV to Groq", audio_data.len() as f32 / 1024.0);
    
    // Get Groq client
    let client = {
        let groq_client = state.groq_client.clone();
        let client_guard = groq_client.lock().map_err(|e| e.to_string())?;
        client_guard.as_ref().ok_or("Groq API key not set")?.clone()
    };
    
    match client.transcribe_audio_bytes(&audio_data, "recording.wav", Some("whisper-large-v3-turbo"), Some("en")).await {
        Ok(transcription) => {
            println!("🎯 === TRANSCRIPTION RESULT ===");
            println!("📝 Text: {}", transcription);
            println!("📊 Length: {} characters", transcription.len());
            println!("🎯 ===========================");
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

#[tauri::command]
async fn copy_to_clipboard(text: String) -> Result<String, String> {
    use std::process::Command;
    
    println!("📋 DEV TIER: Native clipboard copy for {} chars", text.len());
    
    // Try multiple clipboard methods for maximum compatibility
    
    // Method 1: wl-copy for Wayland
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        match Command::new("wl-copy")
            .arg(&text)
            .output()
        {
            Ok(output) if output.status.success() => {
                println!("✅ Clipboard: wl-copy success");
                return Ok("Copied via wl-copy".to_string());
            }
            Ok(_) => println!("❌ wl-copy failed"),
            Err(_) => println!("❌ wl-copy not available"),
        }
    }
    
    // Method 2: xclip for X11
    let mut cmd = Command::new("xclip");
    cmd.args(&["-selection", "clipboard"]);
    cmd.stdin(std::process::Stdio::piped());
    
    match cmd.spawn() {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            match child.wait() {
                Ok(status) if status.success() => {
                    println!("✅ Clipboard: xclip success");
                    return Ok("Copied via xclip".to_string());
                }
                _ => println!("❌ xclip process failed"),
            }
        }
        Err(_) => println!("❌ xclip not available"),
    }
    
    // Method 3: xsel fallback
    match Command::new("xsel")
        .args(&["--clipboard", "--input"])
        .arg(&text)
        .output()
    {
        Ok(output) if output.status.success() => {
            println!("✅ Clipboard: xsel success");
            return Ok("Copied via xsel".to_string());
        }
        Ok(_) => println!("❌ xsel failed"),
        Err(_) => println!("❌ xsel not available"),
    }
    
    Err("All native clipboard methods failed - install wl-clipboard or xclip".to_string())
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            get_audio_levels,
            set_groq_api_key,
            transcribe_audio,
            inject_text,
            get_active_window_info,
            copy_to_clipboard
        ])
        .setup(|app| {
            // NO devtools auto-open - clean desktop app experience
            println!("🚀 AquaVoice starting up...");
            
            // Register global Ctrl+Shift+, hotkey for voice recording
            use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
            
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Comma);
            
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, _shortcut, event| {
                        println!("🎤 Global Ctrl+Shift+, pressed! Event: {:?}", event);
                        
                        // Trigger on any event (pressed/released doesn't matter)
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            
                            // Trigger recording start via frontend
                            let _ = window.eval("window.startGlobalRecording && window.startGlobalRecording()");
                        }
                    })
                    .build(),
            )?;
            
            // Register the shortcut
            app.global_shortcut().register(shortcut)?;
            println!("✅ Global Ctrl+Shift+, hotkey registered!");
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}