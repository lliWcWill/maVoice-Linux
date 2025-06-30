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
│  🎤 maVoice     │  ← Tiny floating widget (100x22px)
│ ▶ ■ ▪ ▪ ▪ ▪    │    Always on top of your screen
└─────────────────┘    Double-click to start!
```

</div>

---

## ✨ Features

- **⚡ Blazing Fast**: Powered by Groq's Whisper Large v3 Turbo model - the fastest inference in the game
- **🎯 Native Performance**: Built with Rust and Tauri for minimal resource usage
- **🎨 Beautiful UI**: Sleek, modern floating widget that stays out of your way
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

- **Size**: 100x22 pixels (compact floating button)
- **Behavior**: Always on top, transparent background, no window borders
- **Dragging**: Right-click or Ctrl+Left-click to drag to a new position

## 🏎️ Why Groq DEV Tier?

<div align="center">
  <table>
    <tr>
      <th>Feature</th>
      <th>Groq DEV Tier</th>
      <th>OpenAI Free</th>
    </tr>
    <tr>
      <td>Speed</td>
      <td>🚀 Lightning Fast</td>
      <td>🐌 Slow</td>
    </tr>
    <tr>
      <td>Rate Limits</td>
      <td>💪 400 RPM</td>
      <td>😔 Limited</td>
    </tr>
    <tr>
      <td>Model</td>
      <td>🧠 Whisper v3 Turbo</td>
      <td>🤖 Basic Whisper</td>
    </tr>
    <tr>
      <td>Quality</td>
      <td>🎯 Premium</td>
      <td>📉 Variable</td>
    </tr>
  </table>
</div>

## 🚀 Quick Start

### 🌟 **ONE-COMMAND Install**

```bash
# Clone and install everything automatically
git clone https://github.com/lliWcWill/maVoice-Linux.git
cd maVoice-Linux
./install.sh

# Add your Groq API key
echo "VITE_GROQ_API_KEY=your_groq_api_key_here" > src-tauri/aquavoice-frontend/.env

# Launch!
npm run dev
```

### Prerequisites

- Node.js 18+
- Rust 1.70+
- A Groq API key ([Get one here](https://console.groq.com))

### Platform-Specific Setup

<details>
<summary><b>🪟 WSL2 Setup (Windows Users)</b></summary>

**✨ BREAKTHROUGH: WSL2 + WSLg provides PERFECT voice dictation with zero audio issues!**

#### Prerequisites

1. **Update WSL2** (from Windows PowerShell as Administrator):
   ```powershell
   wsl --update
   wsl --version  # Ensure version 2 with WSLg
   ```

2. **Install Debian/Ubuntu** if you don't have it:
   ```powershell
   wsl --install -d Debian
   ```

#### Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install system dependencies
sudo apt update && sudo apt install -y \
    build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
    libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libdbus-1-dev \
    libappindicator3-dev librsvg2-dev libasound2-dev \
    xdotool wl-clipboard wtype

# Clone and run
git clone https://github.com/lliWcWill/maVoice-Linux.git
cd maVoice-Linux
./install.sh
```

</details>

<details>
<summary><b>🐧 Native Linux Setup</b></summary>

**Debian/Ubuntu:**
```bash
sudo apt update
sudo apt install -y \
    build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
    libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libdbus-1-dev \
    libappindicator3-dev librsvg2-dev libasound2-dev \
    xdotool wl-clipboard wtype
```

**Fedora/Arch** - See [detailed instructions](QUICK_REFERENCE.md)

</details>

### 📦 Build Debian Package

```bash
# Build the .deb package
npm run build

# The .deb file will be in:
# src-tauri/target/release/bundle/deb/
```

## 🎮 Usage

### Desktop App
1. **Launch maVoice** - The app appears as a sleek floating widget
2. **Double-click to start** - The microphone activates with visual feedback
3. **Speak naturally** - Real-time audio visualization shows your voice
4. **Single-click to stop** - Transcription appears instantly
5. **Copy & paste** - Text is automatically copied to clipboard

### Web Interface (http://localhost:5173)
- **Settings panel** - Click the gear icon for full configuration
- **API key setup** - Secure local storage of your Groq key
- **Model selection** - Choose from Whisper variants
- **Custom prompts** - Add technical terms, names, or style instructions
- **Temperature control** - Adjust creativity vs accuracy
- **Multi-language** - Support for 100+ languages

### Keyboard Shortcuts
- `Ctrl+,` - Open settings
- `Alt+Space` - Toggle recording
- `Double Alt` - Quick record
- `Spacebar` - Stop recording (while active)

## 🛠️ Tech Stack

<div align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Tauri-24C8DB?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri">
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript">
  <img src="https://img.shields.io/badge/Tailwind-38B2AC?style=for-the-badge&logo=tailwind-css&logoColor=white" alt="Tailwind">
</div>

## 🤝 Contributing

We love contributions! Whether it's:

- 🐛 Bug reports
- 💡 Feature requests
- 🔧 Pull requests
- 📖 Documentation improvements

Check out our [Contributing Guide](CONTRIBUTING.md) to get started.

## 📈 Performance

maVoice leverages Groq's incredible inference speed:

- **Transcription Speed**: < 500ms for 30-second audio
- **Memory Usage**: < 50MB idle, < 100MB active
- **CPU Usage**: < 5% during transcription
- **Network**: Minimal bandwidth usage with smart chunking

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
- **You** - For choosing open-source!

---

<div align="center">
  <p>Built with ❤️ by developers who were tired of slow dictation</p>
  <p><strong>maVoice</strong> - Where speed meets simplicity</p>
</div>