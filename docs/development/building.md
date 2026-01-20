# Building

This guide covers building Kuse Cowork for distribution.

## Build Requirements

Ensure you have all prerequisites from [Development Setup](setup.md).

## Development Build

```bash
# Quick development build
pnpm tauri build --debug
```

Output: `src-tauri/target/debug/`

## Production Build

### Local Build

```bash
# Full production build
pnpm tauri build
```

Output locations:
- **macOS**: `src-tauri/target/release/bundle/dmg/`
- **Windows**: `src-tauri/target/release/bundle/msi/`
- **Linux**: `src-tauri/target/release/bundle/deb/`

### Platform-Specific Builds

::: code-group

```bash [macOS]
# Build for current architecture
pnpm tauri build

# Build for specific architecture
pnpm tauri build --target aarch64-apple-darwin  # Apple Silicon
pnpm tauri build --target x86_64-apple-darwin   # Intel

# Universal binary (both architectures)
pnpm tauri build --target universal-apple-darwin

# Output formats:
# .app - Application bundle
# .dmg - Disk image
# .pkg - Installer package
```

```bash [Windows]
# Build for Windows
pnpm tauri build

# Build specific format
pnpm tauri build --bundles msi
pnpm tauri build --bundles nsis

# Output formats:
# .exe - Executable
# .msi - MSI installer
# *-setup.exe - NSIS installer
```

```bash [Linux]
# Build for Linux
pnpm tauri build

# Build specific format
pnpm tauri build --bundles deb
pnpm tauri build --bundles appimage
pnpm tauri build --bundles rpm

# Output formats:
# .deb - Debian package
# .rpm - Red Hat package
# .AppImage - Universal Linux app
```

:::

## Build Configuration

### Tauri Config

`src-tauri/tauri.conf.json`:

```json
{
  "build": {
    "beforeBuildCommand": "pnpm build",
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  "bundle": {
    "active": true,
    "category": "DeveloperTool",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "identifier": "com.kuse-cowork.app",
    "targets": "all"
  },
  "productName": "Kuse Cowork",
  "version": "0.1.0"
}
```

### Cargo Release Profile

`src-tauri/Cargo.toml`:

```toml
[profile.release]
lto = true
opt-level = "s"
codegen-units = 1
panic = "abort"
strip = true
```

## Cross-Compilation

### From macOS

```bash
# Build for Windows (requires mingw)
brew install mingw-w64
rustup target add x86_64-pc-windows-gnu
pnpm tauri build --target x86_64-pc-windows-gnu
```

### From Linux

```bash
# Build for Windows (requires mingw)
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
pnpm tauri build --target x86_64-pc-windows-gnu
```

## CI/CD Build

### GitHub Actions

`.github/workflows/build.yml`:

```yaml
name: Build

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Setup pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install dependencies (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev

      - name: Install frontend dependencies
        run: pnpm install

      - name: Build
        run: pnpm tauri build --target ${{ matrix.target }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: src-tauri/target/${{ matrix.target }}/release/bundle/
```

## Code Signing

### macOS

1. Get Apple Developer certificate
2. Export certificate to `.p12`
3. Set environment variables:

```bash
export APPLE_CERTIFICATE="base64-encoded-p12"
export APPLE_CERTIFICATE_PASSWORD="password"
export APPLE_SIGNING_IDENTITY="Developer ID Application: ..."
```

4. Build with signing:

```bash
pnpm tauri build
```

### Windows

1. Get code signing certificate (EV recommended)
2. Configure in `tauri.conf.json`:

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "YOUR_THUMBPRINT",
      "digestAlgorithm": "sha256"
    }
  }
}
```

## Notarization

### macOS Notarization

1. Configure Apple credentials:

```bash
export APPLE_ID="your@email.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAM_ID"
```

2. Enable in config:

```json
{
  "bundle": {
    "macOS": {
      "entitlements": "entitlements.plist",
      "signingIdentity": "-"
    }
  }
}
```

## Build Optimization

### Minimize Binary Size

```toml
# Cargo.toml
[profile.release]
lto = true           # Link-time optimization
opt-level = "s"      # Optimize for size
strip = true         # Strip symbols
codegen-units = 1    # Better optimization
```

### Faster Builds

```toml
# For development
[profile.dev]
opt-level = 0
debug = true

# For CI
[profile.release]
lto = "thin"         # Faster than full LTO
codegen-units = 16   # Faster compilation
```

## Troubleshooting

### Build fails with missing libraries

**Linux:**

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev
```

**macOS:**

```bash
xcode-select --install
```

### Code signing fails

1. Verify certificate is valid
2. Check keychain access
3. Ensure correct identity name

### Build is too large

1. Enable LTO in release profile
2. Use `opt-level = "s"` or `"z"`
3. Strip debug symbols

### Cross-compilation fails

1. Install target: `rustup target add <target>`
2. Install cross-compiler toolchain
3. Check linker configuration

## Versioning

Update version in multiple places:

1. `package.json`:
```json
{ "version": "1.0.0" }
```

2. `src-tauri/Cargo.toml`:
```toml
version = "1.0.0"
```

3. `src-tauri/tauri.conf.json`:
```json
{ "version": "1.0.0" }
```

## Release Checklist

- [ ] Update version numbers
- [ ] Update CHANGELOG.md
- [ ] Run all tests
- [ ] Build for all platforms
- [ ] Test built applications
- [ ] Sign binaries
- [ ] Notarize macOS build
- [ ] Create GitHub release
- [ ] Upload artifacts
- [ ] Update documentation

## Next Steps

- [Development Setup](setup.md)
- [Contributing](contributing.md)
- [Architecture](../architecture/overview.md)
