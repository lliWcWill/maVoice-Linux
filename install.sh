#!/bin/bash

echo "🎙️ maVoice Installation Script"
echo "============================="

# Check if we're in WSL2
if grep -qi microsoft /proc/version 2>/dev/null; then
    echo "✅ WSL2 detected - perfect environment!"
else
    echo "ℹ️  Native Linux detected"
fi

echo ""
echo "📦 Installing all dependencies..."

# Install root dependencies
echo "→ Installing root dependencies..."
npm install

# Install frontend dependencies  
echo "→ Installing frontend dependencies..."
cd src-tauri/aquavoice-frontend && npm install
cd ../../

echo ""
echo "✅ Installation complete!"
echo ""
echo "🚀 Next steps:"
echo "1. Add your Groq API key:"
echo "   echo 'VITE_GROQ_API_KEY=your_key_here' > src-tauri/aquavoice-frontend/.env"
echo ""
echo "2. Run the app:"
echo "   npm run dev"
echo ""
echo "🎯 Look for a tiny 72x20px floating widget at coordinates (300, 800)"
echo "   Right-click + drag to move it"
echo "   Double-click to start recording"