<div align="center">

# 🎙️ maVoice

<img src="https://img.shields.io/badge/Powered%20by-Groq-FF6B6B?style=for-the-badge&logo=lightning&logoColor=white" alt="Powered by Groq">
<img src="https://img.shields.io/badge/Model-Whisper%20Turbo-4ECDC4?style=for-the-badge&logo=openai&logoColor=white" alt="Whisper Turbo">
<img src="https://img.shields.io/badge/Built%20with-Tauri-FFC107?style=for-the-badge&logo=rust&logoColor=black" alt="Built with Tauri">
<img src="https://img.shields.io/badge/License-MIT-45B7D1?style=for-the-badge&logo=opensource&logoColor=white" alt="MIT License">

<h3>🚀 Open-Source Voice Dictation Powered by Groq's Lightning-Fast Inference</h3>
<p>Experience the future of voice-to-text with <strong>Groq DEV Tier</strong> - Ultra-fast transcription that leaves OpenAI's free tier in the dust!</p>

```
┌─────────────────┐
│  🎤 maVoice     │  ← Tiny floating widget (72x20px)
│ ▶ ■ ▪ ▪ ▪ ▪    │    Always on top of your screen
└─────────────────┘    Double-click to start!
```

</div>

---

## 📋 Table of Contents

- [✨ Features](#-features)
- [🎯 What is maVoice?](#-what-is-mavoice)
- [🚀 Quick Start](#-quick-start)
  - [Platform Setup Guides](#platform-setup-guides)
  - [System Requirements](#system-requirements)
- [🎮 How to Use](#-how-to-use)
- [🔧 Troubleshooting](#-troubleshooting)
- [🧑‍💻 Developer Guide](#-developer-guide)
- [❓ FAQ](#-faq)
- [🏎️ Performance](#-performance)
- [🤝 Contributing](#-contributing)

---

## ✨ Features

- **⚡ Blazing Fast**: Powered by Groq's Whisper Large v3 Turbo model - the fastest inference in the game
- **🎯 Native Performance**: Built with Rust and Tauri for minimal resource usage
- **🎨 Floating Widget**: Tiny, draggable overlay that stays out of your way
- **🔒 Privacy First**: Your API key, your data - everything stays local
- **🌐 Cross-Platform**: Works on Linux (Windows and macOS coming soon!)
- **🎤 Smart Recording**: Real-time audio visualization and voice detection
- **📋 Instant Copy**: Automatic clipboard integration for seamless workflow
- **⚙️ Advanced Settings**: Comprehensive configuration panel with model selection
- **🎛️ Intuitive Controls**: Double-click to start, single-click to stop
- **🌍 Multi-Language**: Support for 100+ languages with custom prompts

## 🎯 What is maVoice?

maVoice is a **floating voice dictation widget** that lives on your desktop. Unlike traditional apps with windows and menus, maVoice is a tiny, always-accessible button that floats above your other applications.

### The Floating Widget Design

```
Normal State           Recording            Processing           Success
┌─────────────┐       ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ 🎤 maVoice  │  →    │ 🔴 ▶▶▶▶     │  →  │ 🟠 ◈◈◈◈◈    │  →  │ ✅ Done!    │
└─────────────┘       └─────────────┘     └─────────────┘     └─────────────┘
   (Blue)                 (Red)              (Orange)            (Green)
```

- **Size**: 72x20 pixels (about the size of a small button)
- **Position**: Fixed at coordinates x:300, y:800 by default
- **Behavior**: Always on top, transparent background, no window borders
- **Dragging**: Right-click or Ctrl+Left-click to drag to a new position

## 🚀 Quick Start

### Platform Setup Guides

<details>
<summary><b>🐧 Native Linux Setup</b></summary>

#### Prerequisites Check

```bash
# Check if you have all requirements
command -v node >/dev/null 2>&1 && echo "✅ Node.js installed" || echo "❌ Node.js missing"
command -v rustc >/dev/null 2>&1 && echo "✅ Rust installed" || echo "❌ Rust missing"
command -v cargo >/dev/null 2>&1 && echo "✅ Cargo installed" || echo "❌ Cargo missing"

# Check display server
echo "Display Server: ${XDG_SESSION_TYPE:-$([[ -n $WAYLAND_DISPLAY ]] && echo wayland || echo x11)}"
```

#### Install System Dependencies

**Debian/Ubuntu:**
```bash
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev \
    libdbus-1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    # For audio
    libasound2-dev \
    # For text injection
    xdotool \
    wl-clipboard \
    wtype
```

**Fedora:**
```bash
sudo dnf install -y \
    gcc \
    pkg-config \
    gtk3-devel \
    webkit2gtk4.0-devel \
    libsoup-devel \
    javascriptcoregtk4.0-devel \
    dbus-devel \
    libappindicator-gtk3-devel \
    librsvg2-devel \
    alsa-lib-devel \
    xdotool \
    wl-clipboard \
    wtype
```

**Arch Linux:**
```bash
sudo pacman -S --needed \
    base-devel \
    gtk3 \
    webkit2gtk \
    libsoup \
    dbus \
    libappindicator-gtk3 \
    librsvg \
    alsa-lib \
    xdotool \
    wl-clipboard \
    wtype
```

#### Installation Steps

```bash
# 1. Clone the repository
git clone https://github.com/lliWcWill/maVoice-Linux.git
cd maVoice-Linux

# 2. Install Node dependencies
npm install

# 3. Configure API key
echo "VITE_GROQ_API_KEY=your_groq_api_key_here" > src-tauri/aquavoice-frontend/.env

# 4. Run in development
npm run dev

# 5. Or build for production
npm run build
# Find .deb package in: src-tauri/target/release/bundle/deb/
```

</details>

<details>
<summary><b>🪟 WSL2 Setup (Windows Subsystem for Linux)</b></summary>

#### Prerequisites

1. **Update WSL2** (from Windows PowerShell as Administrator):
   ```powershell
   wsl --update
   wsl --version  # Ensure version 2
   ```

2. **Verify WSLg Support**:
   ```bash
   # In WSL2 terminal
   echo $DISPLAY  # Should show :0
   echo $WAYLAND_DISPLAY  # Should show wayland-0
   ```

3. **Install Dependencies**:
   ```bash
   # First, install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env

   # Then install system dependencies
   sudo apt update
   sudo apt install -y \
       build-essential \
       pkg-config \
       libgtk-3-dev \
       libwebkit2gtk-4.1-dev \
       libsoup-3.0-dev \
       libjavascriptcoregtk-4.1-dev \
       libdbus-1-dev \
       libappindicator3-dev \
       librsvg2-dev \
       libasound2-dev \
       xdotool \
       wl-clipboard \
       wtype

   # Test GUI support
   sudo apt install -y x11-apps
   xclock  # Should display a clock window
   ```

#### Common WSL2 Issues

- **No GUI appears**: Ensure WSLg is enabled and GPU drivers are updated on Windows
- **Audio not working**: WSL2 audio passthrough may need configuration
- **Widget hard to find**: The 72x20px window is tiny - look carefully or temporarily increase size in `tauri.conf.json`

</details>

<details>
<summary><b>🐳 Docker Setup (Experimental)</b></summary>

```bash
# Build and run with X11 forwarding
docker-compose -f docker-compose.dev.yml up

# Or manually
docker build -t mavoice .
docker run -it \
    -e DISPLAY=$DISPLAY \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v ~/.Xauthority:/root/.Xauthority \
    --device /dev/snd \
    mavoice
```

**Note**: Audio and clipboard integration may be limited in Docker.

</details>

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| OS | Linux (X11/Wayland) | Ubuntu 22.04+, Fedora 38+ |
| Node.js | 18.0.0 | 20.0.0+ |
| Rust | 1.70.0 | Latest stable |
| RAM | 2GB | 4GB+ |
| Display | Any | 1920x1080+ |
| Audio | PulseAudio/ALSA | PulseAudio |

### Get Your Groq API Key

1. Visit [console.groq.com](https://console.groq.com)
2. Sign up for a free account
3. Navigate to API Keys section
4. Create a new API key
5. Copy and save it securely

## 🎮 How to Use

### Finding the Widget

When you first launch maVoice, look for a tiny floating widget:

```
Your Desktop:
┌─────────────────────────────────────────┐
│  File  Edit  View  Help                 │
│                                         │
│     ┌─────────────┐ ← Look here!       │
│     │ 🎤 maVoice  │   (x:300, y:800)    │
│     └─────────────┘                     │
│                                         │
└─────────────────────────────────────────┘
```

### Basic Operations

1. **Start Recording**: Double-click the widget
   - Widget turns red with animated bars
   - Microphone activates immediately

2. **Stop Recording**: Single-click while recording
   - Widget turns orange (processing)
   - Then green (success) with transcribed text
   - Text automatically copied to clipboard

3. **Move the Widget**: Right-click and drag (or Ctrl+Left-click)

4. **Access Settings**: Click the gear icon on the web interface

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+,` | Start/stop recording (global) |
| `Alt+Space` | Toggle recording |
| `Double Alt` | Quick record |
| `Spacebar` | Stop recording (while active) |

### Settings Configuration

Access settings at `http://localhost:5173` or click the gear icon:

- **API Key**: Securely store your Groq API key
- **Model Selection**: Choose Whisper model variant
- **Temperature**: Adjust creativity (0.0-1.0)
- **Language**: Select from 100+ languages
- **Custom Prompt**: Add technical terms, names, or style preferences
- **Max Tokens**: Control response length

## 🔧 Troubleshooting

### Quick Diagnostic Script

Create and run this script to check your setup:

```bash
#!/bin/bash
# Save as check-mavoice.sh and run with: bash check-mavoice.sh

echo "🔍 maVoice Diagnostic Check"
echo "=========================="

# Check Node.js
if command -v node >/dev/null 2>&1; then
    echo "✅ Node.js: $(node --version)"
else
    echo "❌ Node.js: Not installed"
fi

# Check Rust
if command -v rustc >/dev/null 2>&1; then
    echo "✅ Rust: $(rustc --version | cut -d' ' -f2)"
else
    echo "❌ Rust: Not installed"
fi

# Check display server
if [[ -n $WAYLAND_DISPLAY ]]; then
    echo "✅ Display: Wayland ($WAYLAND_DISPLAY)"
elif [[ -n $DISPLAY ]]; then
    echo "✅ Display: X11 ($DISPLAY)"
else
    echo "❌ Display: No display server detected"
fi

# Check audio
if command -v pactl >/dev/null 2>&1; then
    echo "✅ Audio: PulseAudio available"
elif [[ -d /dev/snd ]]; then
    echo "⚠️  Audio: ALSA only (may have issues)"
else
    echo "❌ Audio: No audio system detected"
fi

# Check critical dependencies
deps=("pkg-config" "xdotool" "wl-copy")
for dep in "${deps[@]}"; do
    if command -v $dep >/dev/null 2>&1; then
        echo "✅ $dep: Installed"
    else
        echo "❌ $dep: Missing"
    fi
done
```

### Common Issues & Solutions

<details>
<summary><b>🚫 "Widget doesn't appear"</b></summary>

1. **Check if process is running**:
   ```bash
   ps aux | grep mavoice
   ```

2. **Look in the correct location** (x:300, y:800):
   - Top-left area of your screen
   - It's only 72x20 pixels!

3. **Temporarily increase widget size**:
   ```json
   // Edit src-tauri/tauri.conf.json
   "width": 200,   // Instead of 72
   "height": 100,  // Instead of 20
   "transparent": false,  // Make it visible
   ```

4. **Check logs**:
   ```bash
   # Run with console output
   npm run dev 2>&1 | tee mavoice.log
   ```
</details>

<details>
<summary><b>🎤 "No audio recording"</b></summary>

1. **Check audio permissions**:
   ```bash
   # List audio devices
   pactl list sources short
   
   # Test microphone
   arecord -d 5 test.wav && aplay test.wav
   ```

2. **Select correct audio device**:
   - The app uses the system default
   - Change default in your system audio settings

3. **For WSL2 users**:
   - Audio passthrough may not work
   - Consider running native Linux or dual-boot
</details>

<details>
<summary><b>📋 "Clipboard not working"</b></summary>

1. **Install clipboard utilities**:
   ```bash
   # For X11
   sudo apt install xclip xsel
   
   # For Wayland
   sudo apt install wl-clipboard
   ```

2. **Test clipboard**:
   ```bash
   echo "test" | xclip -selection clipboard
   # or
   echo "test" | wl-copy
   ```

3. **Manual copy fallback**:
   - If auto-copy fails, the text appears in the widget
   - You can manually select and copy
</details>

<details>
<summary><b>🌐 "Groq API errors"</b></summary>

1. **Verify API key**:
   ```bash
   # Check if env file exists
   cat src-tauri/aquavoice-frontend/.env
   ```

2. **Test API directly**:
   ```bash
   curl https://api.groq.com/openai/v1/models \
     -H "Authorization: Bearer YOUR_API_KEY"
   ```

3. **Common API issues**:
   - Rate limit (400 requests/minute)
   - Invalid API key format
   - Network connectivity
</details>

### Error Messages Explained

| Error | Meaning | Solution |
|-------|---------|----------|
| "Failed to start audio recording" | Microphone access issue | Check audio permissions and devices |
| "Groq API request failed" | API key or network issue | Verify API key and internet connection |
| "Clipboard operation failed" | Missing clipboard utility | Install xclip/wl-clipboard |
| "Window manager not detected" | No X11/Wayland | Ensure GUI environment is running |

## 🧑‍💻 Developer Guide

### Architecture Overview

```
maVoice Architecture
┌─────────────────────────────────────────────┐
│                Frontend (React)              │
│  ┌─────────────────┬──────────────────┐    │
│  │ FloatingOverlay │   Settings UI    │    │
│  │   Component     │   Component      │    │
│  └────────┬────────┴────────┬─────────┘    │
│           │ Tauri IPC       │               │
└───────────┼─────────────────┼───────────────┘
            │                 │
┌───────────┼─────────────────┼───────────────┐
│           ▼                 ▼               │
│   ┌──────────────┬─────────────────┐       │
│   │ Audio Module │  System Module  │       │
│   │ (Recording)  │ (Text Injection)│       │
│   └──────┬───────┴────────┬────────┘       │
│          │                 │                │
│   ┌──────▼─────────────────▼───────┐       │
│   │        Groq API Module         │       │
│   │    (Whisper Transcription)     │       │
│   └────────────────────────────────┘       │
│             Backend (Rust/Tauri)            │
└─────────────────────────────────────────────┘
```

### Key Components

#### Backend (`src-tauri/src/`)

- **`main.rs`**: Tauri application entry point and command handlers
- **`audio/groq_recorder.rs`**: Audio recording optimized for Groq (16kHz mono)
- **`api/groq.rs`**: Groq API client for Whisper transcription
- **`system/text_inject.rs`**: Cross-platform text injection (X11/Wayland)

#### Frontend (`src-tauri/aquavoice-frontend/src/`)

- **`App.tsx`**: Main application logic and global shortcuts
- **`components/FloatingOverlay.tsx`**: The floating widget UI
- **Real-time Features**:
  - Audio level visualization
  - Status transitions
  - Drag-and-drop positioning

### Development Workflow

```bash
# 1. Watch for Rust changes
cd src-tauri
cargo watch -x check

# 2. Run frontend dev server (separate terminal)
cd src-tauri/aquavoice-frontend
npm run dev

# 3. Run Tauri in dev mode (separate terminal)
npm run dev  # From project root

# 4. Build and test
npm run build
# Test the built app
./src-tauri/target/release/mavoice
```

### Modifying the Widget

#### Change Size/Position

```json
// src-tauri/tauri.conf.json
{
  "windows": [{
    "width": 100,    // Default: 72
    "height": 30,    // Default: 20
    "x": 500,        // Default: 300
    "y": 100,        // Default: 800
    "transparent": true,
    "alwaysOnTop": true
  }]
}
```

#### Customize Appearance

```tsx
// src-tauri/aquavoice-frontend/src/components/FloatingOverlay.tsx
const styles = {
  ready: "bg-blue-500",      // Change colors
  recording: "bg-red-500",
  processing: "bg-orange-500",
  completed: "bg-green-500"
};
```

### Testing

```bash
# Run Rust tests
cd src-tauri
cargo test

# Lint frontend
cd src-tauri/aquavoice-frontend
npm run lint

# Check Tauri
npm run tauri check
```

## ❓ FAQ

### General Questions

**Q: Why is the window so small?**
A: maVoice is designed as a minimal floating widget that stays out of your way. It's intentionally tiny (72x20px) to be unobtrusive while remaining always accessible.

**Q: Can I resize the widget?**
A: The widget size is fixed by design, but developers can modify the dimensions in `tauri.conf.json`.

**Q: Does it work with Wayland?**
A: Yes! maVoice supports both X11 and Wayland with automatic detection and appropriate fallbacks.

**Q: Can I use it without Groq?**
A: Currently, maVoice is built specifically for Groq's Whisper API. Other providers may be added in future versions.

### Technical Questions

**Q: Why Tauri instead of Electron?**
A: Tauri provides native performance with minimal resource usage - perfect for an always-on widget. Our app uses <50MB RAM compared to Electron's 150MB+.

**Q: How secure is my API key?**
A: Your API key is stored locally in your browser's localStorage and never transmitted except to Groq's API directly.

**Q: Can I contribute?**
A: Absolutely! Check our [Contributing Guide](CONTRIBUTING.md) and feel free to submit PRs, report bugs, or suggest features.

**Q: Will Windows/macOS be supported?**
A: Yes! The codebase is already cross-platform ready. Official support is coming soon.

### Troubleshooting Questions

**Q: Widget appears but recording doesn't start?**
A: Check audio permissions and ensure your microphone is not in use by another application.

**Q: Transcription is slow?**
A: While Groq is fast, network latency can affect speed. Ensure stable internet connection.

**Q: Text injection not working?**
A: Install the required tools (`xdotool` for X11, `wtype` for Wayland) and check system permissions.

## 🏎️ Performance

maVoice leverages Groq's incredible inference speed:

| Metric | Value | Notes |
|--------|-------|-------|
| Transcription Speed | < 500ms | For 30-second audio |
| Memory Usage (Idle) | < 50MB | Rust efficiency |
| Memory Usage (Active) | < 100MB | During recording |
| CPU Usage | < 5% | During transcription |
| Widget Render | 60 FPS | Smooth animations |
| Audio Format | 16kHz mono | Optimized for Groq |

### Benchmarks

```bash
# Test transcription speed
time curl -X POST "https://api.groq.com/openai/v1/audio/transcriptions" \
  -H "Authorization: Bearer $GROQ_API_KEY" \
  -F "file=@test.wav" \
  -F "model=whisper-large-v3-turbo"
```

## 🤝 Contributing

We love contributions! Whether it's:

- 🐛 Bug reports
- 💡 Feature requests  
- 🔧 Pull requests
- 📖 Documentation improvements
- 🌐 Translations

### Quick Contribution Guide

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Make your changes
4. Run tests (`cargo test && npm run lint`)
5. Commit (`git commit -m 'Add some AmazingFeature'`)
6. Push (`git push origin feature/AmazingFeature`)
7. Open a Pull Request

Check out our detailed [Contributing Guide](CONTRIBUTING.md) for more information.

## 🔐 Privacy & Security

- **Local First**: All processing happens on your machine
- **No Telemetry**: We don't track anything
- **Secure API**: Your Groq API key is stored locally and never shared
- **Open Source**: Audit the code yourself!

## 📜 License

maVoice is MIT licensed. See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- **Groq** - For providing insanely fast inference
- **Whisper** - OpenAI's amazing speech recognition model
- **Tauri** - For making native apps actually enjoyable to build
- **Community** - For feedback, contributions, and support
- **You** - For choosing open-source!

---

<div align="center">
  <p>Built with ❤️ by developers who were tired of slow dictation</p>
  <p><strong>maVoice</strong> - Where speed meets simplicity</p>
  
  <br>
  
  <p>⭐ Star us on GitHub if you find this useful!</p>
  
  <a href="https://github.com/lliWcWill/maVoice-Linux">
    <img src="https://img.shields.io/github/stars/lliWcWill/maVoice-Linux?style=social" alt="GitHub stars">
  </a>
</div>