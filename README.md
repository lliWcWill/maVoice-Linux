<div align="center">

# maVoice

**AI voice assistant that lives on your desktop**

<img src="https://img.shields.io/badge/Pure_Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Pure Rust">
<img src="https://img.shields.io/badge/Gemini_Live-4285F4?style=for-the-badge&logo=google&logoColor=white" alt="Gemini Live">
<img src="https://img.shields.io/badge/Groq_Whisper-10B981?style=for-the-badge&logo=lightning&logoColor=white" alt="Groq Whisper">
<img src="https://img.shields.io/badge/wgpu_Shaders-FF6B6B?style=for-the-badge&logo=webgpu&logoColor=white" alt="wgpu">
<img src="https://img.shields.io/badge/License-MIT-45B7D1?style=for-the-badge&logo=opensource&logoColor=white" alt="MIT">

A pure Rust desktop overlay with GPU-rendered visuals, bidirectional voice via Gemini Live, Groq-powered transcription, and a real-time agent dashboard.

</div>

---

## What is maVoice?

maVoice is a **floating AI voice assistant** rendered directly on your desktop using GPU shaders. No Electron. No WebView. No browser. Just two transparent windows — an animated AI orb and a waveform strip — that sit on top of everything and respond to your voice in real-time.

It operates in two modes:

- **Groq mode** — Push-to-talk dictation. Record, transcribe via Groq Whisper, paste to clipboard.
- **Gemini mode** — Always-on bidirectional voice conversation via Gemini 2.0 Flash Live. The AI can search memory, run shell commands, delegate tasks to Claude, and remember things across sessions.

## Two Versions

This repo contains **two implementations** — the original Tauri/React app and the newer pure Rust native overlay. Both live in the same repo and both work.

| | **mavoice-native** (new) | **src-tauri** (original) |
|---|---|---|
| Stack | Pure Rust, wgpu, WGSL shaders | Tauri 2 + React + TypeScript |
| Rendering | GPU shaders on transparent X11 windows | WebKitGTK floating widget |
| Voice | Gemini Live bidirectional + Groq STT | Groq STT only |
| Tools | search_memory, remember, run_command, ask_claude | — |
| Dashboard | WebSocket broadcast to claudegram | — |
| UI | AI orb + waveform strip (shader-rendered) | Floating button with settings panel |
| Size | ~5MB static binary | ~50MB (Tauri + WebKitGTK) |

The **native version** (`mavoice-native/`) is the active development target. The **Tauri version** (`src-tauri/`) remains in the repo as a fully functional alternative — useful if you want the settings UI, web-based configuration panel, or prefer the widget-style interface.

## Architecture (Native)

```
┌──────────────────────────────────────────────────────┐
│          mavoice-native (pure Rust binary)           │
│                                                      │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────┐  │
│  │ wgpu/WGSL│  │  cpal    │  │ Gemini Live (WS)   │  │
│  │ renderer │  │  audio   │  │ bidirectional voice│  │
│  │ 2 windows│  │ capture  │  │ + function calling │  │
│  └──────────┘  └──────────┘  └────────────────────┘  │
│                                                      │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────┐  │
│  │ Groq API │  │ Global   │  │ Dashboard WS       │  │
│  │ Whisper  │  │ Hotkeys  │  │ broadcast (3001)   │  │
│  │ STT      │  │ F2 / F3  │  │ → claudegram UI    │  │
│  └──────────┘  └──────────┘  └────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐ │
│  │ Tools: search_memory, remember, run_command,    │ │
│  │        ask_claude                               │ │
│  └─────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

### Rendering

Two transparent always-on-top windows rendered with **wgpu + WGSL shaders**:

- **AI Orb** (96px) — Animated spiral sphere shader that reacts to voice state (idle pulse, speaking glow, thinking spin)
- **Waveform Strip** (64px) — Real-time audio level visualization at the bottom of the screen

Both windows use `softbuffer` for X11 transparency compositing. No toolkit, no DOM, no CSS — raw GPU pixels on a transparent surface.

### Voice Pipeline

**Groq mode:**
```
Mic (cpal) → WAV buffer → Groq Whisper API → clipboard (xclip) → xdotool paste
```

**Gemini mode:**
```
Mic (cpal) → PCM 16kHz → WebSocket → Gemini 2.0 Flash Live
                                          ↓
                              ← Audio response (PCM 24kHz)
                              ← Function calls (tools)
                              ← Text responses
```

### Gemini Tools

When in Gemini mode, the AI has access to 4 function-calling tools:

| Tool | Description |
|------|-------------|
| `search_memory` | FTS5 search over the ShieldCortex memory database (SQLite) |
| `remember` | Save a new memory to the database for cross-session recall |
| `run_command` | Execute a shell command with 30s timeout, return stdout/stderr |
| `ask_claude` | Delegate a task to Claude Code CLI, return the response |

### Dashboard

A WebSocket broadcast server on `ws://localhost:3001` streams real-time events to the [**claudegram dashboard**](https://github.com/lliWcWill/claudegram-dashboard) — a separate Next.js project with a glass-morphism UI that shows:

- Agent status cards (Claude, Gemini, Droid, Groq) with live state indicators
- Kanban board for tracking agent tasks across columns
- Action log with conversation bubbles, per-event copy, and session export
- Tool call timeline with elapsed timers

See the [claudegram-dashboard repo](https://github.com/lliWcWill/claudegram-dashboard) for setup and usage.

## Quick Start (Native)

### Prerequisites

- Rust 1.75+
- Linux with X11 (Wayland support planned)
- A [Groq API key](https://console.groq.com) for transcription
- A [Google AI API key](https://aistudio.google.com/apikey) for Gemini Live voice

### System Dependencies (Debian/Ubuntu)

```bash
sudo apt install -y \
    build-essential pkg-config \
    libasound2-dev \
    xdotool xclip \
    libx11-dev libxcb1-dev
```

### Build & Install

```bash
git clone https://github.com/lliWcWill/maVoice-Linux.git
cd maVoice-Linux/mavoice-native

# Build release binary
cargo build --release

# Install to ~/.local/bin
cp target/release/mavoice-native ~/.local/bin/

# Create config
mkdir -p ~/.config/mavoice
cat > ~/.config/mavoice/config.toml << 'EOF'
api_key = "gsk_your_groq_key_here"
gemini_api_key = "your_google_ai_key_here"
model = "whisper-large-v3-turbo"
language = "en"
mode = "gemini"
voice_name = "Aoede"
EOF
```

### Run

```bash
mavoice-native
```

### Systemd Service (auto-start)

```bash
mkdir -p ~/.config/systemd/user

cat > ~/.config/systemd/user/mavoice.service << 'EOF'
[Unit]
Description=maVoice — AI Voice Assistant Overlay
Documentation=https://github.com/lliWcWill/maVoice-Linux
After=graphical-session.target

[Service]
Type=simple
ExecStart=%h/.local/bin/mavoice-native
Restart=on-failure
RestartSec=3
Environment=DISPLAY=:0
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now mavoice
```

## Quick Start (Tauri — Legacy)

The original Tauri version is a floating desktop widget with a React-based settings panel, model selection, and multi-language support.

### Prerequisites

- Node.js 18+
- Rust 1.70+
- Tauri 2 system dependencies (WebKitGTK, etc.)

### Install & Run

```bash
git clone https://github.com/lliWcWill/maVoice-Linux.git
cd maVoice-Linux
./install.sh

# Add your Groq API key
echo "VITE_GROQ_API_KEY=your_groq_api_key_here" > src-tauri/aquavoice-frontend/.env

# Launch
npm run dev
```

<details>
<summary><b>Tauri system dependencies (Debian/Ubuntu)</b></summary>

```bash
sudo apt install -y \
    build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
    libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libdbus-1-dev \
    libappindicator3-dev librsvg2-dev libasound2-dev \
    xdotool wl-clipboard wtype
```

</details>

<details>
<summary><b>WSL2 setup</b></summary>

WSL2 + WSLg works for the Tauri version. Update WSL2 from PowerShell:
```powershell
wsl --update
wsl --version  # Ensure version 2 with WSLg
```
Then install dependencies inside your WSL2 distro and run normally.

</details>

### Tauri Usage

- **Double-click** the floating widget to start recording
- **Single-click** to stop and transcribe
- **Right-click** or **Ctrl+click** to drag the widget
- **Settings** via the gear icon (model selection, language, custom prompts, temperature)

## Usage (Native)

### Hotkeys

| Key | Action |
|-----|--------|
| **F2** | Toggle Groq dictation (push-to-talk) |
| **F3** | Toggle Gemini Live voice conversation |

### Groq Mode (F2)

1. Press **F2** to start recording
2. Speak naturally
3. Press **F2** again to stop
4. Transcription is copied to clipboard and pasted at cursor

### Gemini Mode (F3)

1. Press **F3** to open a Gemini Live session
2. Speak naturally — the AI responds with voice in real-time
3. The AI can use tools (search memory, run commands, ask Claude)
4. Press **F3** again to end the session
5. Supports barge-in (interrupt the AI mid-sentence)

### Configuration

Edit `~/.config/mavoice/config.toml`:

```toml
api_key = "gsk_..."                # Groq API key
gemini_api_key = "AI..."           # Google AI API key
model = "whisper-large-v3-turbo"   # Groq model
language = "en"                    # Transcription language
mode = "gemini"                    # Default mode: "groq" or "gemini"
voice_name = "Aoede"               # Gemini voice: Puck, Charon, Kore, Fenrir, Aoede
system_instruction = "..."         # Custom system prompt for Gemini
temperature = 0.0                  # Groq transcription temperature
dictionary = ""                    # Custom terms for Groq
```

## Tech Stack

### Native (`mavoice-native/`)

| Component | Technology |
|-----------|------------|
| Language | Rust (pure, no WebView) |
| GPU Rendering | wgpu + WGSL shaders |
| Window Management | winit + softbuffer (X11 transparency) |
| Audio Capture | cpal (ALSA) |
| Voice AI | Gemini 2.0 Flash Live (WebSocket) |
| Transcription | Groq Whisper Large v3 Turbo |
| Tool Execution | rusqlite, tokio::process, Claude CLI |
| Dashboard | tokio-tungstenite broadcast server |
| Hotkeys | global-hotkey crate |
| Clipboard | xclip, xdotool |

### Tauri (`src-tauri/`)

| Component | Technology |
|-----------|------------|
| Framework | Tauri 2 |
| Frontend | React + TypeScript + Tailwind |
| Transcription | Groq Whisper (via groq-sdk) |
| Audio | Web Audio API |

## Project Structure

```
maVoice-Linux/
├── mavoice-native/              # ← Pure Rust native overlay (active)
│   ├── src/
│   │   ├── main.rs              # Entry point, window creation
│   │   ├── app.rs               # Event loop, state machine, dashboard
│   │   ├── renderer.rs          # wgpu setup, shader pipeline
│   │   ├── shader.wgsl          # Waveform strip shader
│   │   ├── ai_shader.wgsl       # AI orb spiral sphere shader
│   │   ├── config.rs            # TOML config loading
│   │   ├── dashboard.rs         # WebSocket broadcast server
│   │   ├── state_machine.rs     # App state transitions
│   │   ├── api/
│   │   │   ├── gemini.rs        # Gemini Live bidirectional WebSocket
│   │   │   └── groq.rs          # Groq Whisper transcription API
│   │   ├── audio/
│   │   │   ├── recorder.rs      # cpal microphone capture
│   │   │   └── player.rs        # PCM audio playback
│   │   ├── system/
│   │   │   ├── hotkeys.rs       # Global F2/F3 hotkey registration
│   │   │   └── text_inject.rs   # xdotool clipboard paste
│   │   └── tools/
│   │       └── mod.rs           # Gemini function calling tools
│   └── Cargo.toml
│
├── src-tauri/                   # ← Tauri 2 desktop app (legacy)
│   ├── aquavoice-frontend/      # React + TypeScript UI
│   │   └── src/components/
│   │       └── FloatingOverlay.tsx
│   ├── src/main.rs              # Tauri backend
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── install.sh                   # Tauri dependency installer
├── package.json                 # Tauri npm scripts
└── README.md
```

## Related Projects

- **[claudegram-dashboard](https://github.com/lliWcWill/claudegram-dashboard)** — Real-time agent monitoring dashboard (Next.js + glass morphism UI). Connects to maVoice's WebSocket broadcast server to display agent status, kanban tasks, conversation logs, and tool call timelines.

## License

MIT License. See [LICENSE](LICENSE) for details.

---

<div align="center">
  <strong>maVoice</strong> — Pure Rust AI voice on your desktop
</div>
