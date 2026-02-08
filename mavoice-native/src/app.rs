use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId, WindowLevel};

use crate::api::GroqClient;
use crate::audio::GroqRecorder;
use crate::config::Config;
use crate::renderer::{Renderer, Uniforms};
use crate::state_machine::{OverlayState, VisualState};
use crate::system::{HotkeyManager, TextInjector};

/// Events sent from async tasks back to the event loop
#[derive(Debug)]
pub enum AppEvent {
    TranscriptionComplete(String),
    TranscriptionError(String),
}

/// Click tracking for single/double click detection
struct ClickState {
    count: u32,
    timer: Option<std::time::Instant>,
    cooldown_until: Option<std::time::Instant>,
}

/// In-app Alt double-press tracking
struct AltPressState {
    count: u32,
    timer: Option<std::time::Instant>,
}

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    visual: VisualState,
    recorder: Arc<Mutex<GroqRecorder>>,
    groq_client: GroqClient,
    text_injector: TextInjector,
    config: Config,
    hotkey_manager: Option<HotkeyManager>,
    tokio_rt: Arc<tokio::runtime::Runtime>,
    event_proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    click_state: ClickState,
    alt_state: AltPressState,
    last_transcript: String,
    is_dragging: bool,
    /// Window ID of the app that was focused before overlay interaction
    previous_window_id: Option<String>,
}

impl App {
    pub fn new(
        tokio_rt: Arc<tokio::runtime::Runtime>,
        event_proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    ) -> Self {
        let config = Config::load();

        let recorder = GroqRecorder::new().expect("Failed to init audio recorder");
        let groq_client = GroqClient::new(config.api_key.clone());
        let text_injector = TextInjector::new().expect("Failed to init text injector");

        if !groq_client.has_api_key() {
            log::warn!(
                "No API key set! Edit {} or set GROQ_API_KEY env var",
                Config::config_path().display()
            );
        }

        Self {
            window: None,
            renderer: None,
            visual: VisualState::new(),
            recorder: Arc::new(Mutex::new(recorder)),
            groq_client,
            text_injector,
            config,
            hotkey_manager: None,
            tokio_rt,
            event_proxy,
            click_state: ClickState {
                count: 0,
                timer: None,
                cooldown_until: None,
            },
            alt_state: AltPressState {
                count: 0,
                timer: None,
            },
            last_transcript: String::new(),
            is_dragging: false,
            previous_window_id: None,
        }
    }

    fn is_recording(&self) -> bool {
        self.recorder.lock().unwrap().is_recording()
    }

    fn start_recording(&mut self) {
        if self.is_recording() {
            return;
        }

        // Capture the currently focused window BEFORE we steal focus
        self.previous_window_id = self.text_injector.get_active_window_id();
        if let Some(ref id) = self.previous_window_id {
            log::info!("Captured previous window: {}", id);
        }

        log::info!("Starting recording");
        if let Err(e) = self.recorder.lock().unwrap().start_recording() {
            log::error!("Failed to start recording: {}", e);
            return;
        }
        self.visual.set_state(OverlayState::Recording);
    }

    fn stop_recording_and_transcribe(&mut self) {
        if !self.is_recording() {
            return;
        }
        log::info!("Stopping recording, starting transcription");

        let wav_data = match self.recorder.lock().unwrap().stop_recording() {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to stop recording: {}", e);
                self.visual.set_state(OverlayState::Idle);
                return;
            }
        };

        self.visual.set_state(OverlayState::Processing);

        // Spawn async transcription on tokio runtime
        let client = self.groq_client.clone();
        let proxy = self.event_proxy.clone();
        let model = Some(self.config.model.clone());
        let language = self.config.effective_language().map(|s| s.to_string());
        let dictionary = self.config.effective_dictionary().map(|s| s.to_string());
        let temperature = Some(self.config.temperature);
        let response_format = Some(self.config.response_format.clone());

        self.tokio_rt.spawn(async move {
            match client
                .transcribe_audio_bytes(
                    &wav_data,
                    "recording.wav",
                    model.as_deref(),
                    language.as_deref(),
                    dictionary.as_deref(),
                    response_format.as_deref(),
                    temperature,
                )
                .await
            {
                Ok(text) => {
                    let _ = proxy.send_event(AppEvent::TranscriptionComplete(text));
                }
                Err(e) => {
                    let _ = proxy.send_event(AppEvent::TranscriptionError(e.to_string()));
                }
            }
        });
    }

    fn handle_transcription_result(&mut self, text: String) {
        log::info!("Transcription: {}", text);
        self.last_transcript = text.clone();
        self.visual.set_state(OverlayState::Done);

        // Inject text into the previously focused window (not the overlay)
        let target = self.previous_window_id.as_deref();
        if let Err(e) = self.text_injector.inject_text_to(&text, target) {
            log::error!("Text injection failed: {}", e);
        }
    }

    fn toggle_recording(&mut self) {
        if self.is_recording() {
            self.stop_recording_and_transcribe();
        } else {
            // For global hotkey: capture focused window (overlay isn't focused)
            if self.previous_window_id.is_none() {
                self.previous_window_id = self.text_injector.get_active_window_id();
                if let Some(ref id) = self.previous_window_id {
                    log::info!("Captured previous window via hotkey: {}", id);
                }
            }
            self.start_recording();
        }
    }

    fn set_skip_taskbar(_window: &Window) {
        // Use xdotool to set skip-taskbar by window name (works on X11)
        // This is a post-creation escape hatch since winit doesn't expose atom APIs
        let _ = std::process::Command::new("xdotool")
            .args(["search", "--name", "maVoice", "set_window", "--skip-taskbar", "1"])
            .output();
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Detect screen dimensions
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        let (screen_w, screen_h) = monitor
            .map(|m| {
                let s = m.size();
                (s.width, s.height)
            })
            .unwrap_or((1920, 1080));

        let strip_w = screen_w as f64;
        let strip_h = 64.0;

        let attrs = Window::default_attributes()
            .with_title("maVoice")
            .with_inner_size(LogicalSize::new(strip_w, strip_h))
            .with_position(LogicalPosition::new(0.0, screen_h as f64 - strip_h))
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        Self::set_skip_taskbar(&window);

        // Init wgpu renderer on tokio runtime (async)
        let win_clone = window.clone();
        let renderer = self
            .tokio_rt
            .block_on(async { Renderer::new(win_clone).await });

        self.renderer = Some(renderer);
        self.window = Some(window);

        // Init global hotkeys
        match HotkeyManager::new() {
            Ok(hk) => self.hotkey_manager = Some(hk),
            Err(e) => log::warn!("Global hotkeys unavailable: {}", e),
        }

        log::info!(
            "Window created: {}x{} at bottom of {}x{} screen",
            strip_w,
            strip_h,
            screen_w,
            screen_h
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
            }

            WindowEvent::RedrawRequested => {
                // Poll audio levels
                let raw_levels = self.recorder.lock().unwrap().get_audio_levels();

                // Update visual state
                let needs_redraw = self.visual.update(raw_levels);

                if let Some(r) = &self.renderer {
                    let size = r.surface_config.width as f32;
                    let height = r.surface_config.height as f32;

                    let uniforms = Uniforms {
                        resolution: [size, height],
                        time: r.elapsed(),
                        intensity: self.visual.effective_intensity(),
                        levels: self.visual.effective_levels(),
                        color: self.visual.color,
                        mode: self.visual.mode,
                    };

                    match r.render(&uniforms) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            let w = r.surface_config.width;
                            let h = r.surface_config.height;
                            if let Some(r) = &mut self.renderer {
                                r.resize(w, h);
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("Out of GPU memory");
                            event_loop.exit();
                        }
                        Err(e) => log::warn!("Render error: {:?}", e),
                    }
                }

                // Request next frame if active
                if needs_redraw || self.visual.state != OverlayState::Idle {
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }

            // --- Mouse click handling ---
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                match button {
                    MouseButton::Left => {
                        if self.is_dragging {
                            return;
                        }
                        // Ignore clicks during cooldown (after starting recording)
                        if let Some(cd) = self.click_state.cooldown_until {
                            if std::time::Instant::now() < cd {
                                return;
                            }
                            self.click_state.cooldown_until = None;
                        }
                        self.click_state.count += 1;
                        self.click_state.timer = Some(std::time::Instant::now());
                    }
                    MouseButton::Right => {
                        // Right-click drag
                        self.is_dragging = true;
                        if let Some(w) = &self.window {
                            let _ = w.drag_window();
                        }
                    }
                    _ => {}
                }
            }

            // --- Keyboard handling ---
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                match &event.logical_key {
                    // Alt+Space → toggle recording
                    Key::Named(NamedKey::Space) => {
                        // Check if Alt is held by looking at modifiers
                        // For simplicity, Space alone stops recording
                        if self.is_recording() {
                            self.stop_recording_and_transcribe();
                        }
                    }

                    // Alt key double-press detection
                    Key::Named(NamedKey::Alt) => {
                        self.alt_state.count += 1;
                        self.alt_state.timer = Some(std::time::Instant::now());
                    }

                    _ => {}
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Process click timer (280ms window for double-click)
        if let Some(timer) = self.click_state.timer {
            if timer.elapsed().as_millis() >= 280 {
                let count = self.click_state.count;
                self.click_state.count = 0;
                self.click_state.timer = None;

                if count == 1 {
                    if self.is_recording() {
                        self.stop_recording_and_transcribe();
                    }
                    // Single click while idle with transcript → could copy
                } else if count >= 2 && !self.is_recording() {
                    self.start_recording();
                    // Cooldown: ignore clicks for 500ms after starting
                    self.click_state.cooldown_until = Some(
                        std::time::Instant::now() + std::time::Duration::from_millis(500),
                    );
                }
            }
        }

        // Process Alt double-press timer (400ms window)
        if let Some(timer) = self.alt_state.timer {
            if timer.elapsed().as_millis() >= 400 {
                let count = self.alt_state.count;
                self.alt_state.count = 0;
                self.alt_state.timer = None;

                if count >= 2 && !self.is_recording() {
                    self.start_recording();
                }
            }
        }

        // Check global hotkeys
        if let Some(ref hk) = self.hotkey_manager {
            if hk.poll_toggle() {
                self.toggle_recording();
            }
        }

        // Drive animation — request redraw when anything is visible
        if self.visual.state != OverlayState::Idle || self.visual.intensity > 0.001 {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }

        // Reset drag state
        self.is_dragging = false;
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::TranscriptionComplete(text) => {
                self.handle_transcription_result(text);
                // Trigger redraw for Done state
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            AppEvent::TranscriptionError(err) => {
                log::error!("Transcription error: {}", err);
                self.visual.set_state(OverlayState::Idle);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }
    }
}
