# AquaVoice-Linux Development Status & Handoff Guide

## 🎯 PROJECT OVERVIEW
**Goal**: Build the FIRST Linux version of AquaVoice - a floating voice recorder that injects transcribed text anywhere on desktop

**Current AquaVoice**: Mac/Windows only, 450ms transcription, context-aware, works in ANY app (terminals, browsers, editors)

**Our Stack**: Rust + Tauri + Groq API + Docker

---

## 📋 RESEARCH COMPLETED

### ✅ AquaVoice Analysis
- **Features**: 450ms transcription speed, context-aware, app-agnostic text insertion
- **Architecture**: Fusion transcription engine + client-side context processor
- **Missing**: Linux support (HUGE market gap!)

### ✅ Technical Stack Decision
- **Winner**: Rust + Tauri (fast, native, <3MB binaries)
- **Audio**: CPAL crate for cross-platform recording
- **Text Insertion**: 
  - X11: `xdotool` (easy)
  - Wayland: `wl-clipboard` + workarounds (challenging but doable)
- **Packaging**: Built-in .deb via `cargo tauri build`

### ✅ Docker Environment
- **Files Created**: `Dockerfile`, `docker-compose.dev.yml`
- **Features**: Full dev environment with audio, X11/Wayland, Rust, Node.js, Tauri CLI
- **Ready**: Audio recording, system access, GUI forwarding

---

## 🚀 NEXT STEPS (IMMEDIATE ACTIONS)

### 1. Initialize Tauri Project
```bash
cd AquaVoice-Linux
npm create tauri-app@latest . --name "AquaVoice Linux" --window-title "AquaVoice Linux" --template vanilla
```

### 2. Start Docker Development
```bash
# Set up environment
export UID=$(id -u)
export GID=$(id -g)

# Start dev container (X11)
docker-compose -f docker-compose.dev.yml up aquavoice-dev -d

# OR for Wayland
docker-compose -f docker-compose.dev.yml up aquavoice-wayland -d

# Enter container
docker exec -it aquavoice-linux-dev bash
```

### 3. Add Core Dependencies
```bash
# In src-tauri directory
cargo add cpal              # Audio recording
cargo add tokio --features full  # Async runtime
cargo add serde_json        # JSON handling
cargo add reqwest --features json  # HTTP client for Groq
cargo add tauri-plugin-shell  # System commands
```

### 4. Implement MVP Components
1. **Audio Capture** (Rust backend)
2. **Groq API Integration** (Rust backend)  
3. **Text Injection** (X11 first, Wayland later)
4. **Floating UI** (HTML/CSS/JS frontend)

---

## 🔧 TECHNICAL ARCHITECTURE

### Backend (Rust)
```
src-tauri/src/
├── main.rs           # Tauri app entry
├── audio/
│   ├── capture.rs    # CPAL audio recording
│   └── mod.rs
├── api/
│   ├── groq.rs       # Groq transcription API
│   └── mod.rs
├── system/
│   ├── text_inject.rs # X11/Wayland text insertion
│   └── mod.rs
└── lib.rs           # Module declarations
```

### Frontend (Web)
```
src/
├── index.html       # Floating UI
├── styles.css       # macOS-inspired design
├── main.js          # Tauri IPC commands
└── assets/
```

### Key Crates Needed
- `cpal` - Audio recording
- `reqwest` - HTTP client for Groq API
- `serde/serde_json` - JSON serialization
- `tokio` - Async runtime
- `tauri-plugin-shell` - System command execution

---

## 🎯 MVP FEATURES (Phase 1)
1. ✅ **Project Setup** (Docker + Tauri)
2. ⏳ **Audio Recording** (CPAL microphone capture)
3. ⏳ **Groq Integration** (Audio → Text via API)
4. ⏳ **Basic Text Injection** (X11 via xdotool)
5. ⏳ **Floating UI** (Always-on-top window)
6. ⏳ **Global Hotkey** (Record/Stop)

---

## 🔥 CONTEXT7 SETUP REQUIRED

**CRITICAL**: New Claude instance needs Context7 docs:

```bash
# Get Tauri docs
mcp__context7__resolve-library-id libraryName="tauri"
mcp__context7__get-library-docs context7CompatibleLibraryID="/tauri-apps/tauri-docs" topic="getting started development setup" tokens=5000

# Get CPAL audio docs  
mcp__context7__resolve-library-id libraryName="cpal"
mcp__context7__get-library-docs context7CompatibleLibraryID="/rustaudio/cpal" topic="audio recording microphone" tokens=3000

# Get Groq API integration (if available)
mcp__context7__resolve-library-id libraryName="groq"
```

---

## 💡 HANDOFF PROMPT FOR NEW CLAUDE

```
I'm continuing development of AquaVoice-Linux, the first Linux clone of the popular Mac/Windows voice recorder app. 

CONTEXT: Read SETUP-STATUS.md in this directory for full research and architecture decisions.

KEY POINTS:
- Stack: Rust + Tauri + Groq API + Docker  
- Goal: 450ms transcription speed, floating UI, cross-app text insertion
- Phase 1: MVP with audio recording → Groq → text injection (X11 first)
- Docker environment ready, need to initialize Tauri project and start coding

IMMEDIATE TASKS:
1. Use Context7 to get Tauri and CPAL documentation (commands in SETUP-STATUS.md)
2. Initialize Tauri project structure
3. Implement audio recording with CPAL
4. Set up Groq API integration for transcription
5. Create basic floating UI

This is a high-priority project with huge market potential. Let's build fast and iterate.
```

---

## 📊 CURRENT TODO STATUS

✅ Research AquaVoice architecture and features  
✅ Research Linux desktop development best practices  
✅ Research text insertion methods (X11/Wayland)  
✅ Research audio permissions on Debian  
✅ Research packaging methods  
✅ Design system architecture  
✅ Create Docker development environment  
⏳ Initialize Tauri project structure  
⏳ Implement audio recording (CPAL)  
⏳ Set up Groq API integration  
⏳ Test end-to-end pipeline  

---

**STATUS**: Ready to start coding. Docker environment prepared, architecture designed, all research complete. Next Claude instance should pick up from Tauri initialization and start building the core functionality.

---

## 🚨 CRITICAL REMINDERS

1. **Wayland Challenge**: Text injection is restricted - plan for clipboard fallback
2. **Audio Permissions**: PulseAudio works automatically on modern Debian
3. **Performance Target**: <450ms transcription to match AquaVoice
4. **Market Opportunity**: First-to-market Linux voice recorder with AquaVoice features
5. **Docker Ready**: Full dev environment with audio, GUI, and system access configured