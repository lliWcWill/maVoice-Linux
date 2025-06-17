# AquaVoice-Linux Development Container
# Rust + Tauri + Audio libs + System deps for Debian
FROM rust:1.75-bookworm as builder

# Install system dependencies for Tauri and audio
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libgtk-3-dev \
    libglib2.0-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libatk1.0-dev \
    libgdk-pixbuf2.0-dev \
    libsoup2.4-dev \
    libjavascriptcoregtk-4.1-dev \
    # Audio dependencies
    libasound2-dev \
    libpulse-dev \
    libjack-jackd2-dev \
    portaudio19-dev \
    # System automation deps  
    xdotool \
    wl-clipboard \
    xclip \
    pkg-config \
    build-essential \
    curl \
    wget \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js for frontend
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

# Install Tauri CLI
RUN cargo install tauri-cli --version "^2.0.0" --locked

# Set working directory
WORKDIR /app

# Development stage - mount source code
FROM builder as dev
EXPOSE 1420 1430
CMD ["tail", "-f", "/dev/null"]

# Production build stage
FROM builder as prod
COPY . .
RUN npm install && npm run tauri build

# Runtime stage
FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-0 \
    libgtk-3-0 \
    libasound2 \
    libpulse0 \
    xdotool \
    wl-clipboard \
    xclip \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=prod /app/src-tauri/target/release/bundle/deb/*.deb /tmp/
RUN dpkg -i /tmp/*.deb || apt-get install -f -y

CMD ["aquavoice-linux"]