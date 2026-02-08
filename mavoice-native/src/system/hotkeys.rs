use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};

pub struct HotkeyManager {
    #[allow(dead_code)]
    manager: GlobalHotKeyManager,
    toggle_hotkey_id: u32,
}

impl HotkeyManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = GlobalHotKeyManager::new()?;

        // Ctrl+Shift+Comma — global toggle hotkey
        let toggle = HotKey::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::Comma,
        );
        let toggle_id = toggle.id();
        manager.register(toggle)?;

        log::info!("Global hotkey registered: Ctrl+Shift+Comma (id={})", toggle_id);

        Ok(Self {
            manager,
            toggle_hotkey_id: toggle_id,
        })
    }

    /// Check for pending hotkey events. Returns true if toggle was pressed.
    /// Drains all events but only fires on Pressed (ignores Released).
    pub fn poll_toggle(&self) -> bool {
        let mut fired = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.toggle_hotkey_id
                && event.state == global_hotkey::HotKeyState::Pressed
            {
                fired = true;
            }
        }
        fired
    }

    pub fn _manager(&self) -> &GlobalHotKeyManager {
        &self.manager
    }
}
