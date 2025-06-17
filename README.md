# 🎙️ AquaVoice-Linux 

**The First Linux Voice Recorder with AquaVoice-Style Features**

A floating, context-aware voice recorder that transcribes speech and injects text anywhere on your Linux desktop - terminals, browsers, editors, anywhere with a text cursor.

## 🚀 Quick Start

```bash
# Start development environment
export UID=$(id -u) && export GID=$(id -g)
docker-compose -f docker-compose.dev.yml up aquavoice-dev -d
docker exec -it aquavoice-linux-dev bash

# Initialize project (first time)
npm create tauri-app@latest . --template vanilla

# Run development server
npm run tauri dev
```

## 🎯 Features (Planned)

- ⚡ **Ultra-fast transcription** (<450ms via Groq API)
- 🎯 **Context-aware** (analyzes screen content for accuracy)
- 🌍 **Universal compatibility** (works in ANY application)
- 🔥 **Floating UI** (always-on-top recorder)
- ⌨️ **Global hotkeys** (record from anywhere)
- 🐧 **Linux native** (X11 + Wayland support)

## 🏗️ Tech Stack

- **Backend**: Rust + Tauri
- **Audio**: CPAL (cross-platform audio)
- **API**: Groq Speech-to-Text
- **UI**: HTML/CSS/JS (macOS-inspired)
- **System**: xdotool (X11), wl-clipboard (Wayland)
- **Deployment**: Docker + .deb packages

## 📋 Development Status

See `SETUP-STATUS.md` for detailed progress and architecture decisions.

## 🤝 Contributing

This project aims to bring AquaVoice-quality voice recording to Linux users. Currently in active development.

## 📄 License

MIT License - See LICENSE file for details