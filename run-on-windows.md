# Running maVoice on Native Windows

Since WSL2 doesn't support microphone input, here's how to run maVoice directly on Windows:

## Prerequisites

1. **Install Node.js for Windows**
   - Download from: https://nodejs.org/
   - Choose the LTS version

2. **Install Rust for Windows**
   - Download from: https://www.rust-lang.org/tools/install
   - Run: `rustup-init.exe`

3. **Install Visual Studio Build Tools**
   - Download from: https://visualstudio.microsoft.com/downloads/
   - Select "Desktop development with C++" workload

## Steps

1. **Open Windows Terminal/PowerShell** (not WSL)

2. **Clone the repository**
   ```powershell
   git clone https://github.com/lliWcWill/maVoice-Linux.git
   cd maVoice-Linux
   ```

3. **Install dependencies**
   ```powershell
   npm install
   ```

4. **Set up API key**
   ```powershell
   echo "VITE_GROQ_API_KEY=your_groq_api_key_here" > src-tauri/aquavoice-frontend/.env
   ```

5. **Run the app**
   ```powershell
   npm run dev
   ```

The app should now work with full microphone access on Windows!

## Note
- The window will still be tiny (72x20px)
- Look at coordinates x:300, y:800
- The app name says "Linux" but it works on Windows too (Tauri is cross-platform)