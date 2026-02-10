#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
SERVICE_DIR="$HOME/.config/systemd/user"
ENV_FILE="$HOME/.config/mavoice/env"

echo "=== maVoice Installer ==="

# 1. Build release binary
echo "[1/4] Building release binary..."
cd "$SCRIPT_DIR"
cargo build --release
echo "  Build complete."

# 2. Install binary
echo "[2/4] Installing binary to $BIN_DIR..."
mkdir -p "$BIN_DIR"
cp target/release/mavoice-native "$BIN_DIR/mavoice-native"
chmod +x "$BIN_DIR/mavoice-native"
echo "  Installed mavoice-native."

# 3. Create env file for API keys if it doesn't exist
if [ ! -f "$ENV_FILE" ]; then
    echo "[3/4] Creating env file at $ENV_FILE..."
    mkdir -p "$(dirname "$ENV_FILE")"
    cat > "$ENV_FILE" <<'ENVEOF'
# maVoice API keys — used by the systemd service
# Uncomment and fill in your keys:
# GROQ_API_KEY=
# GEMINI_API_KEY=
ENVEOF
    echo "  Created env template. Edit $ENV_FILE to add your API keys."
else
    echo "[3/4] Env file already exists at $ENV_FILE (skipping)."
fi

# 4. Install and enable systemd user service
echo "[4/4] Installing systemd user service..."
mkdir -p "$SERVICE_DIR"
cp "$SCRIPT_DIR/mavoice.service" "$SERVICE_DIR/mavoice.service"
systemctl --user daemon-reload
systemctl --user enable mavoice.service
echo "  Service enabled. Starting..."
systemctl --user restart mavoice.service
echo "  Done!"

echo ""
echo "=== Installation complete ==="
echo "  Binary:  $BIN_DIR/mavoice-native"
echo "  Service: $SERVICE_DIR/mavoice.service"
echo "  Env:     $ENV_FILE"
echo ""
echo "Commands:"
echo "  systemctl --user status mavoice    # Check status"
echo "  systemctl --user restart mavoice   # Restart"
echo "  systemctl --user stop mavoice      # Stop"
echo "  journalctl --user -u mavoice -f    # View logs"
