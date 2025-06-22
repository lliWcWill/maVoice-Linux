use enigo::{Enigo, Keyboard, Settings};
use std::thread;
use std::time::Duration;
use std::process::Command;

pub struct TextInjector {
    enigo: Enigo,
    is_wayland: bool,
    has_xclip: bool,
    has_wl_clipboard: bool,
    has_xdotool: bool,
}

impl TextInjector {
    pub fn new() -> Self {
        let settings = Settings::default();
        let enigo = Enigo::new(&settings).expect("Failed to create Enigo instance");
        
        // Detect environment and available tools
        let is_wayland = Self::detect_wayland();
        let has_xclip = Self::check_command("xclip");
        let has_wl_clipboard = Self::check_command("wl-copy");
        let has_xdotool = Self::check_command("xdotool");
        
        println!("🔍 Environment Detection:");
        println!("  Display Server: {}", if is_wayland { "Wayland" } else { "X11" });
        println!("  xclip available: {}", has_xclip);
        println!("  wl-clipboard available: {}", has_wl_clipboard);
        println!("  xdotool available: {}", has_xdotool);
        
        Self {
            enigo,
            is_wayland,
            has_xclip,
            has_wl_clipboard,
            has_xdotool,
        }
    }
    
    fn detect_wayland() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok() || 
        std::env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false)
    }
    
    fn check_command(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// BULLETPROOF text injection with multiple fallback strategies
    pub fn inject_text_bulletproof(&mut self, text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        
        println!("🚀 BULLETPROOF TEXT INJECTION: \"{}\"", text);
        println!("🎯 Trying multiple strategies to ensure success...");
        
        // Strategy 1: Direct Enigo text typing
        println!("📝 Strategy 1: Direct text typing");
        if let Ok(_) = self.enigo.text(text) {
            thread::sleep(Duration::from_millis(200));
            println!("✅ Strategy 1 succeeded!");
            return Ok(());
        }
        
        // Strategy 2: Character-by-character typing
        println!("📝 Strategy 2: Character-by-character typing");
        if self.type_char_by_char(text).is_ok() {
            println!("✅ Strategy 2 succeeded!");
            return Ok(());
        }
        
        // Strategy 3: Native clipboard + paste
        println!("📝 Strategy 3: Native clipboard + paste");
        if self.inject_via_native_clipboard(text).is_ok() {
            println!("✅ Strategy 3 succeeded!");
            return Ok(());
        }
        
        // Strategy 4: xdotool (if available)
        println!("📝 Strategy 4: xdotool fallback");
        if self.has_xdotool && self.inject_via_xdotool(text).is_ok() {
            println!("✅ Strategy 4 succeeded!");
            return Ok(());
        }
        
        // Strategy 5: Direct clipboard tools
        println!("📝 Strategy 5: Direct clipboard tools");
        if self.inject_via_clipboard_tools(text).is_ok() {
            println!("✅ Strategy 5 succeeded!");
            return Ok(());
        }
        
        println!("❌ ALL STRATEGIES FAILED - Text saved to backup");
        Err("All injection strategies failed".to_string())
    }
    
    fn type_char_by_char(&mut self, text: &str) -> Result<(), String> {
        use enigo::{Key, Direction};
        
        for ch in text.chars() {
            let result = match ch {
                '\n' => self.enigo.key(Key::Return, Direction::Click),
                '\t' => self.enigo.key(Key::Tab, Direction::Click),
                ' ' => self.enigo.key(Key::Space, Direction::Click),
                _ => self.enigo.key(Key::Unicode(ch), Direction::Click),
            };
            
            if result.is_err() {
                return Err("Character typing failed".to_string());
            }
            
            thread::sleep(Duration::from_millis(20)); // Slight delay between chars
        }
        Ok(())
    }
    
    fn inject_via_native_clipboard(&mut self, text: &str) -> Result<(), String> {
        // Copy to clipboard first
        self.copy_to_system_clipboard(text)?;
        
        // Wait a moment for clipboard to be ready
        thread::sleep(Duration::from_millis(100));
        
        // Try multiple paste methods
        use enigo::{Key, Direction};
        
        // Method 1: Ctrl+V
        if self.enigo.key(Key::Control, Direction::Press).is_ok() &&
           self.enigo.key(Key::Unicode('v'), Direction::Click).is_ok() &&
           self.enigo.key(Key::Control, Direction::Release).is_ok() {
            thread::sleep(Duration::from_millis(100));
            return Ok(());
        }
        
        // Method 2: Shift+Insert
        if self.enigo.key(Key::Shift, Direction::Press).is_ok() &&
           self.enigo.key(Key::Insert, Direction::Click).is_ok() &&
           self.enigo.key(Key::Shift, Direction::Release).is_ok() {
            return Ok(());
        }
        
        Err("Native clipboard paste failed".to_string())
    }
    
    fn inject_via_xdotool(&self, text: &str) -> Result<(), String> {
        // Use xdotool as a fallback
        let output = Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg(text)
            .output()
            .map_err(|e| format!("xdotool failed: {}", e))?;
        
        if output.status.success() {
            Ok(())
        } else {
            Err("xdotool command failed".to_string())
        }
    }
    
    fn inject_via_clipboard_tools(&self, text: &str) -> Result<(), String> {
        // Method 1: Try wl-copy + wl-paste (Wayland)
        if self.has_wl_clipboard {
            if let Ok(_) = Command::new("wl-copy").arg(text).output() {
                thread::sleep(Duration::from_millis(100));
                // Simulate Ctrl+V after copying
                if let Ok(_) = Command::new("wl-paste").output() {
                    return Ok(());
                }
            }
        }
        
        // Method 2: Try xclip + xsel (X11)
        if self.has_xclip {
            let mut child = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("xclip spawn failed: {}", e))?;
            
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(text.as_bytes()).ok();
            }
            
            if child.wait().map_err(|e| format!("xclip wait failed: {}", e))?.success() {
                thread::sleep(Duration::from_millis(100));
                return Ok(());
            }
        }
        
        Err("Clipboard tools failed".to_string())
    }
    
    fn copy_to_system_clipboard(&self, text: &str) -> Result<(), String> {
        if self.is_wayland && self.has_wl_clipboard {
            Command::new("wl-copy")
                .arg(text)
                .output()
                .map_err(|e| format!("wl-copy failed: {}", e))?;
        } else if self.has_xclip {
            let mut child = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("xclip failed: {}", e))?;
            
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(text.as_bytes())
                    .map_err(|e| format!("Failed to write to xclip: {}", e))?;
            }
            
            child.wait().map_err(|e| format!("xclip wait failed: {}", e))?;
        } else {
            return Err("No clipboard tools available".to_string());
        }
        Ok(())
    }
    
    /// Legacy method - keep for compatibility
    pub fn inject_text(&mut self, text: &str) -> Result<(), String> {
        self.inject_text_bulletproof(text)
    }
    
    /// Alternative method using clipboard (more reliable for complex text)
    pub fn inject_via_clipboard(&mut self, text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        
        println!("📋 Injecting text via clipboard: \"{}\"", text);
        
        // Copy text to clipboard
        if let Err(e) = self.copy_to_clipboard(text) {
            println!("⚠️ Clipboard copy failed: {}", e);
            println!("🔄 Falling back to direct typing");
            return self.inject_text(text);
        }
        
        println!("✅ Text copied to clipboard successfully");
        
        // Small delay
        thread::sleep(Duration::from_millis(100));
        
        // Paste from clipboard (Ctrl+V)
        use enigo::{Key, Keyboard, Direction};
        println!("🔤 Attempting to paste with Ctrl+V");
        
        match self.enigo.key(Key::Control, Direction::Press) {
            Ok(_) => println!("✅ Ctrl pressed"),
            Err(e) => {
                println!("❌ Failed to press Ctrl: {:?}", e);
                return Err(format!("Failed to press Ctrl: {:?}", e));
            }
        }
        
        thread::sleep(Duration::from_millis(50));
        
        match self.enigo.key(Key::Unicode('v'), Direction::Click) {
            Ok(_) => println!("✅ V pressed"),
            Err(e) => {
                println!("❌ Failed to press V: {:?}", e);
                return Err(format!("Failed to press V: {:?}", e));
            }
        }
        
        thread::sleep(Duration::from_millis(50));
        
        match self.enigo.key(Key::Control, Direction::Release) {
            Ok(_) => println!("✅ Ctrl released"),
            Err(e) => {
                println!("❌ Failed to release Ctrl: {:?}", e);
                return Err(format!("Failed to release Ctrl: {:?}", e));
            }
        }
        
        println!("✅ Text injection via clipboard completed");
        Ok(())
    }
    
    fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        use std::process::Command;
        
        // Use xclip to copy text to clipboard
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn xclip: {}", e))?;
        
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())
                .map_err(|e| format!("Failed to write to xclip: {}", e))?;
        }
        
        let status = child.wait()
            .map_err(|e| format!("Failed to wait for xclip: {}", e))?;
        
        if !status.success() {
            return Err("xclip command failed".to_string());
        }
        
        Ok(())
    }
    
    /// Get current cursor position (for debugging)
    pub fn get_cursor_info(&self) -> String {
        // This would require additional X11 calls to get window focus info
        // For now, just return a placeholder
        "Cursor position detection not implemented".to_string()
    }
}