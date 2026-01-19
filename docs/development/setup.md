# Development Setup

This guide covers setting up a development environment for contributing to Kuse Cowork.

## Prerequisites

### Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | 18+ | Frontend runtime |
| pnpm | 8+ | Package manager |
| Rust | 1.70+ | Backend language |
| Docker | 20+ | Container runtime |

### Platform-Specific Setup

=== "macOS"

    ```bash
    # Install Homebrew
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Install dependencies
    brew install node pnpm rust

    # Install Tauri CLI
    cargo install tauri-cli

    # Install Docker Desktop
    brew install --cask docker
    ```

=== "Windows"

    1. Install [Node.js](https://nodejs.org/) (LTS version)
    2. Install pnpm: `npm install -g pnpm`
    3. Install [Rust](https://rustup.rs/)
    4. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with C++ workload
    5. Install [Docker Desktop](https://www.docker.com/products/docker-desktop)
    6. Install Tauri CLI: `cargo install tauri-cli`

=== "Linux (Ubuntu/Debian)"

    ```bash
    # Install system dependencies
    sudo apt update
    sudo apt install -y \
        libwebkit2gtk-4.1-dev \
        build-essential \
        curl \
        wget \
        libssl-dev \
        libgtk-3-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev

    # Install Node.js
    curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
    sudo apt install -y nodejs

    # Install pnpm
    npm install -g pnpm

    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env

    # Install Tauri CLI
    cargo install tauri-cli

    # Install Docker
    curl -fsSL https://get.docker.com -o get-docker.sh
    sudo sh get-docker.sh
    sudo usermod -aG docker $USER
    ```

## Clone and Setup

```bash
# Clone the repository
git clone https://github.com/kuse-cowork/kuse_cowork.git
cd kuse_cowork

# Install frontend dependencies
pnpm install

# Verify Rust setup
cd src-tauri
cargo check
cd ..
```

## Development Mode

### Start Development Server

```bash
# Start with hot reload
pnpm tauri dev
```

This will:
1. Start the Vite dev server (port 1420)
2. Compile the Rust backend
3. Launch the application with DevTools

### Frontend Only

For faster iteration on UI changes:

```bash
# Start Vite dev server only
pnpm dev
```

Access at `http://localhost:1420`

!!! note "Web Mode Limitations"
    Without Tauri, some features are unavailable:
    - Tool execution
    - Docker integration
    - MCP connections
    - Local file access

### Backend Only

To work on Rust code:

```bash
cd src-tauri

# Check compilation
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## Project Structure

```
kuse_cowork/
├── src/                    # Frontend source
│   ├── App.tsx
│   ├── components/
│   ├── stores/
│   └── lib/
├── src-tauri/              # Backend source
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
├── docs/                   # Documentation
├── public/                 # Static assets
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
└── mkdocs.yml
```

## IDE Setup

### VS Code (Recommended)

Install extensions:
- **rust-analyzer**: Rust language support
- **Tauri**: Tauri development tools
- **SolidJS**: SolidJS snippets and highlighting
- **ESLint**: JavaScript linting
- **Prettier**: Code formatting

Settings (`.vscode/settings.json`):

```json
{
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "esbenp.prettier-vscode",
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

### JetBrains IDEs

- Install Rust plugin
- Install Tauri plugin
- Configure TypeScript for SolidJS JSX

## Environment Variables

Create `.env` for development:

```bash
# .env
VITE_DEV_MODE=true
RUST_LOG=debug
```

## Testing

### Frontend Tests

```bash
# Run tests
pnpm test

# Run with coverage
pnpm test -- --coverage
```

### Backend Tests

```bash
cd src-tauri

# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### E2E Tests

```bash
# Run E2E tests (requires built app)
pnpm tauri build
pnpm test:e2e
```

## Debugging

### Frontend Debugging

1. Open DevTools in the app (Cmd+Option+I / Ctrl+Shift+I)
2. Use Console for logs
3. Use Sources for breakpoints
4. Use Network for API calls

### Backend Debugging

Add logging:

```rust
use log::{debug, info, error};

info!("Starting agent loop");
debug!("Tool use: {:?}", tool_use);
error!("Failed: {}", err);
```

Run with debug logging:

```bash
RUST_LOG=debug pnpm tauri dev
```

### VS Code Debugging

Launch config (`.vscode/launch.json`):

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Tauri",
      "cargo": {
        "args": ["build", "--manifest-path=./src-tauri/Cargo.toml"]
      },
      "preLaunchTask": "ui:dev"
    }
  ]
}
```

## Code Style

### TypeScript/JavaScript

- Use ESLint and Prettier
- Follow SolidJS conventions
- Prefer functional components

```typescript
// Good
const Component: Component<Props> = (props) => {
  const [state, setState] = createSignal(0);
  return <div>{state()}</div>;
};

// Avoid
class Component extends SolidComponent { }
```

### Rust

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow Rust API guidelines

```rust
// Good
pub fn process_item(item: &Item) -> Result<Output, Error> {
    // ...
}

// Avoid
pub fn ProcessItem(item: Item) -> Output { }
```

## Common Tasks

### Add a New Tool

1. Create `src-tauri/src/tools/my_tool.rs`
2. Add to `src-tauri/src/tools/mod.rs`
3. Register in tool executor
4. Add to allowed tools list

### Add a New Provider

1. Add format to `ApiFormat` enum
2. Implement in `LLMClient`
3. Add to provider detection
4. Update frontend settings

### Add a Frontend Component

1. Create `src/components/MyComponent.tsx`
2. Create `src/components/MyComponent.css`
3. Import and use in parent component

## Troubleshooting

??? question "Cargo build fails"

    ```bash
    # Clean and rebuild
    cd src-tauri
    cargo clean
    cargo build
    ```

??? question "pnpm install fails"

    ```bash
    # Clear cache and reinstall
    rm -rf node_modules
    pnpm store prune
    pnpm install
    ```

??? question "Tauri dev crashes"

    1. Check Rust compilation: `cd src-tauri && cargo check`
    2. Check Vite: `pnpm dev`
    3. Check logs for errors

??? question "Docker not working"

    1. Ensure Docker Desktop is running
    2. Check Docker socket: `docker ps`
    3. On Linux: `sudo chmod 666 /var/run/docker.sock`

## Next Steps

- [Contributing Guide](contributing.md)
- [Building](building.md)
- [Architecture](../architecture/overview.md)
