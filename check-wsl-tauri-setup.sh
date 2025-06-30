#!/bin/bash

echo "=== WSL GUI Support Check for Tauri Applications ==="
echo

# Check WSLg support
echo "1. Checking WSLg support..."
echo "   DISPLAY variable: $DISPLAY"
echo "   WAYLAND_DISPLAY: $WAYLAND_DISPLAY"
echo "   X11 socket: $(ls /tmp/.X11-unix/ 2>/dev/null | grep -c X0)"
echo "   PulseAudio: $PULSE_SERVER"
echo

# Check WSL version
echo "2. WSL Information:"
echo "   Kernel: $(uname -r)"
echo "   WSL version supports GUI: $(if [[ -n "$DISPLAY" && -n "$WAYLAND_DISPLAY" ]]; then echo "YES"; else echo "NO"; fi)"
echo

# Check Tauri dependencies
echo "3. Checking Tauri dependencies..."
echo

# Function to check if a package is installed
check_package() {
    if command -v pkg-config >/dev/null 2>&1; then
        if pkg-config --exists "$1" 2>/dev/null; then
            echo "   ✓ $1: $(pkg-config --modversion $1 2>/dev/null || echo "installed")"
        else
            echo "   ✗ $1: NOT FOUND"
        fi
    else
        if dpkg -l | grep -q "^ii.*$2"; then
            echo "   ✓ $2: installed"
        else
            echo "   ✗ $2: NOT INSTALLED"
        fi
    fi
}

# Check essential packages
echo "   Essential packages:"
check_package "gtk+-3.0" "libgtk-3-dev"
check_package "webkit2gtk-4.0" "libwebkit2gtk-4.0-dev"
check_package "libsoup-2.4" "libsoup2.4-dev"
check_package "dbus-1" "libdbus-1-dev"

echo
echo "   Build tools:"
if command -v pkg-config >/dev/null 2>&1; then
    echo "   ✓ pkg-config: installed"
else
    echo "   ✗ pkg-config: NOT INSTALLED"
fi

if command -v gcc >/dev/null 2>&1; then
    echo "   ✓ gcc: $(gcc --version | head -n1)"
else
    echo "   ✗ gcc: NOT INSTALLED"
fi

echo
echo "   Optional packages:"
check_package "appindicator3-0.1" "libappindicator3-dev"
check_package "librsvg-2.0" "librsvg2-dev"

echo
echo "4. Installation commands for missing dependencies:"
echo "   For Ubuntu/Debian WSL2:"
echo "   sudo apt update"
echo "   sudo apt install -y \\"
echo "       pkg-config \\"
echo "       libdbus-1-dev \\"
echo "       libgtk-3-dev \\"
echo "       libsoup2.4-dev \\"
echo "       libjavascriptcoregtk-4.0-dev \\"
echo "       libwebkit2gtk-4.0-dev \\"
echo "       libappindicator3-dev \\"
echo "       librsvg2-dev \\"
echo "       build-essential"

echo
echo "5. Additional recommendations:"
echo "   - Make sure you have the latest WSL2 kernel (wsl --update)"
echo "   - Install latest GPU drivers on Windows host"
echo "   - DO NOT install third-party X servers like VcXsrv"
echo "   - Ensure DISPLAY=:0 is set in your shell profile"

echo
echo "6. Testing GUI support:"
echo "   After installing dependencies, test with:"
echo "   - xclock (simple X11 test)"
echo "   - glxgears (OpenGL test)"
echo "   - cargo tauri dev (run your Tauri app)"