use std::process::Command;
use std::error::Error;

pub struct TextInjector {
    backend: TextInjectionBackend,
}

#[derive(Debug, Clone)]
pub enum TextInjectionBackend {
    X11,
    Wayland,
}

impl TextInjector {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let backend = Self::detect_display_server()?;
        Ok(TextInjector { backend })
    }

    fn detect_display_server() -> Result<TextInjectionBackend, Box<dyn Error>> {
        // Check for Wayland first
        if std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("XDG_SESSION_TYPE").map_or(false, |s| s == "wayland") {
            Ok(TextInjectionBackend::Wayland)
        }
        // Check for X11
        else if std::env::var("DISPLAY").is_ok() {
            Ok(TextInjectionBackend::X11)
        }
        else {
            Err("No supported display server detected".into())
        }
    }

    pub fn inject_text(&self, text: &str) -> Result<(), Box<dyn Error>> {
        match self.backend {
            TextInjectionBackend::X11 => self.inject_text_x11(text),
            TextInjectionBackend::Wayland => self.inject_text_wayland(text),
        }
    }

    fn inject_text_x11(&self, text: &str) -> Result<(), Box<dyn Error>> {
        // Use xdotool to type the text
        let output = Command::new("xdotool")
            .args(&["type", "--clearmodifiers", text])
            .output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("xdotool failed: {}", error_msg).into());
        }

        Ok(())
    }

    fn inject_text_wayland(&self, text: &str) -> Result<(), Box<dyn Error>> {
        // For Wayland, we'll use the clipboard approach as a fallback
        // since direct text injection is more restricted
        
        // First, copy text to clipboard using wl-clipboard
        let mut copy_cmd = Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stdin) = copy_cmd.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())?;
        }

        let copy_result = copy_cmd.wait()?;
        if !copy_result.success() {
            return Err("Failed to copy text to clipboard".into());
        }

        // Then simulate Ctrl+V to paste
        // Note: This is a workaround and may not work in all applications
        // A more robust solution would require compositor-specific protocols
        let paste_output = Command::new("wtype")
            .args(&["-M", "ctrl", "-P", "v", "-m", "ctrl"])
            .output();

        match paste_output {
            Ok(output) => {
                if !output.status.success() {
                    // Fall back to just notifying the user to paste manually
                    eprintln!("Text copied to clipboard. Please paste manually with Ctrl+V");
                }
            }
            Err(_) => {
                // wtype not available, just notify user
                eprintln!("Text copied to clipboard. Please paste manually with Ctrl+V");
            }
        }

        Ok(())
    }

    pub fn get_active_window_info(&self) -> Result<WindowInfo, Box<dyn Error>> {
        match self.backend {
            TextInjectionBackend::X11 => self.get_active_window_info_x11(),
            TextInjectionBackend::Wayland => self.get_active_window_info_wayland(),
        }
    }

    fn get_active_window_info_x11(&self) -> Result<WindowInfo, Box<dyn Error>> {
        // Get active window ID
        let window_id_output = Command::new("xdotool")
            .args(&["getactivewindow"])
            .output()?;

        if !window_id_output.status.success() {
            return Err("Failed to get active window".into());
        }

        let window_id = String::from_utf8_lossy(&window_id_output.stdout).trim().to_string();

        // Get window name
        let window_name_output = Command::new("xdotool")
            .args(&["getwindowname", &window_id])
            .output()?;

        let window_name = if window_name_output.status.success() {
            String::from_utf8_lossy(&window_name_output.stdout).trim().to_string()
        } else {
            "Unknown".to_string()
        };

        // Get window class
        let window_class_output = Command::new("xprop")
            .args(&["-id", &window_id, "WM_CLASS"])
            .output()?;

        let window_class = if window_class_output.status.success() {
            String::from_utf8_lossy(&window_class_output.stdout).trim().to_string()
        } else {
            "Unknown".to_string()
        };

        Ok(WindowInfo {
            id: window_id,
            title: window_name,
            class: window_class,
        })
    }

    fn get_active_window_info_wayland(&self) -> Result<WindowInfo, Box<dyn Error>> {
        // Wayland doesn't provide easy access to window information
        // This would require compositor-specific solutions
        Ok(WindowInfo {
            id: "unknown".to_string(),
            title: "Unknown (Wayland)".to_string(),
            class: "Unknown (Wayland)".to_string(),
        })
    }

    pub fn backend(&self) -> &TextInjectionBackend {
        &self.backend
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub class: String,
}

impl Default for TextInjector {
    fn default() -> Self {
        Self::new().expect("Failed to create TextInjector")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_injector_creation() {
        // This test will pass if we can detect a display server
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            assert!(TextInjector::new().is_ok());
        }
    }

    #[test]
    fn test_display_server_detection() {
        // Mock environment variables for testing
        if std::env::var("DISPLAY").is_ok() {
            assert!(matches!(
                TextInjector::detect_display_server().unwrap(),
                TextInjectionBackend::X11
            ));
        }
    }
}