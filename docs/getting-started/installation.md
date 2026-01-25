# Installation

This guide covers how to install Kuse Cowork on your system.

## System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| **OS** | macOS 10.15+, Windows 10+, Linux (Ubuntu 20.04+) |
| **RAM** | 4 GB |
| **Disk** | 500 MB free space |
| **Docker** | Docker Desktop or Docker Engine |

### Recommended Requirements

| Component | Requirement |
|-----------|-------------|
| **RAM** | 8 GB+ (16 GB for local models) |
| **Disk** | 2 GB+ (more for local models) |
| **GPU** | NVIDIA GPU with CUDA (for local models) |

## Installation Methods

###  Build from Source

#### Prerequisites

::: code-group

```bash [macOS]
# Install Homebrew if not installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install node pnpm rust

# Install Tauri CLI
cargo install tauri-cli
```

```bash [Linux]
# Install dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# Install pnpm
npm install -g pnpm

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Tauri CLI
cargo install tauri-cli
```

```powershell [Windows]
# Install via winget or download manually:
# 1. Node.js: https://nodejs.org/ (v18+)
# 2. pnpm: npm install -g pnpm
# 3. Rust: https://rustup.rs/
# 4. Visual Studio Build Tools with C++ workload
# 5. Tauri CLI: cargo install tauri-cli
```

:::

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/kuse-ai/kuse_cowork.git
cd kuse_cowork

# Install dependencies
pnpm install

# Development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Docker Setup

Kuse Cowork uses Docker for secure command execution. Make sure Docker is installed and running:

::: code-group

```bash [macOS]
# Download Docker Desktop from https://www.docker.com/products/docker-desktop
# Start Docker Desktop
# Verify installation:
docker run hello-world
```

```bash [Linux]
# Install Docker Engine
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker

# Verify installation
docker run hello-world
```

```powershell [Windows]
# Download Docker Desktop from https://www.docker.com/products/docker-desktop
# Enable WSL 2 backend (recommended)
# Start Docker Desktop
# Verify installation:
docker run hello-world
```

:::

## Verifying Installation

After installation, launch Kuse Cowork and verify:

1. The application opens without errors
2. Docker status shows "Connected" in settings
3. You can configure at least one AI provider

## Troubleshooting

### Application won't start on macOS

If you see "App is damaged and can't be opened", run:

```bash
xattr -cr /Applications/Kuse_Cowork.app
```

### Docker connection failed

1. Ensure Docker Desktop is running
2. Check Docker socket permissions (Linux):
   ```bash
   sudo chmod 666 /var/run/docker.sock
   ```
3. Restart Kuse Cowork

### Build fails on Windows

Ensure Visual Studio Build Tools are installed with "C++ build tools" workload.

## Next Steps

- [Quick Start Guide](quickstart.md) - Get started with your first task
- [Configuration](configuration.md) - Configure API keys and settings
