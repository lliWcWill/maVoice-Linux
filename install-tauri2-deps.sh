#!/bin/bash
# Install Tauri 2 dependencies for maVoice

echo "Installing Tauri 2 dependencies..."
echo "This will install libsoup-3.0-dev and webkit2gtk-4.1"

# Update package list
sudo apt update

# Install the correct dependencies for Tauri 2
sudo apt install -y \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev \
    libwebkit2gtk-4.1-dev

echo "✅ Dependencies installed!"
echo "Now try running: npm run dev"