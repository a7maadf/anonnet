# Building AnonNet for macOS

This guide explains how to build AnonNet binaries for macOS (both Intel and Apple Silicon).

## Prerequisites

### 1. Install Xcode Command Line Tools

```bash
xcode-select --install
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Verify Installation

```bash
rustc --version
cargo --version
```

## Building

### Option 1: Use the Build Script (Recommended)

```bash
# Clone the repository
git clone https://github.com/a7maadf/anonnet.git
cd anonnet

# Run the build script
./build-macos.sh
```

The script will:
- Detect your architecture (Apple Silicon or Intel)
- Build optimized release binaries
- Create a distribution package
- Generate checksums

### Option 2: Manual Build

#### For Apple Silicon (M1/M2/M3):

```bash
# Add target
rustup target add aarch64-apple-darwin

# Build
cargo build --release --target aarch64-apple-darwin --bin anonnet-daemon --bin anonweb

# Binaries will be in:
# target/aarch64-apple-darwin/release/anonnet-daemon
# target/aarch64-apple-darwin/release/anonweb
```

#### For Intel (x86_64):

```bash
# Add target
rustup target add x86_64-apple-darwin

# Build
cargo build --release --target x86_64-apple-darwin --bin anonnet-daemon --bin anonweb

# Binaries will be in:
# target/x86_64-apple-darwin/release/anonnet-daemon
# target/x86_64-apple-darwin/release/anonweb
```

## Distribution Package

After building, you'll have:

### For Apple Silicon:
- `dist/anonnet-v1.0-macos-apple-silicon.tar.gz`
- `dist/anonnet-v1.0-macos-apple-silicon.tar.gz.sha256`

### For Intel:
- `dist/anonnet-v1.0-macos-intel.tar.gz`
- `dist/anonnet-v1.0-macos-intel.tar.gz.sha256`

## Running on macOS

### 1. Extract the Package

```bash
tar -xzf anonnet-v1.0-macos-apple-silicon.tar.gz
cd anonnet-v1.0-macos-arm64
```

### 2. Run the Launcher

```bash
./start.sh
```

Or launch the browser directly:

```bash
./browser/scripts/launch-anonnet-browser.sh
```

## macOS-Specific Notes

### Security & Privacy

macOS Gatekeeper may prevent the binaries from running initially. To allow them:

```bash
# Allow the daemon
xattr -d com.apple.quarantine bin/anonnet-daemon

# Allow anonweb
xattr -d com.apple.quarantine bin/anonweb
```

Or go to: **System Settings → Privacy & Security** and click "Allow" when prompted.

### Firewall Configuration

To allow incoming connections (for relay/bootstrap mode):

1. Go to **System Settings → Network → Firewall**
2. Click **Options**
3. Add `anonnet-daemon` to allowed applications
4. Or temporarily disable firewall for testing

### Browser Configuration

#### Firefox Location on macOS:
- Default: `/Applications/Firefox.app/Contents/MacOS/firefox`
- User installed: `~/Applications/Firefox.app/Contents/MacOS/firefox`

The launch script will auto-detect Firefox location.

### Port Requirements

Ensure these ports are accessible:
- **9090**: P2P network (for relay/bootstrap mode)
- **9050**: SOCKS5 proxy (local only)
- **8118**: HTTP proxy (local only)
- **19150+**: REST API (local only)

## Troubleshooting

### "Cannot verify developer" Error

```bash
# Remove quarantine attribute from all binaries
find . -name "anonnet-daemon" -exec xattr -d com.apple.quarantine {} \;
find . -name "anonweb" -exec xattr -d com.apple.quarantine {} \;
```

### Permission Denied

```bash
# Make binaries executable
chmod +x bin/anonnet-daemon
chmod +x bin/anonweb
chmod +x start.sh
```

### Build Fails with OpenSSL Error

```bash
# Install OpenSSL via Homebrew
brew install openssl@3

# Set environment variables
export OPENSSL_DIR=$(brew --prefix openssl@3)
export PKG_CONFIG_PATH="$OPENSSL_DIR/lib/pkgconfig"

# Rebuild
cargo clean
cargo build --release
```

### "xcrun: error" During Build

```bash
# Reinstall Command Line Tools
sudo rm -rf /Library/Developer/CommandLineTools
xcode-select --install
```

## Universal Binary (Intel + Apple Silicon)

To create a universal binary that works on both architectures:

```bash
# Build for both targets
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Create universal binaries using lipo
lipo -create \
  target/aarch64-apple-darwin/release/anonnet-daemon \
  target/x86_64-apple-darwin/release/anonnet-daemon \
  -output anonnet-daemon-universal

lipo -create \
  target/aarch64-apple-darwin/release/anonweb \
  target/x86_64-apple-darwin/release/anonweb \
  -output anonweb-universal

# Verify
lipo -info anonnet-daemon-universal
# Should show: Architectures in the fat file: anonnet-daemon-universal are: x86_64 arm64
```

## Performance Notes

### Apple Silicon Performance
Apple Silicon Macs (M1/M2/M3) provide excellent performance for AnonNet:
- **2-3x faster** circuit creation vs Intel
- **Lower power consumption** when running as relay
- **Better thermal management** for 24/7 bootstrap nodes

### Recommended Configurations

**For Browsing (Client Mode):**
- Any Mac with 4GB+ RAM
- macOS 11.0 or later

**For Relay Node:**
- 8GB+ RAM recommended
- SSD storage
- Good network connection

**For Bootstrap Node:**
- 16GB+ RAM
- SSD storage
- Static IP or reliable DDNS
- 24/7 uptime capability

## Building for Distribution

If you're building for public distribution:

1. **Code signing** (optional but recommended):
   ```bash
   codesign --force --deep --sign "Developer ID Application: Your Name" bin/anonnet-daemon
   ```

2. **Notarization** (required for public distribution):
   - Requires Apple Developer account ($99/year)
   - See: https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution

3. **Create DMG** (user-friendly distribution):
   ```bash
   # Install create-dmg
   brew install create-dmg

   # Create DMG
   create-dmg \
     --volname "AnonNet v1.0" \
     --window-pos 200 120 \
     --window-size 600 400 \
     --icon-size 100 \
     --app-drop-link 425 120 \
     "AnonNet-v1.0.dmg" \
     "dist/anonnet-v1.0-macos-arm64/"
   ```

## Support

For build issues or questions:
- GitHub Issues: https://github.com/a7maadf/anonnet/issues
- Check existing issues before creating new ones
- Include macOS version and architecture in bug reports

## Bootstrap Configuration

The binaries include the production bootstrap node pre-configured:
- **37.114.50.194:9090**

No manual configuration needed!
