use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, DrawingArea, Button};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use cpal::{Device, Stream, StreamConfig, SampleRate};
use crossbeam_channel::{unbounded, Receiver, Sender};

mod audio_recorder;
mod text_injector;
mod groq_client;

use audio_recorder::GroqRecorder;
use text_injector::TextInjector;
use groq_client::{GroqClient, get_groq_api_key};

const APP_ID: &str = "com.aquavoice.gtk";

struct FloatingMic {
    window: ApplicationWindow,
    button: Button,
    recorder: Arc<Mutex<GroqRecorder>>,
    text_injector: Arc<Mutex<TextInjector>>,
    groq_client: Arc<GroqClient>,
    is_recording: Rc<RefCell<bool>>,
}

impl FloatingMic {
    fn new(app: &Application) -> Self {
        // Create a small, circular window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("AquaVoice")
            .default_width(60)
            .default_height(60)
            .decorated(false)           // No window decorations
            .resizable(false)
            .build();

        // Window will be small and circular - floating behavior handled by window manager
        
        // Create circular microphone button
        let button = Button::builder()
            .width_request(56)
            .height_request(56)
            .build();
        
        // Style the button to be circular
        button.set_css_classes(&["circular-button"]);
        
        // Make window draggable - GTK4 way using GestureDrag
        let drag_gesture = gtk::GestureDrag::new();
        drag_gesture.set_button(3); // Right-click for dragging
        
        let window_for_drag = window.clone();
        drag_gesture.connect_drag_begin(move |gesture, start_x, start_y| {
            println!("🎯 Right-click drag detected, starting window move");
            
            // Get the toplevel surface for move operations
            let surface = window_for_drag.surface();
            if let Some(toplevel) = surface.downcast_ref::<gtk::gdk::Toplevel>() {
                if let Some(device) = gesture.device() {
                    let button = gesture.current_button();
                    let timestamp = gesture.current_event_time();
                    
                    // Start window move operation using toplevel
                    toplevel.begin_move(
                        device,
                        button as i32,
                        start_x,
                        start_y,
                        timestamp,
                    );
                }
            }
        });
        
        button.add_controller(drag_gesture);
        
        button.set_tooltip_text(Some("Left-click: Record | Right-click+drag: Move"));
        
        // Add button to window
        window.set_child(Some(&button));
        
        // Initialize components
        let recorder = Arc::new(Mutex::new(GroqRecorder::new().unwrap()));
        let text_injector = Arc::new(Mutex::new(TextInjector::new()));
        let groq_client = Arc::new(GroqClient::new(
            get_groq_api_key().expect("GROQ_API_KEY must be set")
        ));
        let is_recording = Rc::new(RefCell::new(false));
        
        // Set up button click handler
        let recorder_clone = recorder.clone();
        let text_injector_clone = text_injector.clone();
        let groq_client_clone = groq_client.clone();
        let is_recording_clone = is_recording.clone();
        let button_clone = button.clone();
        
        button.connect_clicked(move |_| {
            let mut recording = is_recording_clone.borrow_mut();
            
            if *recording {
                // Stop recording and process
                println!("🛑 Stopping recording...");
                button_clone.set_sensitive(false); // Disable button during processing
                
                if let Ok(mut rec) = recorder_clone.lock() {
                    match rec.stop_recording() {
                        Ok(audio_data) => {
                            println!("📤 Got {} bytes of audio", audio_data.len());
                            
                            // Change button state immediately after stopping recording
                            button_clone.set_css_classes(&["circular-button", "processing"]);
                            *recording = false;
                            
                            // Process transcription in background
                            let groq_clone = groq_client_clone.clone();
                            let injector_clone = text_injector_clone.clone();
                            let button_final = button_clone.clone();
                            
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                rt.block_on(async move {
                                    match groq_clone.transcribe_audio(audio_data).await {
                                        Ok(transcription) => {
                                            println!("🔥 AUTO-INJECTING: \"{}\"", transcription);
                                            
                                            // Simple text injection (working version)
                                            if let Ok(mut injector) = injector_clone.lock() {
                                                if let Err(e) = injector.inject_via_clipboard(&transcription) {
                                                    println!("❌ Text injection failed: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            println!("❌ Transcription failed: {}", e);
                                        }
                                    }
                                    
                                    // Re-enable button when done
                                    glib::spawn_future_local(async move {
                                        button_final.set_css_classes(&["circular-button", "ready"]);
                                        button_final.set_sensitive(true);
                                    });
                                });
                            });
                        }
                        Err(e) => {
                            println!("❌ Failed to stop recording: {}", e);
                            button_clone.set_css_classes(&["circular-button", "ready"]);
                            button_clone.set_sensitive(true);
                            *recording = false;
                        }
                    }
                } else {
                    button_clone.set_css_classes(&["circular-button", "ready"]);
                    button_clone.set_sensitive(true);
                    *recording = false;
                }
            } else {
                // Start recording
                println!("🎤 Starting recording...");
                if let Ok(mut rec) = recorder_clone.lock() {
                    match rec.start_recording() {
                        Ok(_) => {
                            button_clone.set_css_classes(&["circular-button", "recording"]);
                            *recording = true;
                        }
                        Err(e) => println!("❌ Failed to start recording: {}", e),
                    }
                }
            }
        });
        
        // Load CSS for circular button styling
        Self::load_css();
        
        Self {
            window,
            button,
            recorder,
            text_injector,
            groq_client,
            is_recording,
        }
    }
    
    fn load_css() {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(
            "
            .circular-button {
                border-radius: 50%;
                background: linear-gradient(135deg, #3b82f6, #2563eb);
                border: 2px solid rgba(255, 255, 255, 0.3);
                box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
                color: white;
                font-size: 24px;
            }
            
            .circular-button.recording {
                background: linear-gradient(135deg, #ef4444, #dc2626);
                animation: pulse 1s infinite;
            }
            
            .circular-button.processing {
                background: linear-gradient(135deg, #f59e0b, #d97706);
                animation: pulse 0.5s infinite;
            }
            
            .circular-button.ready {
                background: linear-gradient(135deg, #3b82f6, #2563eb);
            }
            
            .circular-button:hover {
                transform: scale(1.05);
                transition: transform 0.2s ease;
            }
            
            @keyframes pulse {
                0% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.7); }
                70% { box-shadow: 0 0 0 10px rgba(239, 68, 68, 0); }
                100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0); }
            }
            
            window {
                background: transparent;
            }
            "
        );
        
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    
    fn show(&self) {
        self.window.present();
    }
}


fn main() -> glib::ExitCode {
    // Initialize GTK4
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        println!("🚀 AquaVoice GTK4 Starting...");
        
        
        let floating_mic = FloatingMic::new(app);
        floating_mic.show();
        
        println!("✅ Floating microphone ready!");
    });

    app.run()
}