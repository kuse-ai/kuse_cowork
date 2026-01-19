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

### Method 1: Download Pre-built Binaries

The easiest way to install Kuse Cowork is to download pre-built binaries from GitHub Releases.

1. Go to [GitHub Releases](https://github.com/kuse-cowork/kuse_cowork/releases)
2. Download the appropriate file for your platform:
   - **macOS**: `kuse-cowork_x.x.x_aarch64.dmg` (Apple Silicon) or `kuse-cowork_x.x.x_x64.dmg` (Intel)
   - **Windows**: `kuse-cowork_x.x.x_x64-setup.exe`
   - **Linux**: `kuse-cowork_x.x.x_amd64.deb` or `kuse-cowork_x.x.x_amd64.AppImage`
3. Install the application

### Method 2: Build from Source

#### Prerequisites

Before building from source, ensure you have:

=== "macOS"

    ```bash
    # Install Homebrew if not installed
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Install dependencies
    brew install node pnpm rust

    # Install Tauri CLI
    cargo install tauri-cli
    ```

=== "Windows"

    1. Install [Node.js](https://nodejs.org/) (v18+)
    2. Install [pnpm](https://pnpm.io/installation): `npm install -g pnpm`
    3. Install [Rust](https://rustup.rs/)
    4. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
    5. Install Tauri CLI: `cargo install tauri-cli`

=== "Linux"

    ```bash
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

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/kuse-cowork/kuse_cowork.git
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

=== "macOS"

    1. Download and install [Docker Desktop for Mac](https://www.docker.com/products/docker-desktop)
    2. Start Docker Desktop
    3. Verify installation: `docker run hello-world`

=== "Windows"

    1. Download and install [Docker Desktop for Windows](https://www.docker.com/products/docker-desktop)
    2. Enable WSL 2 backend (recommended)
    3. Start Docker Desktop
    4. Verify installation: `docker run hello-world`

=== "Linux"

    ```bash
    # Install Docker Engine
    curl -fsSL https://get.docker.com -o get-docker.sh
    sudo sh get-docker.sh

    # Add user to docker group
    sudo usermod -aG docker $USER
    newgrp docker

    # Verify installation
    docker run hello-world
    ```

## Verifying Installation

After installation, launch Kuse Cowork and verify:

1. The application opens without errors
2. Docker status shows "Connected" in settings
3. You can configure at least one AI provider

## Troubleshooting

### Common Issues

??? question "Application won't start on macOS"

    If you see "App is damaged and can't be opened", run:
    ```bash
    xattr -cr /Applications/Kuse\ Cowork.app
    ```

??? question "Docker connection failed"

    1. Ensure Docker Desktop is running
    2. Check Docker socket permissions (Linux):
       ```bash
       sudo chmod 666 /var/run/docker.sock
       ```
    3. Restart Kuse Cowork

??? question "Build fails on Windows"

    Ensure Visual Studio Build Tools are installed with "C++ build tools" workload.

## Next Steps

- [Quick Start Guide](quickstart.md) - Get started with your first task
- [Configuration](configuration.md) - Configure API keys and settings
