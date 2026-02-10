use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};

/// Result of polling hotkey events.
pub struct HotkeyPoll {
    pub toggle_fired: bool,
    pub mode_switch_fired: bool,
}

pub struct HotkeyManager {
    #[allow(dead_code)]
    manager: GlobalHotKeyManager,
    toggle_hotkey_id: u32,
    mode_switch_hotkey_id: u32,
}

impl HotkeyManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = GlobalHotKeyManager::new()?;

        // Ctrl+Shift+Comma — toggle recording
        let toggle = HotKey::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::Comma,
        );
        let toggle_id = toggle.id();
        manager.register(toggle)?;

        // Ctrl+Shift+Period — switch voice mode (Groq ↔ Gemini)
        let mode_switch = HotKey::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::Period,
        );
        let mode_switch_id = mode_switch.id();
        manager.register(mode_switch)?;

        log::info!(
            "Global hotkeys: Ctrl+Shift+Comma (toggle={}), Ctrl+Shift+Period (mode={})",
            toggle_id,
            mode_switch_id
        );

        Ok(Self {
            manager,
            toggle_hotkey_id: toggle_id,
            mode_switch_hotkey_id: mode_switch_id,
        })
    }

    /// Check for pending hotkey events. Returns true if toggle was pressed.
    /// Drains all events but only fires on Pressed (ignores Released).
    pub fn poll_toggle(&self) -> bool {
        self.poll().toggle_fired
    }

    /// Check for all hotkey events. Returns which hotkeys were pressed.
    pub fn poll(&self) -> HotkeyPoll {
        let mut toggle_fired = false;
        let mut mode_switch_fired = false;

        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state != global_hotkey::HotKeyState::Pressed {
                continue;
            }
            if event.id == self.toggle_hotkey_id {
                toggle_fired = true;
            } else if event.id == self.mode_switch_hotkey_id {
                mode_switch_fired = true;
            }
        }

        HotkeyPoll {
            toggle_fired,
            mode_switch_fired,
        }
    }

    pub fn _manager(&self) -> &GlobalHotKeyManager {
        &self.manager
    }
}
