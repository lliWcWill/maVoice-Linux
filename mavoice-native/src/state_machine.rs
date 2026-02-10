/// Overlay visual state — drives shader uniforms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayState {
    Idle,
    Recording,   // Mode A: buffering for Groq
    Processing,  // Mode A: waiting for Groq API
    Done,        // Mode A: transcription complete
    Listening,   // Mode B: streaming to Gemini, user speaking
    AISpeaking,  // Mode B: Gemini responding with audio
}

/// Color palette
const COLOR_IDLE: [f32; 3] = [0.0, 0.0, 0.0];
const COLOR_RECORDING: [f32; 3] = [1.0, 0.51, 0.24];    // warm amber
const COLOR_PROCESSING: [f32; 3] = [0.9, 0.76, 0.31];   // golden
const COLOR_DONE: [f32; 3] = [0.31, 0.86, 0.51];        // emerald
const COLOR_LISTENING: [f32; 3] = [0.024, 0.714, 0.831]; // cyan #06B6D4
const COLOR_AI_SPEAKING: [f32; 3] = [0.337, 0.467, 0.969]; // soft blue #5677F7

impl OverlayState {
    /// User waveform color (bottom line)
    pub fn user_color(&self) -> [f32; 3] {
        match self {
            OverlayState::Idle => COLOR_IDLE,
            OverlayState::Recording => COLOR_RECORDING,
            OverlayState::Processing => COLOR_PROCESSING,
            OverlayState::Done => COLOR_DONE,
            OverlayState::Listening => COLOR_LISTENING,
            OverlayState::AISpeaking => COLOR_LISTENING, // stays cyan when AI responds
        }
    }
}

/// Smoothed visual state interpolated per-frame.
/// Tracks user waveform (bottom) and AI bubble (top) independently.
pub struct VisualState {
    pub state: OverlayState,
    // User channel (bottom waveform)
    pub levels: [f32; 4],
    pub intensity: f32,
    pub color: [f32; 3],
    pub mode: f32, // 0.0 = waveform, 1.0 = processing
    // AI channel (top bubble)
    pub ai_levels: [f32; 4],
    pub ai_intensity: f32,
    pub ai_color: [f32; 3],
    // Timing
    pub done_start: Option<std::time::Instant>,
}

impl VisualState {
    pub fn new() -> Self {
        Self {
            state: OverlayState::Idle,
            levels: [0.0; 4],
            intensity: 0.0,
            color: COLOR_IDLE,
            mode: 0.0,
            ai_levels: [0.0; 4],
            ai_intensity: 0.0,
            ai_color: COLOR_AI_SPEAKING,
            done_start: None,
        }
    }

    /// Transition to a new state
    pub fn set_state(&mut self, new_state: OverlayState) {
        if self.state == new_state {
            return;
        }
        log::debug!("State: {:?} -> {:?}", self.state, new_state);
        self.state = new_state;
        if new_state == OverlayState::Done {
            self.done_start = Some(std::time::Instant::now());
        }
    }

    /// Per-frame update — returns true if a redraw is needed.
    /// `raw_levels`: mic input levels. `output_levels`: AI audio output levels.
    pub fn update(&mut self, raw_levels: [f32; 4]) -> bool {
        self.update_with_output(raw_levels, [0.0; 4])
    }

    /// Per-frame update with both input and output audio levels.
    pub fn update_with_output(&mut self, raw_levels: [f32; 4], output_levels: [f32; 4]) -> bool {
        // ── User channel (bottom waveform) ──
        let user_active = matches!(
            self.state,
            OverlayState::Recording | OverlayState::Listening
        );

        // Smooth user audio levels — fast attack, moderate decay
        let user_speed = if user_active { 0.35 } else { 0.2 };
        let user_target = if user_active { raw_levels } else { [0.0; 4] };
        for i in 0..4 {
            self.levels[i] += (user_target[i] - self.levels[i]) * user_speed;
            if self.levels[i] < 0.003 {
                self.levels[i] = 0.0;
            }
        }

        // Smooth user intensity
        let user_int_target = match self.state {
            OverlayState::Idle => 0.0,
            OverlayState::AISpeaking => 0.15, // dim but not gone when AI speaks
            _ => 1.0,
        };
        let user_int_speed = if self.state == OverlayState::Idle { 0.1 } else { 0.15 };
        self.intensity += (user_int_target - self.intensity) * user_int_speed;
        if self.intensity < 0.003 {
            self.intensity = 0.0;
        }

        // Smooth user color
        let tc = self.state.user_color();
        for i in 0..3 {
            self.color[i] += (tc[i] - self.color[i]) * 0.08;
        }

        // Mode: 0.0 for waveform, 1.0 for processing orbs
        let target_mode = if self.state == OverlayState::Processing { 1.0 } else { 0.0 };
        self.mode += (target_mode - self.mode) * 0.15;

        // ── AI channel (top bubble) ──
        let ai_active = self.state == OverlayState::AISpeaking;

        // Smooth AI audio levels
        let ai_speed = if ai_active { 0.18 } else { 0.3 };
        let ai_target = if ai_active { output_levels } else { [0.0; 4] };
        for i in 0..4 {
            self.ai_levels[i] += (ai_target[i] - self.ai_levels[i]) * ai_speed;
            if self.ai_levels[i] < 0.003 {
                self.ai_levels[i] = 0.0;
            }
        }

        // Smooth AI intensity
        let ai_int_target = match self.state {
            OverlayState::AISpeaking => 1.0,
            OverlayState::Listening => 0.25, // dim presence while user speaks
            _ => 0.0,
        };
        let ai_int_speed = if ai_active { 0.15 } else { 0.1 };
        self.ai_intensity += (ai_int_target - self.ai_intensity) * ai_int_speed;
        if self.ai_intensity < 0.003 {
            self.ai_intensity = 0.0;
        }

        // Smooth AI color (stays blue)
        for i in 0..3 {
            self.ai_color[i] += (COLOR_AI_SPEAKING[i] - self.ai_color[i]) * 0.08;
        }

        // ── Done state auto-reset ──
        if self.state == OverlayState::Done {
            if let Some(start) = self.done_start {
                if start.elapsed().as_secs_f32() >= 2.0 {
                    self.set_state(OverlayState::Idle);
                    self.done_start = None;
                }
            }
        }

        // Also handle mode-switch flash (Listening/Recording with done_start)
        if matches!(self.state, OverlayState::Listening | OverlayState::Recording) {
            if let Some(start) = self.done_start {
                if start.elapsed().as_secs_f32() >= 1.0 {
                    self.set_state(OverlayState::Idle);
                    self.done_start = None;
                }
            }
        }

        // Redraw needed if anything is visible
        self.intensity > 0.001
            || self.ai_intensity > 0.001
            || self.levels.iter().any(|&l| l > 0.001)
            || self.ai_levels.iter().any(|&l| l > 0.001)
            || self.state != OverlayState::Idle
    }

    /// Get effective user levels for the shader
    pub fn effective_levels(&self) -> [f32; 4] {
        match self.state {
            OverlayState::Recording | OverlayState::Listening => {
                self.levels.map(|l| l.max(0.18))
            }
            OverlayState::AISpeaking => {
                // Dim user waveform to subtle breathing
                self.levels.map(|l| l.max(0.05))
            }
            OverlayState::Processing => [0.0; 4],
            OverlayState::Done => {
                if let Some(start) = self.done_start {
                    let fade = (1.0 - start.elapsed().as_secs_f32() / 1.5).max(0.0);
                    [0.4 * fade, 0.6 * fade, 0.5 * fade, 0.3 * fade]
                } else {
                    [0.0; 4]
                }
            }
            OverlayState::Idle => [0.0; 4],
        }
    }

    /// Get effective AI levels for the shader
    pub fn effective_ai_levels(&self) -> [f32; 4] {
        match self.state {
            OverlayState::AISpeaking => {
                self.ai_levels.map(|l| l.max(0.15))
            }
            OverlayState::Listening => {
                // Subtle presence while user speaks
                [0.03; 4]
            }
            _ => [0.0; 4],
        }
    }

    /// Get effective user intensity for the shader
    pub fn effective_intensity(&self) -> f32 {
        match self.state {
            OverlayState::Recording | OverlayState::Listening => self.intensity * 0.85,
            OverlayState::AISpeaking => self.intensity * 0.3, // dim user while AI speaks
            OverlayState::Done => {
                if let Some(start) = self.done_start {
                    let fade = (1.0 - start.elapsed().as_secs_f32() / 1.5).max(0.0);
                    fade * 0.8
                } else {
                    0.0
                }
            }
            _ => self.intensity,
        }
    }

    /// Get effective AI intensity for the shader
    pub fn effective_ai_intensity(&self) -> f32 {
        match self.state {
            OverlayState::AISpeaking => self.ai_intensity * 0.9,
            OverlayState::Listening => self.ai_intensity * 0.2, // subtle presence
            _ => self.ai_intensity,
        }
    }
}
