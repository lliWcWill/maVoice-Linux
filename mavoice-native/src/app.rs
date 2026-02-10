use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId, WindowLevel};

use serde_json::json;

use crate::api::gemini::{FunctionCall, FunctionResponse, GeminiEvent};
use crate::api::{GeminiLiveClient, GroqClient};
use crate::audio::{AudioPlayer, GroqRecorder};
use crate::dashboard::DashboardBroadcaster;

/// Global storage for the Gemini client (needed because it's created in an async task
/// but used from the winit event loop thread). Protected by Mutex.
static GEMINI_CLIENT: std::sync::LazyLock<Mutex<Option<GeminiLiveClient>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Global storage for the dashboard broadcast server.
static DASHBOARD: std::sync::LazyLock<Mutex<Option<DashboardBroadcaster>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));
use crate::config::Config;
use crate::renderer::{AiUniforms, GpuContext, Renderer, UserUniforms};
use crate::state_machine::{OverlayState, VisualState};
use crate::system::{HotkeyManager, TextInjector};

/// Current time as Unix milliseconds (for dashboard event timestamps).
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Voice mode — determines hotkey behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoiceMode {
    Groq,       // Mode A: record → Groq Whisper → text paste
    GeminiLive, // Mode B: stream → bidirectional voice with Gemini
}

/// Events sent from async tasks back to the event loop
#[derive(Debug)]
pub enum AppEvent {
    TranscriptionComplete(String),
    TranscriptionError(String),
    // Gemini Live events
    GeminiReady,
    GeminiAudio(Vec<u8>),
    GeminiText(String),
    GeminiInterrupted,
    GeminiTurnComplete,
    GeminiToolCall(Vec<FunctionCall>),
    GeminiToolCallCancellation(Vec<String>),
    GeminiError(String),
    GeminiClosed(String),
    // Tool execution results
    ToolResult {
        call_id: String,
        name: String,
        result: serde_json::Value,
    },
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
    // User window (bottom strip) — existing waveform
    user_window: Option<Arc<Window>>,
    user_window_id: Option<WindowId>,
    user_renderer: Option<Renderer>,
    // AI window (top strip) — Gemini bubble
    ai_window: Option<Arc<Window>>,
    ai_window_id: Option<WindowId>,
    ai_renderer: Option<Renderer>,
    // Shared GPU context
    gpu: Option<GpuContext>,
    // Visual state
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
    // Gemini Live fields
    mode: VoiceMode,
    /// Which mode started the current recording (so we stop correctly)
    recording_mode: Option<VoiceMode>,
    audio_player: Option<AudioPlayer>,
    gemini_connecting: bool,
    /// IDs of tool calls currently in flight (for cancellation tracking)
    pending_tool_calls: HashSet<String>,
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

        let initial_mode = if config.mode == "gemini" {
            VoiceMode::GeminiLive
        } else {
            VoiceMode::Groq
        };

        Self {
            user_window: None,
            user_window_id: None,
            user_renderer: None,
            ai_window: None,
            ai_window_id: None,
            ai_renderer: None,
            gpu: None,
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
            mode: initial_mode,
            recording_mode: None,
            audio_player: None,
            gemini_connecting: false,
            pending_tool_calls: HashSet::new(),
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
        self.broadcast_dashboard("groq:start", json!({ "timestamp": now_ms() }));

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

    // ── Gemini Live methods ──────────────────────────────────────────

    /// True if Gemini session is active (connected or connecting, mic streaming)
    fn gemini_session_active(&self) -> bool {
        let has_client = GEMINI_CLIENT
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.is_open())
            .unwrap_or(false);
        has_client || self.gemini_connecting
    }

    /// Toggle Gemini Live session on/off.
    fn toggle_gemini_session(&mut self) {
        if self.gemini_session_active() || self.is_recording() {
            self.disconnect_gemini();
        } else {
            self.connect_gemini();
        }
    }

    /// Connect to Gemini Live and start continuous mic streaming.
    fn connect_gemini(&mut self) {
        if self.gemini_session_active() {
            return;
        }

        if self.config.gemini_api_key.is_empty() {
            log::error!("[Gemini] No API key! Set GEMINI_API_KEY or add gemini_api_key to config.toml");
            return;
        }

        log::info!("[Gemini] Starting live session...");

        // Init audio player if needed
        if self.audio_player.is_none() {
            match AudioPlayer::new() {
                Ok(player) => self.audio_player = Some(player),
                Err(e) => {
                    log::error!("Failed to init audio player: {}", e);
                    return;
                }
            }
        }

        self.gemini_connecting = true;
        self.visual.set_state(OverlayState::Processing);

        let api_key = self.config.gemini_api_key.clone();
        let voice_name = self.config.voice_name.clone();
        let system_instruction = self.config.system_instruction.clone();
        let proxy = self.event_proxy.clone();

        self.tokio_rt.spawn(async move {
            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<GeminiEvent>();

            match GeminiLiveClient::connect(&api_key, &voice_name, &system_instruction, event_tx)
                .await
            {
                Ok(client) => {
                    log::info!("[Gemini] WebSocket connected, starting event bridge");

                    let proxy_clone = proxy.clone();
                    tokio::spawn(async move {
                        while let Some(event) = event_rx.recv().await {
                            let app_event = match event {
                                GeminiEvent::Ready => AppEvent::GeminiReady,
                                GeminiEvent::Audio(data) => AppEvent::GeminiAudio(data),
                                GeminiEvent::Text(text) => AppEvent::GeminiText(text),
                                GeminiEvent::Interrupted => AppEvent::GeminiInterrupted,
                                GeminiEvent::TurnComplete => AppEvent::GeminiTurnComplete,
                                GeminiEvent::ToolCall(calls) => AppEvent::GeminiToolCall(calls),
                                GeminiEvent::ToolCallCancellation(ids) => {
                                    AppEvent::GeminiToolCallCancellation(ids)
                                }
                                GeminiEvent::Error(e) => AppEvent::GeminiError(e),
                                GeminiEvent::Closed(reason) => AppEvent::GeminiClosed(reason),
                            };
                            if proxy_clone.send_event(app_event).is_err() {
                                break;
                            }
                        }
                    });

                    GEMINI_CLIENT.lock().unwrap().replace(client);
                    let _ = proxy.send_event(AppEvent::GeminiReady);
                }
                Err(e) => {
                    log::error!("[Gemini] Connection failed: {}", e);
                    let _ = proxy.send_event(AppEvent::GeminiError(e));
                }
            }
        });
    }

    /// Start continuous mic streaming after Gemini connection is ready.
    fn start_gemini_mic(&mut self) {
        if self.is_recording() {
            return;
        }

        log::info!("[Gemini] Starting continuous mic stream");

        // Set up streaming callback — sends audio to Gemini in real-time
        let streaming_cb: crate::audio::recorder::StreamingCallback =
            Arc::new(move |pcm_s16le: &[u8]| {
                let guard = GEMINI_CLIENT.lock().unwrap();
                if let Some(ref c) = *guard {
                    c.send_audio(pcm_s16le);
                }
            });
        self.recorder
            .lock()
            .unwrap()
            .set_streaming_callback(Some(streaming_cb));

        if let Err(e) = self.recorder.lock().unwrap().start_recording() {
            log::error!("Failed to start recording: {}", e);
            return;
        }

        // Mic is live — user waveform always visible
        self.visual.set_state(OverlayState::Listening);
    }

    /// Disconnect from Gemini Live and stop everything.
    fn disconnect_gemini(&mut self) {
        log::info!("[Gemini] Disconnecting session");

        // Stop mic
        if self.is_recording() {
            let _ = self.recorder.lock().unwrap().stop_recording();
        }
        self.recorder.lock().unwrap().set_streaming_callback(None);

        // Close WebSocket
        if let Some(client) = GEMINI_CLIENT.lock().unwrap().take() {
            client.close();
        }

        // Clear audio playback
        if let Some(ref player) = self.audio_player {
            player.clear();
        }

        self.gemini_connecting = false;
        self.recording_mode = None;
        self.visual.set_state(OverlayState::Idle);
    }

    /// Dispatch tool calls to async executors, tracking their IDs.
    fn dispatch_tool_calls(&mut self, calls: Vec<FunctionCall>) {
        for call in calls {
            self.pending_tool_calls.insert(call.id.clone());

            let proxy = self.event_proxy.clone();
            let call_id = call.id.clone();
            let call_name = call.name.clone();
            let call_args = call.args.clone();

            self.tokio_rt.spawn(async move {
                let result = crate::tools::execute(&call_name, &call_args).await;
                let _ = proxy.send_event(AppEvent::ToolResult {
                    call_id,
                    name: call_name,
                    result,
                });
            });
        }
    }

    fn set_skip_taskbar(name: &str) {
        // Use xdotool to set skip-taskbar by window name (works on X11)
        let _ = std::process::Command::new("xdotool")
            .args(["search", "--name", name, "set_window", "--skip-taskbar", "1"])
            .output();
    }

    /// Request redraw on both windows
    fn request_redraw_all(&self) {
        if let Some(w) = &self.user_window {
            w.request_redraw();
        }
        if let Some(w) = &self.ai_window {
            w.request_redraw();
        }
    }

    /// Broadcast a JSON event to connected dashboard clients.
    fn broadcast_dashboard(&self, event_type: &str, payload: serde_json::Value) {
        if let Some(ref server) = *DASHBOARD.lock().unwrap() {
            server.broadcast(event_type, payload);
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.user_window.is_some() {
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

        // ── Create USER window (bottom, 64px) ──
        let user_h = 64.0;
        let user_attrs = Window::default_attributes()
            .with_title("maVoice")
            .with_inner_size(LogicalSize::new(strip_w, user_h))
            .with_position(LogicalPosition::new(0.0, screen_h as f64 - user_h))
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);

        let user_window = Arc::new(
            event_loop
                .create_window(user_attrs)
                .expect("Failed to create user window"),
        );

        // ── Create AI window (compact floating orb, centered at top) ──
        let ai_w = 400.0;
        let ai_h = 200.0;
        let ai_x = (screen_w as f64 - ai_w) / 2.0;
        let ai_attrs = Window::default_attributes()
            .with_title("maVoice-AI")
            .with_inner_size(LogicalSize::new(ai_w, ai_h))
            .with_position(LogicalPosition::new(ai_x, 0.0))
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);

        let ai_window = Arc::new(
            event_loop
                .create_window(ai_attrs)
                .expect("Failed to create AI window"),
        );

        // ── Init shared GPU context (no surface needed — render to texture) ──
        let gpu = self
            .tokio_rt
            .block_on(async { GpuContext::new().await });

        // ── Create renderers ──
        let user_renderer = Renderer::new(
            &gpu,
            user_window.clone(),
            include_str!("shader.wgsl"),
            std::mem::size_of::<UserUniforms>(),
        );

        let ai_renderer = Renderer::new(
            &gpu,
            ai_window.clone(),
            include_str!("ai_shader.wgsl"),
            std::mem::size_of::<AiUniforms>(),
        );

        // Store window IDs for event routing
        self.user_window_id = Some(user_window.id());
        self.ai_window_id = Some(ai_window.id());

        self.user_renderer = Some(user_renderer);
        self.ai_renderer = Some(ai_renderer);
        self.gpu = Some(gpu);
        self.user_window = Some(user_window);
        self.ai_window = Some(ai_window);

        // Skip taskbar for both windows
        Self::set_skip_taskbar("maVoice");
        Self::set_skip_taskbar("maVoice-AI");

        // Init global hotkeys
        match HotkeyManager::new() {
            Ok(hk) => self.hotkey_manager = Some(hk),
            Err(e) => log::warn!("Global hotkeys unavailable: {}", e),
        }

        log::info!(
            "Windows created: user={}x{} (bottom), AI={}x{} (top center) on {}x{} screen",
            strip_w, user_h, ai_w, ai_h, screen_w, screen_h
        );

        // Start dashboard WebSocket broadcast server
        self.tokio_rt.spawn(async {
            match DashboardBroadcaster::start(3001).await {
                Ok(server) => {
                    DASHBOARD.lock().unwrap().replace(server);
                }
                Err(e) => log::warn!("[Dashboard] Failed to start: {}", e),
            }
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Route events by window ID
        let is_user_window = Some(window_id) == self.user_window_id;
        let is_ai_window = Some(window_id) == self.ai_window_id;

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if is_user_window {
                    if let Some(r) = &mut self.user_renderer {
                        r.resize(size.width, size.height);
                    }
                } else if is_ai_window {
                    if let Some(r) = &mut self.ai_renderer {
                        r.resize(size.width, size.height);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                // Poll audio levels from mic
                let raw_levels = self.recorder.lock().unwrap().get_audio_levels();

                // Poll audio levels from AI output (if playing)
                let output_levels = self
                    .audio_player
                    .as_ref()
                    .map(|p| p.get_output_levels())
                    .unwrap_or([0.0; 4]);

                // Update visual state with both channels
                self.visual.update_with_output(raw_levels, output_levels);

                let elapsed = self.gpu.as_ref().map(|g| g.elapsed()).unwrap_or(0.0);

                // ── Render user window ──
                if is_user_window {
                    if let Some(r) = &mut self.user_renderer {
                        let uniforms = UserUniforms {
                            resolution: [r.width as f32, r.height as f32],
                            time: elapsed,
                            intensity: self.visual.effective_intensity(),
                            levels: self.visual.effective_levels(),
                            color: self.visual.color,
                            mode: self.visual.mode,
                        };
                        r.render_bytes(bytemuck::bytes_of(&uniforms));
                    }
                }

                // ── Render AI window ──
                if is_ai_window {
                    if let Some(r) = &mut self.ai_renderer {
                        let uniforms = AiUniforms {
                            resolution: [r.width as f32, r.height as f32],
                            time: elapsed,
                            intensity: self.visual.effective_ai_intensity(),
                            levels: self.visual.effective_ai_levels(),
                            color: self.visual.ai_color,
                            _pad: 0.0,
                        };
                        r.render_bytes(bytemuck::bytes_of(&uniforms));
                    }
                }

                // Request next frame if active (either channel)
                let ai_playing = self
                    .audio_player
                    .as_ref()
                    .map(|p| p.is_playing())
                    .unwrap_or(false);
                if self.visual.state != OverlayState::Idle
                    || self.visual.intensity > 0.001
                    || self.visual.ai_intensity > 0.001
                    || ai_playing
                {
                    self.request_redraw_all();
                }
            }

            // --- Mouse click handling (user window only) ---
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } if is_user_window => {
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
                        if let Some(w) = &self.user_window {
                            let _ = w.drag_window();
                        }
                    }
                    _ => {}
                }
            }

            // --- Keyboard handling (user window only) ---
            WindowEvent::KeyboardInput { event, .. } if is_user_window => {
                if event.state != ElementState::Pressed {
                    return;
                }

                match &event.logical_key {
                    Key::Named(NamedKey::Space) => {
                        if self.is_recording() {
                            self.stop_recording_and_transcribe();
                        }
                    }
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
                } else if count >= 2 && !self.is_recording() {
                    self.start_recording();
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
            let poll = hk.poll();
            if poll.toggle_fired {
                if self.gemini_session_active() || self.recording_mode == Some(VoiceMode::GeminiLive) {
                    self.disconnect_gemini();
                }
                self.mode = VoiceMode::Groq;
                self.recording_mode = Some(VoiceMode::Groq);
                self.toggle_recording();
            }
            if poll.mode_switch_fired {
                if self.recording_mode == Some(VoiceMode::Groq) && self.is_recording() {
                    let _ = self.recorder.lock().unwrap().stop_recording();
                    self.visual.set_state(OverlayState::Idle);
                }
                self.mode = VoiceMode::GeminiLive;
                self.recording_mode = Some(VoiceMode::GeminiLive);
                self.toggle_gemini_session();
            }
        }

        // Drive animation — request redraw when anything is visible
        if self.visual.state != OverlayState::Idle
            || self.visual.intensity > 0.001
            || self.visual.ai_intensity > 0.001
        {
            self.request_redraw_all();
        }

        // Reset drag state
        self.is_dragging = false;
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::TranscriptionComplete(text) => {
                self.broadcast_dashboard("groq:complete", json!({
                    "text": text,
                    "timestamp": now_ms(),
                }));
                self.handle_transcription_result(text);
                self.request_redraw_all();
            }
            AppEvent::TranscriptionError(err) => {
                log::error!("Transcription error: {}", err);
                self.broadcast_dashboard("groq:error", json!({
                    "error": err,
                    "timestamp": now_ms(),
                }));
                self.visual.set_state(OverlayState::Idle);
                self.request_redraw_all();
            }

            // ── Gemini Live events ──

            AppEvent::GeminiReady => {
                log::info!("[Gemini] Ready — session established, starting mic");
                self.gemini_connecting = false;
                self.broadcast_dashboard("voice:open", json!({ "timestamp": now_ms() }));
                self.start_gemini_mic();
                self.request_redraw_all();
            }

            AppEvent::GeminiAudio(pcm_data) => {
                if let Some(ref player) = self.audio_player {
                    player.enqueue(&pcm_data);
                }
                if self.visual.state != OverlayState::AISpeaking {
                    log::info!("[Gemini] AI speaking — audio arriving");
                    self.broadcast_dashboard("voice:speaking", json!({ "timestamp": now_ms() }));
                    self.visual.set_state(OverlayState::AISpeaking);
                }
                self.request_redraw_all();
            }

            AppEvent::GeminiText(text) => {
                log::info!("[Gemini] Text: {}", text);
                self.broadcast_dashboard("voice:text", json!({
                    "text": text,
                    "timestamp": now_ms(),
                }));
            }

            AppEvent::GeminiInterrupted => {
                log::info!("[Gemini] Interrupted (barge-in)");
                self.broadcast_dashboard("voice:interrupted", json!({ "timestamp": now_ms() }));
                if let Some(ref player) = self.audio_player {
                    player.clear();
                }
                self.visual.set_state(OverlayState::Listening);
                self.request_redraw_all();
            }

            AppEvent::GeminiTurnComplete => {
                log::info!("[Gemini] Turn complete — back to listening");
                self.broadcast_dashboard("voice:listening", json!({ "timestamp": now_ms() }));
                self.visual.set_state(OverlayState::Listening);
                self.request_redraw_all();
            }

            AppEvent::GeminiToolCall(calls) => {
                log::info!("[Gemini] Tool calls received: {}", calls.len());
                let ts = now_ms();
                for call in &calls {
                    self.broadcast_dashboard("voice:tool_call", json!({
                        "chatId": call.id,
                        "toolName": call.name,
                        "input": call.args,
                        "timestamp": ts,
                    }));
                }
                self.dispatch_tool_calls(calls);
            }

            AppEvent::GeminiToolCallCancellation(ids) => {
                log::info!("[Gemini] Tool call cancellation: {:?}", ids);
                for id in &ids {
                    self.pending_tool_calls.remove(id);
                }
            }

            AppEvent::ToolResult {
                call_id,
                name,
                result,
            } => {
                // Only send response if the call wasn't cancelled
                if self.pending_tool_calls.remove(&call_id) {
                    log::info!("[Tool] {} completed (id={})", name, call_id);
                    self.broadcast_dashboard("voice:tool_result", json!({
                        "chatId": call_id,
                        "toolName": name,
                        "timestamp": now_ms(),
                    }));
                    let response = FunctionResponse {
                        id: call_id,
                        name,
                        response: result,
                    };
                    let guard = GEMINI_CLIENT.lock().unwrap();
                    if let Some(ref client) = *guard {
                        client.send_tool_response(vec![response]);
                    }
                } else {
                    log::info!("[Tool] {} result dropped (cancelled, id={})", name, call_id);
                }
            }

            AppEvent::GeminiError(err) => {
                log::error!("[Gemini] Error: {}", err);
                self.broadcast_dashboard("voice:close", json!({
                    "reason": format!("error: {}", err),
                    "timestamp": now_ms(),
                }));
                self.disconnect_gemini();
                self.request_redraw_all();
            }

            AppEvent::GeminiClosed(reason) => {
                log::warn!("[Gemini] Session closed: {}", reason);
                self.broadcast_dashboard("voice:close", json!({
                    "reason": reason,
                    "timestamp": now_ms(),
                }));
                self.disconnect_gemini();
                self.request_redraw_all();
            }
        }
    }
}
