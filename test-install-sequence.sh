#!/bin/bash

echo "🧪 TESTING MAVOICE INSTALLATION SEQUENCE"
echo "========================================"

# Create test directory
mkdir -p test-clean-install
cd test-clean-install

echo ""
echo "📥 Step 1: Clone repository..."
git clone https://github.com/Cwilliams333/maVoice-Enhanced.git
cd maVoice-Enhanced

echo ""
echo "📦 Step 2: Test ROOT npm install..."
npm install
echo "✅ Root npm install complete"

echo ""
echo "📦 Step 3: Test FRONTEND npm install..."
cd src-tauri/aquavoice-frontend
npm install
echo "✅ Frontend npm install complete"

echo ""
echo "🎯 Step 4: Test if we can run development..."
cd ../../  # Back to root
echo "Attempting to run 'npm run dev' from root directory..."

echo ""
echo "🔍 ANALYSIS:"
echo "- Root package.json contains: $(cat package.json | grep -A 5 '"dependencies"')"
echo "- Frontend package.json contains React, Vite, etc."
echo ""
echo "📋 CONCLUSION:"
echo "This project requires DUAL npm install:"
echo "1. npm install (in root) - for Tauri CLI"
echo "2. npm install (in src-tauri/aquavoice-frontend) - for React/Vite"
echo "3. npm run dev (from root) - to start both backend + frontend"

echo ""
echo "🚨 The current dual-package structure means users MUST:"
echo "   npm install                                    # Root dependencies"
echo "   cd src-tauri/aquavoice-frontend && npm install # Frontend dependencies"  
echo "   cd ../../ && npm run dev                       # Run from root"