#![allow(dead_code)]
use std::error::Error;
use std::process::Command;

pub struct TextInjector {
    backend: TextInjectionBackend,
}

#[derive(Debug, Clone)]
pub enum TextInjectionBackend {
    X11,
    Wayland,
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub class: String,
}

impl TextInjector {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let backend = Self::detect_display_server()?;
        log::info!("Text injector using {:?} backend", backend);
        Ok(TextInjector { backend })
    }

    fn detect_display_server() -> Result<TextInjectionBackend, Box<dyn Error>> {
        if std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("XDG_SESSION_TYPE").map_or(false, |s| s == "wayland")
        {
            Ok(TextInjectionBackend::Wayland)
        } else if std::env::var("DISPLAY").is_ok() {
            Ok(TextInjectionBackend::X11)
        } else {
            Err("No supported display server detected".into())
        }
    }

    pub fn inject_text(&self, text: &str) -> Result<(), Box<dyn Error>> {
        match self.backend {
            TextInjectionBackend::X11 => self.inject_text_x11(text),
            TextInjectionBackend::Wayland => self.inject_text_wayland(text),
        }
    }

    /// Get the currently focused window ID on X11 (before overlay steals focus)
    pub fn get_active_window_id(&self) -> Option<String> {
        let output = Command::new("xdotool")
            .args(["getactivewindow"])
            .output()
            .ok()?;
        if output.status.success() {
            let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !id.is_empty() {
                return Some(id);
            }
        }
        None
    }

    fn inject_text_x11(&self, text: &str) -> Result<(), Box<dyn Error>> {
        self.inject_text_x11_to(text, None)
    }

    /// Inject text on X11 by copying to clipboard and pasting into the target window.
    /// If `target_window_id` is provided, refocuses that window first.
    pub fn inject_text_to(&self, text: &str, target_window_id: Option<&str>) -> Result<(), Box<dyn Error>> {
        match self.backend {
            TextInjectionBackend::X11 => self.inject_text_x11_to(text, target_window_id),
            TextInjectionBackend::Wayland => self.inject_text_wayland(text),
        }
    }

    fn inject_text_x11_to(&self, text: &str, target_window_id: Option<&str>) -> Result<(), Box<dyn Error>> {
        // Step 1: Copy text to clipboard via xclip
        let mut xclip = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stdin) = xclip.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())?;
        }

        let clip_result = xclip.wait()?;
        if !clip_result.success() {
            return Err("xclip failed to copy text to clipboard".into());
        }
        log::info!("Text copied to clipboard ({} chars)", text.len());

        // Step 2: Refocus the target window (the one that was active before overlay)
        if let Some(win_id) = target_window_id {
            let focus_output = Command::new("xdotool")
                .args(["windowactivate", "--sync", win_id])
                .output()?;
            if !focus_output.status.success() {
                log::warn!("Failed to refocus window {}, trying paste anyway", win_id);
            }
            // Brief pause to let the window manager complete the focus switch
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Step 3: Paste via Ctrl+V
        let paste_output = Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+v"])
            .output()?;

        if !paste_output.status.success() {
            let error_msg = String::from_utf8_lossy(&paste_output.stderr);
            log::warn!("xdotool paste failed: {}. Text is in clipboard — paste manually with Ctrl+V", error_msg);
        }

        Ok(())
    }

    fn inject_text_wayland(&self, text: &str) -> Result<(), Box<dyn Error>> {
        // Copy to clipboard via wl-copy
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

        // Simulate Ctrl+V via wtype
        let paste_output = Command::new("wtype")
            .args(["-M", "ctrl", "-P", "v", "-m", "ctrl"])
            .output();

        match paste_output {
            Ok(output) => {
                if !output.status.success() {
                    log::warn!("Text copied to clipboard. Please paste manually with Ctrl+V");
                }
            }
            Err(_) => {
                log::warn!("wtype not available. Text copied to clipboard — paste with Ctrl+V");
            }
        }
        Ok(())
    }

    pub fn get_active_window_info(&self) -> Result<WindowInfo, Box<dyn Error>> {
        match self.backend {
            TextInjectionBackend::X11 => self.get_active_window_info_x11(),
            TextInjectionBackend::Wayland => Ok(WindowInfo {
                id: "unknown".to_string(),
                title: "Unknown (Wayland)".to_string(),
                class: "Unknown (Wayland)".to_string(),
            }),
        }
    }

    fn get_active_window_info_x11(&self) -> Result<WindowInfo, Box<dyn Error>> {
        let window_id_output = Command::new("xdotool").args(["getactivewindow"]).output()?;

        if !window_id_output.status.success() {
            return Err("Failed to get active window".into());
        }

        let window_id = String::from_utf8_lossy(&window_id_output.stdout)
            .trim()
            .to_string();

        let window_name_output = Command::new("xdotool")
            .args(["getwindowname", &window_id])
            .output()?;

        let window_name = if window_name_output.status.success() {
            String::from_utf8_lossy(&window_name_output.stdout)
                .trim()
                .to_string()
        } else {
            "Unknown".to_string()
        };

        let window_class_output = Command::new("xprop")
            .args(["-id", &window_id, "WM_CLASS"])
            .output()?;

        let window_class = if window_class_output.status.success() {
            String::from_utf8_lossy(&window_class_output.stdout)
                .trim()
                .to_string()
        } else {
            "Unknown".to_string()
        };

        Ok(WindowInfo {
            id: window_id,
            title: window_name,
            class: window_class,
        })
    }

    pub fn backend(&self) -> &TextInjectionBackend {
        &self.backend
    }
}
