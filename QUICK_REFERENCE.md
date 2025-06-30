# 🎤 maVoice Quick Reference

## 🚀 Quick Setup (Copy & Paste)

### WSL2 Users
```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Install dependencies
sudo apt update && sudo apt install -y build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.0-dev libsoup2.4-dev libjavascriptcoregtk-4.0-dev libdbus-1-dev libappindicator3-dev librsvg2-dev libasound2-dev xdotool wl-clipboard wtype

# 3. Clone and setup
git clone https://github.com/lliWcWill/maVoice-Linux.git
cd maVoice-Linux
npm install
echo "VITE_GROQ_API_KEY=your_key_here" > src-tauri/aquavoice-frontend/.env

# 4. Run
npm run dev
```

### Native Linux Users
```bash
# Run our setup script
./setup-mavoice.sh
```

## 🎯 Finding the Widget

```
Your Screen (1920x1080)
┌────────────────────────┐
│     ↓ Here (x:300)     │
│  ┌─────────────┐       │ ← y:800
│  │ 🎤 maVoice  │       │   (tiny 72x20px widget)
│  └─────────────┘       │
└────────────────────────┘
```

**Can't find it?** Edit `src-tauri/tauri.conf.json`:
```json
"width": 200,  // Make it bigger temporarily
"height": 100,
"transparent": false  // Make it visible
```

## 🎮 Controls

| Action | How to |
|--------|---------|
| Start Recording | Double-click widget |
| Stop Recording | Single-click widget |
| Move Widget | Right-click + drag |
| Global Shortcut | Ctrl+Shift+, |

## 🔴 Widget States

- 🔵 **Blue** = Ready
- 🔴 **Red** = Recording (with animated bars)
- 🟠 **Orange** = Processing
- 🟢 **Green** = Success (text copied!)

## 🚫 Common Fixes

### "No widget appears"
```bash
ps aux | grep mavoice  # Check if running
# Look at x:300, y:800 (top-left area)
```

### "No audio"
```bash
pactl list sources short  # Check audio devices
arecord -d 5 test.wav    # Test mic
```

### "Clipboard fails"
```bash
# X11
sudo apt install xclip xsel

# Wayland  
sudo apt install wl-clipboard
```

### "API errors"
```bash
# Check API key
cat src-tauri/aquavoice-frontend/.env

# Test API
curl https://api.groq.com/openai/v1/models \
  -H "Authorization: Bearer YOUR_KEY"
```

## 📁 Key Files

- `tauri.conf.json` - Window size/position
- `.env` - API key storage
- `FloatingOverlay.tsx` - Widget UI
- `groq_recorder.rs` - Audio recording

## 🛠️ Dev Commands

```bash
# Frontend only
cd src-tauri/aquavoice-frontend && npm run dev

# Full app (what you want)
npm run dev

# Build .deb package
npm run build
```

## 🆘 Still Stuck?

1. Run diagnostics: `bash check-mavoice.sh`
2. Check logs: `npm run dev 2>&1 | tee debug.log`
3. File issue: [GitHub Issues](https://github.com/lliWcWill/maVoice-Linux/issues)

---
Remember: The widget is TINY (72x20px) - about the size of this box: [    ]