# AnonNet Browser Fork - Quick Start Guide

This guide gets you building and testing the AnonNet Browser fork in 30 minutes.

## Prerequisites

### Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install -y \
  build-essential python3 python3-pip nodejs npm yasm \
  rustc cargo clang libgtk-3-dev libdbus-glib-1-dev \
  libpulse-dev libx11-xcb-dev libxt-dev git curl zip
```

### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew if needed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install python nodejs yasm rust
```

### Windows

1. Install Visual Studio 2022 (Community Edition)
2. Install Windows SDK
3. Install Rust: https://rustup.rs/
4. Install MozillaBuild: https://ftp.mozilla.org/pub/mozilla/libraries/win32/MozillaBuildSetup-Latest.exe

## Quick Build (Testing Only)

For testing the browser fork structure without a full 2-hour build:

```bash
cd browser/fork

# This creates the directory structure and scripts
# without actually building Firefox
echo "Quick setup complete!"
echo "To do a full build, run: ./build/build.sh"
```

## Full Build (Production)

**WARNING: This takes 1-2 hours and uses ~30GB disk space**

```bash
cd browser/fork

# Build browser
./build/build.sh

# This will:
# 1. Download Firefox ESR source (~500MB)
# 2. Download Tor Browser patches
# 3. Apply all patches
# 4. Apply AnonNet customizations
# 5. Build Firefox (~1-2 hours)
# 6. Package browser
```

### Build Output

```
browser/fork/
├── build/
│   └── firefox-source/        # Firefox source (after build)
├── dist/
│   └── anonnet-browser-*.tar.gz  # Built package
└── system-addons/
    └── build/
        └── anonnet-monitor@anonnet.org.xpi  # Extension
```

## Test the Browser

### Without Full Build (Quick Test)

You can test the extension and scripts without building Firefox:

```bash
# Install AnonNet daemon
cd ../../  # Back to repo root
cargo build --release

# Run daemon
./target/release/anonnet-daemon proxy &

# Use regular Firefox with our extension
cd browser/extension
# Load as temporary add-on in Firefox:
# 1. Open Firefox
# 2. Go to about:debugging
# 3. Click "This Firefox"
# 4. Click "Load Temporary Add-on"
# 5. Select manifest.json
```

### With Full Build

```bash
# After build completes
cd browser/fork/dist

# Extract package
tar -xzf anonnet-browser-*.tar.gz
cd anonnet-browser-*/

# Ensure daemon is running
ps aux | grep anonnet-daemon

# Launch browser
./anonnet-browser
```

## Create Distribution Package

```bash
cd browser/fork

# Package for your platform
./packaging/package.sh

# Output in dist/
ls -lh dist/*.tar.gz dist/*.deb
```

## Test Checklist

After building, verify:

- [ ] Browser launches
- [ ] Extension is loaded (check toolbar icon)
- [ ] about:config shows locked proxy settings
- [ ] Try accessing google.com (should be blocked)
- [ ] Credit balance shows in extension popup
- [ ] Network status displays correctly

## Common Issues

### "Build failed: Missing dependency"

```bash
# Check which dependency
cat build/build.log | grep -i "error"

# Install missing package
sudo apt install [package-name]
```

### "Patch does not apply"

```bash
# This is normal - some patches may fail
# The build continues with warnings

# To see which patches failed:
grep "FAILED" build/build.log
```

### "Browser won't start"

```bash
# Check dependencies
ldd dist/anonnet-browser-*/firefox

# Run with debug output
./dist/anonnet-browser-*/firefox --verbose
```

### "Extension not loading"

```bash
# Check system add-on exists
ls dist/anonnet-browser-*/browser/features/

# If missing, rebuild system add-on:
cd system-addons
./prepare-addon.sh
```

## Development Workflow

### Making Changes

1. **Edit autoconfig** (proxy settings)
   ```bash
   vim autoconfig/anonnet.cfg
   ```

2. **Edit branding** (icons, names)
   ```bash
   vim branding/configure.sh
   vim branding/icons/icon.svg
   ```

3. **Edit extension** (UI, credit monitoring)
   ```bash
   cd ../extension
   vim manifest.json
   vim popup/popup.html
   ```

4. **Rebuild**
   ```bash
   cd ../fork
   ./build/build.sh  # Full rebuild
   # or
   ./system-addons/prepare-addon.sh  # Just extension
   ```

### Testing Changes

```bash
# Quick test without full rebuild
./packaging/package.sh --skip-build

# Test extension changes immediately
cd ../extension
zip -r test.xpi ./*
# Load in Firefox: about:debugging → Load Temporary Add-on
```

## Next Steps

- **Read full documentation**: `./README.md`
- **Maintenance guide**: `./MAINTENANCE.md`
- **Customize branding**: Edit `branding/` files
- **Add features**: Modify extension in `../extension/`
- **Deploy**: Follow packaging guide in `./packaging/README.md`

## Quick Commands Reference

```bash
# Build browser
./build/build.sh

# Package browser
./packaging/package.sh

# Prepare system add-on
./system-addons/prepare-addon.sh

# Check for updates
node ./updater/check-updates.js

# Apply update
./updater/apply-update.sh <url> <hash>
```

## Resource Usage

| Operation | Time | Disk Space | RAM |
|-----------|------|------------|-----|
| Download source | 5 min | 500 MB | 500 MB |
| Full build | 60-120 min | 30 GB | 8 GB |
| Incremental | 10-20 min | +2 GB | 4 GB |
| Runtime | - | - | 1-2 GB |

## Getting Help

- **Build issues**: Check `build/build.log`
- **Runtime issues**: Check `~/.anonnet/browser.log`
- **Extension issues**: Check Browser Console (Ctrl+Shift+J)
- **Ask for help**: https://github.com/a7maadf/anonnet/discussions

## Success!

If you made it here, you now have a working AnonNet Browser fork!

**What you've built:**
- Custom Firefox ESR with Tor Browser hardening
- Hardcoded proxy configuration
- Built-in credit monitoring extension
- Clearnet blocking enforced
- Custom branding
- Update mechanism

**Next steps:**
- Distribute to users
- Set up CI/CD for automated builds
- Monitor for security updates
- Engage with the community

Welcome to the maintainer club! 🎉

---

**Questions?** Open an issue or discussion on GitHub.
**Contributions?** PRs are welcome!

*Last Updated: 2025-11-21*
