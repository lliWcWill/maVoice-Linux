#!/bin/bash

echo "🔧 Setting up AquaVoice GTK4 dependencies..."

# Install required system packages
echo "📦 Installing system dependencies..."
sudo apt update
sudo apt install -y libgtk-4-dev pkg-config xclip libxtst-dev

echo "🔑 Setting up Groq API key..."
echo "Please set your GROQ_API_KEY environment variable:"
echo "export GROQ_API_KEY=\"your-groq-api-key-here\""
echo ""

echo "✅ Setup complete! Now you can run:"
echo "cd /home/player3vsgpt/Desktop/Projects/AquaVoice-Linux/src-gtk"
echo "cargo run"