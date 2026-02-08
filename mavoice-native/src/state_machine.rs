/// Overlay visual state — drives shader uniforms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayState {
    Idle,
    Recording,
    Processing,
    Done,
}

/// Color palette — exact RGB values from FloatingOverlay.tsx
const COLOR_IDLE: [f32; 3] = [0.0, 0.0, 0.0];
const COLOR_RECORDING: [f32; 3] = [1.0, 0.51, 0.24]; // warm amber
const COLOR_PROCESSING: [f32; 3] = [0.9, 0.76, 0.31]; // golden
const COLOR_DONE: [f32; 3] = [0.31, 0.86, 0.51]; // emerald

impl OverlayState {
    pub fn target_color(&self) -> [f32; 3] {
        match self {
            OverlayState::Idle => COLOR_IDLE,
            OverlayState::Recording => COLOR_RECORDING,
            OverlayState::Processing => COLOR_PROCESSING,
            OverlayState::Done => COLOR_DONE,
        }
    }
}

/// Smoothed visual state interpolated per-frame
pub struct VisualState {
    pub state: OverlayState,
    pub levels: [f32; 4],
    pub intensity: f32,
    pub color: [f32; 3],
    pub mode: f32, // 0.0 = waveform, 1.0 = processing
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

    /// Per-frame update — returns true if a redraw is needed
    pub fn update(&mut self, raw_levels: [f32; 4]) -> bool {
        let is_active = self.state == OverlayState::Recording;

        // Smooth audio levels (lerp 0.18 when active, 0.3 when decaying)
        let level_speed = if is_active { 0.18 } else { 0.3 };
        let target_levels = if is_active { raw_levels } else { [0.0; 4] };
        for i in 0..4 {
            self.levels[i] += (target_levels[i] - self.levels[i]) * level_speed;
            if self.levels[i] < 0.003 {
                self.levels[i] = 0.0;
            }
        }

        // Smooth intensity (0.15 ramping up, 0.1 decaying)
        let target_int = if self.state == OverlayState::Idle {
            0.0
        } else {
            1.0
        };
        let int_speed = if self.state == OverlayState::Idle {
            0.1
        } else {
            0.15
        };
        self.intensity += (target_int - self.intensity) * int_speed;
        if self.intensity < 0.003 {
            self.intensity = 0.0;
        }

        // Smooth color (lerp 0.08)
        let tc = self.state.target_color();
        for i in 0..3 {
            self.color[i] += (tc[i] - self.color[i]) * 0.08;
        }

        // Mode: 0.0 for waveform, 1.0 for processing orbs
        let target_mode = if self.state == OverlayState::Processing {
            1.0
        } else {
            0.0
        };
        self.mode += (target_mode - self.mode) * 0.15;

        // Handle Done state: 1.5s fade, auto-idle at 2s
        if self.state == OverlayState::Done {
            if let Some(start) = self.done_start {
                let elapsed = start.elapsed().as_secs_f32();
                if elapsed >= 2.0 {
                    self.set_state(OverlayState::Idle);
                    self.done_start = None;
                }
            }
        }

        // Redraw needed if anything is visible
        self.intensity > 0.001
            || self.levels.iter().any(|&l| l > 0.001)
            || self.state != OverlayState::Idle
    }

    /// Get effective levels for the shader (with recording floor + done fade)
    pub fn effective_levels(&self) -> [f32; 4] {
        match self.state {
            OverlayState::Recording => {
                // Floor at 0.12 for base waveform movement
                self.levels.map(|l| l.max(0.12))
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

    /// Get effective intensity for the shader
    pub fn effective_intensity(&self) -> f32 {
        match self.state {
            OverlayState::Recording => self.intensity * 0.85,
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
}
