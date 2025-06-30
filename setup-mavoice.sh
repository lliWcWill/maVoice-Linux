#!/bin/bash
# maVoice Setup Script
# This script helps set up maVoice on various Linux environments

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ASCII Art Banner
echo -e "${BLUE}"
cat << "EOF"
 _ __ ___   __ ___   ____ (_) ___ ___
| '_ ` _ \ / _` \ \ / / _ \| |/ __/ _ \
| | | | | | (_| |\ V / (_) | | (_|  __/
|_| |_| |_|\__,_| \_/ \___/|_|\___\___|

Open-Source Voice Dictation Setup
EOF
echo -e "${NC}"

# Detect OS
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        VER=$VERSION_ID
    else
        echo -e "${RED}Cannot detect OS. Please install manually.${NC}"
        exit 1
    fi
}

# Check if running in WSL
check_wsl() {
    if grep -qi microsoft /proc/version; then
        echo -e "${YELLOW}🪟 WSL2 detected!${NC}"
        IS_WSL=true
        
        # Check WSLg
        if [ -n "$DISPLAY" ] && [ -n "$WAYLAND_DISPLAY" ]; then
            echo -e "${GREEN}✅ WSLg is properly configured${NC}"
        else
            echo -e "${RED}❌ WSLg may not be configured properly${NC}"
            echo "Please run 'wsl --update' from Windows PowerShell as Administrator"
        fi
    else
        IS_WSL=false
    fi
}

# Check display server
check_display() {
    if [ -n "$WAYLAND_DISPLAY" ]; then
        echo -e "${GREEN}✅ Wayland display detected: $WAYLAND_DISPLAY${NC}"
        DISPLAY_SERVER="wayland"
    elif [ -n "$DISPLAY" ]; then
        echo -e "${GREEN}✅ X11 display detected: $DISPLAY${NC}"
        DISPLAY_SERVER="x11"
    else
        echo -e "${RED}❌ No display server detected${NC}"
        DISPLAY_SERVER="none"
    fi
}

# Check prerequisites
check_prereqs() {
    echo -e "\n${BLUE}Checking prerequisites...${NC}"
    
    # Node.js
    if command -v node >/dev/null 2>&1; then
        NODE_VERSION=$(node --version)
        echo -e "${GREEN}✅ Node.js: $NODE_VERSION${NC}"
    else
        echo -e "${RED}❌ Node.js: Not installed${NC}"
        MISSING_PREREQS+=("nodejs")
    fi
    
    # Rust
    if command -v rustc >/dev/null 2>&1; then
        RUST_VERSION=$(rustc --version | cut -d' ' -f2)
        echo -e "${GREEN}✅ Rust: $RUST_VERSION${NC}"
    else
        echo -e "${RED}❌ Rust: Not installed${NC}"
        MISSING_PREREQS+=("rust")
    fi
    
    # Cargo
    if command -v cargo >/dev/null 2>&1; then
        echo -e "${GREEN}✅ Cargo: Installed${NC}"
    else
        echo -e "${RED}❌ Cargo: Not installed${NC}"
        MISSING_PREREQS+=("cargo")
    fi
}

# Install Node.js
install_nodejs() {
    echo -e "\n${BLUE}Installing Node.js...${NC}"
    curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
    sudo apt-get install -y nodejs
}

# Install Rust
install_rust() {
    echo -e "\n${BLUE}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
}

# Install system dependencies
install_deps() {
    echo -e "\n${BLUE}Installing system dependencies...${NC}"
    
    case $OS in
        ubuntu|debian)
            sudo apt update
            sudo apt install -y \
                build-essential \
                pkg-config \
                libgtk-3-dev \
                libwebkit2gtk-4.0-dev \
                libsoup2.4-dev \
                libjavascriptcoregtk-4.0-dev \
                libdbus-1-dev \
                libappindicator3-dev \
                librsvg2-dev \
                libasound2-dev \
                xdotool \
                wl-clipboard \
                wtype
            ;;
        fedora)
            sudo dnf install -y \
                gcc \
                pkg-config \
                gtk3-devel \
                webkit2gtk4.0-devel \
                libsoup-devel \
                javascriptcoregtk4.0-devel \
                dbus-devel \
                libappindicator-gtk3-devel \
                librsvg2-devel \
                alsa-lib-devel \
                xdotool \
                wl-clipboard \
                wtype
            ;;
        arch|manjaro)
            sudo pacman -S --needed --noconfirm \
                base-devel \
                gtk3 \
                webkit2gtk \
                libsoup \
                dbus \
                libappindicator-gtk3 \
                librsvg \
                alsa-lib \
                xdotool \
                wl-clipboard \
                wtype
            ;;
        *)
            echo -e "${RED}Unsupported distribution: $OS${NC}"
            echo "Please install dependencies manually"
            exit 1
            ;;
    esac
}

# Setup maVoice
setup_mavoice() {
    echo -e "\n${BLUE}Setting up maVoice...${NC}"
    
    # Install npm dependencies
    echo "Installing npm dependencies..."
    npm install
    
    # Check for .env file
    if [ ! -f "src-tauri/aquavoice-frontend/.env" ]; then
        echo -e "\n${YELLOW}⚠️  No .env file found${NC}"
        echo "Please create one with your Groq API key:"
        echo -e "${GREEN}echo \"VITE_GROQ_API_KEY=your_groq_api_key_here\" > src-tauri/aquavoice-frontend/.env${NC}"
    else
        echo -e "${GREEN}✅ .env file exists${NC}"
    fi
}

# Test GUI
test_gui() {
    if [ "$DISPLAY_SERVER" != "none" ]; then
        echo -e "\n${BLUE}Testing GUI support...${NC}"
        if command -v xclock >/dev/null 2>&1; then
            echo "Running xclock test (close the window to continue)..."
            timeout 5 xclock 2>/dev/null || true
            echo -e "${GREEN}✅ GUI test completed${NC}"
        fi
    fi
}

# Main installation flow
main() {
    echo -e "${BLUE}Starting maVoice setup...${NC}\n"
    
    # Checks
    detect_os
    check_wsl
    check_display
    check_prereqs
    
    # Install missing prerequisites
    if [ ${#MISSING_PREREQS[@]} -gt 0 ]; then
        echo -e "\n${YELLOW}Missing prerequisites detected${NC}"
        
        for prereq in "${MISSING_PREREQS[@]}"; do
            case $prereq in
                nodejs)
                    read -p "Install Node.js? (y/n) " -n 1 -r
                    echo
                    if [[ $REPLY =~ ^[Yy]$ ]]; then
                        install_nodejs
                    fi
                    ;;
                rust|cargo)
                    read -p "Install Rust? (y/n) " -n 1 -r
                    echo
                    if [[ $REPLY =~ ^[Yy]$ ]]; then
                        install_rust
                    fi
                    ;;
            esac
        done
    fi
    
    # Install system dependencies
    read -p "Install system dependencies? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        install_deps
    fi
    
    # Setup maVoice
    setup_mavoice
    
    # Test GUI
    test_gui
    
    # Final instructions
    echo -e "\n${GREEN}✅ Setup complete!${NC}"
    echo -e "\n${BLUE}Next steps:${NC}"
    echo "1. Add your Groq API key to .env file"
    echo "2. Run 'npm run dev' to start in development mode"
    echo "3. Look for the tiny floating widget (72x20px) at position x:300, y:800"
    
    if [ "$IS_WSL" = true ]; then
        echo -e "\n${YELLOW}WSL2 Note:${NC}"
        echo "- Make sure WSLg is updated: wsl --update (from Windows)"
        echo "- Audio recording may have limitations in WSL"
        echo "- Consider temporarily increasing widget size in tauri.conf.json"
    fi
    
    echo -e "\n${BLUE}Happy voice dictating! 🎤${NC}"
}

# Run main function
main