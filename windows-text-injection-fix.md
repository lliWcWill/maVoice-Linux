# Quick Windows Fix for Text Injection

The error "No supported display server detected" happens because the text injection code looks for Linux display servers (X11/Wayland) but you're on Windows.

## Quick Fix 1: Disable Text Injection (Clipboard Only)

Edit `src-tauri/src/system/text_inject.rs` and modify the `inject_text` function:

```rust
#[cfg(target_os = "windows")]
pub async fn inject_text(text: &str) -> Result<(), String> {
    println!("💾 Text copied to clipboard (Windows): {}", &text[..text.len().min(50)]);
    Ok(()) // Just skip injection on Windows for now
}

// Keep the existing Linux implementation
#[cfg(not(target_os = "windows"))]
pub async fn inject_text(text: &str) -> Result<(), String> {
    // ... existing Linux code ...
}
```

## Quick Fix 2: Use Windows SendInput API

Or add proper Windows text injection using the Windows API:

```rust
#[cfg(target_os = "windows")]
use winapi::um::winuser::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_UNICODE};

#[cfg(target_os = "windows")]
pub async fn inject_text(text: &str) -> Result<(), String> {
    // Copy to clipboard first
    use clipboard_win::{Clipboard, formats};
    Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?
        .set_string(text)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;
    
    // Simulate Ctrl+V
    unsafe {
        // Send Ctrl down
        let mut input = INPUT {
            type_: INPUT_KEYBOARD,
            u: std::mem::zeroed(),
        };
        *input.u.ki_mut() = KEYBDINPUT {
            wVk: 0x11, // VK_CONTROL
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };
        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        
        // Send V down
        input.u.ki_mut().wVk = 0x56; // VK_V
        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        
        // Send V up
        input.u.ki_mut().dwFlags = KEYEVENTF_KEYUP;
        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        
        // Send Ctrl up
        input.u.ki_mut().wVk = 0x11;
        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
    }
    
    Ok(())
}
```

Then add to `Cargo.toml`:
```toml
[dependencies.winapi]
version = "0.3"
features = ["winuser", "processthreadsapi"]

[dependencies.clipboard-win]
version = "4.5"
```

For now, I recommend **Quick Fix 1** - just disable injection and use clipboard only. The text will still be copied automatically!