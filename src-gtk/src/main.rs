//! AquaVoice GTK – main entry

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button};
use gtk::gdk::Key;

use glib::{clone, MainContext};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod audio_recorder;
mod text_injector;
mod groq_client;

use audio_recorder::GroqRecorder;
use text_injector::TextInjector;
use groq_client::{get_groq_api_key, GroqClient};

const APP_ID: &str = "com.aquavoice.gtk";

/// Top-level UI widget
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
        // ───── Window ───────────────────────────────────────────────────────
        let window = ApplicationWindow::builder()
            .application(app)
            .title("AquaVoice")
            .default_width(128)  // Exact button size
            .default_height(36)
            .decorated(false)
            .resizable(false)
            .build();
        window.set_deletable(false);           // no close button
        
        // Enable transparency support
        window.set_opacity(0.98);

        // ───── Single unified button (no container padding) ───────────────────────────────────────────────────
        let button = Button::builder()
            .width_request(128)
            .height_request(36)
            .build();
        button.set_css_classes(&["island-button", "ready"]);
        
        // Set button directly as window child (no containers!)
        window.set_child(Some(&button));

        // ───── Runtime state ─────────────────────────────────────────────
        let is_recording = Rc::new(RefCell::new(false));

        // ───── Mouse gestures ────────────────────────────────────────────────
        // Click gestures for recording
        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_button(1); // left mouse button
        click_gesture.connect_pressed(clone!(
            @weak button,
            @strong is_recording
            => move |gesture, n_press, _x, _y| {
                if n_press == 1 && *is_recording.borrow() {
                    // Single click while recording stops it
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    button.activate();
                } else if n_press == 2 && !*is_recording.borrow() {
                    // Double-click when not recording starts it
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    button.activate();
                }
            }
        ));
        button.add_controller(click_gesture);
        
        // Drag gesture for moving
        let drag_gesture = gtk::GestureDrag::new();
        drag_gesture.set_button(1); // left button
        drag_gesture.connect_drag_begin(clone!(@weak window => move |g, _x, _y| {
            if let (Some(device), Some(ev)) = (g.device(), g.current_event()) {
                if let Some((root_x, root_y)) = ev.position() {
                    let surface = window.surface();
                    if let Ok(toplevel) = surface.downcast::<gtk::gdk::Toplevel>() {
                        toplevel.begin_move(
                            &device,
                            g.current_button() as i32,
                            root_x,
                            root_y,
                            g.current_event_time(),
                        );
                    }
                }
            }
        }));
        window.add_controller(drag_gesture);
        
        button.set_tooltip_text(Some(
            "Double-click or Space+Alt² to record • Single click/Space to stop",
        ));

        // ───── Hotkey combination: Hold Space + Double-tap Alt ────────────────────────────────
        let key_ctrl = gtk::EventControllerKey::new();
        
        // Track key states
        let space_pressed = Rc::new(RefCell::new(false));
        let alt_tap_count = Rc::new(RefCell::new(0));
        let last_alt_tap = Rc::new(RefCell::new(std::time::Instant::now()));
        
        // Handle key press events
        key_ctrl.connect_key_pressed(clone!(
            @weak button,
            @strong space_pressed,
            @strong alt_tap_count,
            @strong last_alt_tap,
            @strong is_recording
            => @default-return glib::Propagation::Proceed,
            move |_, key, _kc, _state| {
                match key {
                    Key::space => {
                        // If recording, space stops recording immediately
                        if *is_recording.borrow() {
                            button.activate();
                        } else {
                            *space_pressed.borrow_mut() = true;
                        }
                    }
                    Key::Alt_L | Key::Alt_R => {
                        // Only count Alt taps if Space is held and not currently recording
                        if *space_pressed.borrow() && !*is_recording.borrow() {
                            let now = std::time::Instant::now();
                            let time_since_last = now.duration_since(*last_alt_tap.borrow());
                            
                            // Reset count if too much time passed (500ms window for double-tap)
                            if time_since_last.as_millis() > 500 {
                                *alt_tap_count.borrow_mut() = 0;
                            }
                            
                            *alt_tap_count.borrow_mut() += 1;
                            *last_alt_tap.borrow_mut() = now;
                            
                            // Trigger on second Alt tap while Space is held
                            if *alt_tap_count.borrow() == 2 {
                                button.activate();
                                *alt_tap_count.borrow_mut() = 0; // Reset for next activation
                            }
                        }
                    }
                    _ => {}
                }
                glib::Propagation::Proceed
            }
        ));
        
        // Handle key release events
        key_ctrl.connect_key_released(clone!(
            @strong space_pressed,
            @strong alt_tap_count
            => move |_, key, _kc, _state| {
                if key == Key::space {
                    *space_pressed.borrow_mut() = false;
                    *alt_tap_count.borrow_mut() = 0; // Reset Alt tap count when Space is released
                }
            }
        ));
        
        window.add_controller(key_ctrl);

        // ───── Runtime objects ─────────────────────────────────────────────
        let recorder = Arc::new(Mutex::new(
            GroqRecorder::new().expect("Failed to init recorder"),
        ));
        let text_injector = Arc::new(Mutex::new(TextInjector::new()));
        let groq_client = Arc::new(GroqClient::new(
            get_groq_api_key().expect("GROQ_API_KEY not set"),
        ));

        // ───── Async channel for background results ─────────────────────────
        let (tx, rx) = async_channel::unbounded::<Result<String, String>>();

        let button_weak = button.downgrade();
        let text_injector_clone = text_injector.clone();
        
        MainContext::default().spawn_local(async move {
            while let Ok(msg) = rx.recv().await {
                match msg {
                    Ok(text) => {
                        println!("🔥 TRANSCRIBED:\n{text}");
                        if let Ok(mut inj) = text_injector_clone.lock() {
                            inj.inject_via_clipboard(&text).ok();
                        }
                    }
                    Err(e) => eprintln!("❌ Transcription failed: {e}"),
                }
                if let Some(btn) = button_weak.upgrade() {
                    btn.set_css_classes(&["island-button", "ready"]);
                    btn.set_sensitive(true);
                }
            }
        });

        // ───── Button click handler (start/stop) ───────────────────────────
        button.connect_clicked(clone!(
            @strong recorder,
            @strong is_recording,
            @strong groq_client,
            @strong tx,
            => move |btn| {
                let mut rec_flag = is_recording.borrow_mut();
                if *rec_flag {
                    // ==== STOP ==================================================
                    btn.set_sensitive(false);
                    if let Ok(mut rec) = recorder.lock() {
                        match rec.stop_recording() {
                            Ok(audio_data) => {
                                btn.set_css_classes(&["island-button", "processing"]);
                                *rec_flag = false;

                                let groq_clone = groq_client.clone();
                                let tx_thread = tx.clone();
                                std::thread::spawn(move || {
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let result = rt.block_on(async {
                                        groq_clone
                                            .transcribe_audio(audio_data)
                                            .await
                                            .map_err(|e| e.to_string())
                                    });
                                    tx_thread.send_blocking(result).ok();
                                });
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to stop recording: {e}");
                                btn.set_css_classes(&["island-button", "ready"]);
                                btn.set_sensitive(true);
                                *rec_flag = false;
                            }
                        }
                    }
                } else {
                    // ==== START =================================================
                    println!("🎤 Starting recording …");
                    if let Ok(mut rec) = recorder.lock() {
                        match rec.start_recording() {
                            Ok(_) => {
                                btn.set_css_classes(&["island-button", "recording"]);
                                *rec_flag = true;
                            }
                            Err(e) => eprintln!("❌ Failed to start recording: {e}"),
                        }
                    }
                }
            }
        ));

        // ───── CSS ─────────────────────────────────────────────────────────
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

    /// Load embedded CSS
    fn load_css() {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(
            "
            /* 2025 Dynamic Island CSS - NO CONTAINER, PURE FLOATING BUTTON */
            window {
                background-color: rgba(0, 0, 0, 0);
                border: none;
                padding: 0;
                margin: 0;
            }
            
            .island-button {
                background: rgba(15, 15, 15, 0.95);
                border-radius: 18px;
                border: none;
                box-shadow: 0 8px 32px rgba(0, 0, 0, 0.9),
                           0 0 0 0.5px rgba(255, 255, 255, 0.15),
                           inset 0 1px 0 rgba(255, 255, 255, 0.2);
                transition: all 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94);
                padding: 0;
                margin: 0;
                font-family: -apple-system, 'SF Pro Display', system-ui, sans-serif;
                font-weight: 600;
                font-size: 11px;
                letter-spacing: -0.1px;
                color: rgba(255, 255, 255, 0.9);
                /* Fill entire window - no padding or margins */
                min-width: 128px;
                min-height: 36px;
            }
            
            .island-button.ready {
                background: rgba(15, 15, 15, 0.92);
                color: rgba(255, 255, 255, 0.9);
            }
            
            .island-button.ready:hover {
                background: rgba(25, 25, 25, 0.94);
                box-shadow: 0 12px 48px rgba(0, 0, 0, 1.0),
                           0 0 0 0.5px rgba(255, 255, 255, 0.18),
                           inset 0 1px 0 rgba(255, 255, 255, 0.25);
                transform: scale(1.08) translateY(-3px);
            }
            
            .island-button.recording {
                background: rgba(255, 59, 48, 0.95);
                color: rgba(255, 255, 255, 1);
                animation: island-record-pulse 2s ease-in-out infinite;
            }
            
            .island-button.processing {
                background: rgba(255, 149, 0, 0.95);
                color: rgba(255, 255, 255, 1);
                animation: island-process-wave 1.2s ease-in-out infinite;
            }
            
            @keyframes island-record-pulse {
                0%, 100% {
                    box-shadow: 0 8px 32px rgba(255, 59, 48, 0.5),
                               0 0 0 0.5px rgba(255, 255, 255, 0.15),
                               inset 0 1px 0 rgba(255, 255, 255, 0.25);
                    background: rgba(255, 59, 48, 0.95);
                }
                50% {
                    box-shadow: 0 12px 48px rgba(255, 59, 48, 0.8),
                               0 0 32px rgba(255, 59, 48, 0.6),
                               0 0 0 0.5px rgba(255, 255, 255, 0.2),
                               inset 0 1px 0 rgba(255, 255, 255, 0.4);
                    background: rgba(255, 69, 58, 0.98);
                    transform: scale(1.08);
                }
            }
            
            @keyframes island-process-wave {
                0% {
                    box-shadow: 0 8px 32px rgba(255, 149, 0, 0.5),
                               0 0 0 0.5px rgba(255, 255, 255, 0.15),
                               inset 0 1px 0 rgba(255, 255, 255, 0.25);
                }
                25% {
                    box-shadow: 0 10px 36px rgba(255, 149, 0, 0.6),
                               0 0 16px rgba(255, 149, 0, 0.4),
                               0 0 0 0.5px rgba(255, 255, 255, 0.18),
                               inset 0 1px 0 rgba(255, 255, 255, 0.3);
                }
                50% {
                    box-shadow: 0 12px 40px rgba(255, 149, 0, 0.7),
                               0 0 24px rgba(255, 149, 0, 0.5),
                               0 0 0 0.5px rgba(255, 255, 255, 0.2),
                               inset 0 1px 0 rgba(255, 255, 255, 0.35);
                }
                75% {
                    box-shadow: 0 10px 36px rgba(255, 149, 0, 0.6),
                               0 0 16px rgba(255, 149, 0, 0.4),
                               0 0 0 0.5px rgba(255, 255, 255, 0.18),
                               inset 0 1px 0 rgba(255, 255, 255, 0.3);
                }
                100% {
                    box-shadow: 0 8px 32px rgba(255, 149, 0, 0.5),
                               0 0 0 0.5px rgba(255, 255, 255, 0.15),
                               inset 0 1px 0 rgba(255, 255, 255, 0.25);
                }
            }
            ",
        );
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("No display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    fn show(&self) {
        self.window.present();
    }
}

fn main() -> glib::ExitCode {
    // Load .env file if it exists
    if let Err(e) = dotenv::dotenv() {
        eprintln!("⚠️  Could not load .env file: {}", e);
    }
    
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        println!("🚀 AquaVoice GTK4 starting …");
        let mic = FloatingMic::new(app);
        mic.show();
    });
    app.run()
}
