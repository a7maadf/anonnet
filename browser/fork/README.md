# AnonNet Browser Fork

A custom Firefox ESR browser fork with Tor Browser hardening, exclusively designed for the AnonNet network.

## Architecture

This is a **Firefox ESR-based browser** with:
- Tor Browser's privacy patches and hardening
- Hardcoded AnonNet proxy configuration (cannot be disabled)
- Built-in credit monitoring extension (system add-on)
- Custom branding and identity
- Automatic updates
- .anon-only browsing (clearnet blocked)

## Why Fork Instead of Configuration?

While our launcher + extension approach works, a proper fork provides:

1. **Immutable Security**: Users cannot disable proxy or security settings
2. **Professional Branding**: Custom name, icon, splash screen
3. **System Integration**: Proper desktop integration, file associations
4. **Update Control**: Manage security updates independently
5. **Trust Signal**: Users know this is purpose-built for AnonNet

## Build System

We use a **hybrid approach** for maintainability:

```
Firefox ESR (Mozilla)
  └── Apply Tor Browser patches (from Tor Project)
      └── Apply AnonNet customizations
          └── Bundle system add-ons
              └── Package for distribution
```

### Key Modifications

1. **Branding** (`branding/`)
   - Custom icons (16, 32, 48, 64, 128, 256px)
   - Splash screen and about dialog
   - Product name: "AnonNet Browser"
   - User agent customization

2. **AutoConfig** (`autoconfig/`)
   - Hardcoded proxy settings (locked)
   - Security preferences (immutable)
   - API endpoint configuration
   - Cannot be overridden by user

3. **System Add-ons** (`system-addons/`)
   - Credit monitoring extension (bundled)
   - Cannot be disabled or removed
   - Loaded before any user add-ons

4. **Update System** (`updater/`)
   - Check for browser updates
   - Check for daemon updates
   - Automatic security patches
   - Version compatibility checking

## Directory Structure

```
fork/
├── branding/              # Custom branding assets
│   ├── icons/             # All icon sizes
│   ├── content/           # About dialog, splash
│   ├── locales/           # Localized strings
│   └── configure.sh       # Branding config script
├── build/                 # Build scripts
│   ├── mozconfig          # Firefox build configuration
│   ├── build.sh           # Main build script
│   ├── apply-patches.sh   # Apply Tor Browser patches
│   └── package.sh         # Create distribution packages
├── autoconfig/            # Locked preferences
│   ├── autoconfig.js      # AutoConfig loader
│   └── anonnet.cfg        # Locked preferences
├── system-addons/         # Bundled extensions
│   └── anonnet-monitor/   # Credit monitoring (from ../extension)
├── updater/               # Update mechanism
│   ├── update-manifest.json
│   ├── check-updates.js
│   └── apply-update.sh
├── packaging/             # Distribution packages
│   ├── linux/             # .deb, .rpm, .tar.gz
│   ├── macos/             # .dmg
│   └── windows/           # .exe installer
└── README.md              # This file
```

## Building

### Prerequisites

**Linux:**
```bash
sudo apt install build-essential python3 nodejs yasm rustc cargo \
  libgtk-3-dev libdbus-glib-1-dev libpulse-dev \
  libx11-xcb-dev libxt-dev
```

**macOS:**
```bash
brew install python nodejs yasm rust
xcode-select --install
```

**Windows:**
- Visual Studio 2022
- Windows SDK
- Rust toolchain
- MozillaBuild environment

### Quick Build

```bash
cd browser/fork
./build/build.sh
```

This will:
1. Download Firefox ESR source
2. Download Tor Browser patches
3. Apply patches
4. Apply AnonNet customizations
5. Build browser
6. Package for distribution

Build time: ~1-2 hours on modern hardware

### Custom Build

```bash
# Build specific version
./build/build.sh --firefox-version 128.6.0esr

# Build without optimization (faster, for testing)
./build/build.sh --debug

# Build for specific platform
./build/build.sh --platform linux

# Skip Tor Browser patches (just use Firefox ESR)
./build/build.sh --no-tor-patches
```

## Maintenance Schedule

**Monthly:**
- Check for Firefox ESR security updates
- Rebuild with latest patches
- Test with current anonnet daemon
- Update dependencies

**Annually (Major ESR Updates):**
- Migrate to new Firefox ESR version (e.g., 128 → 140)
- Re-apply all Tor Browser patches
- Test extensively with network
- Update build scripts as needed

**As Needed:**
- Critical security patches (within 24 hours)
- AnonNet daemon API changes
- User-reported bugs

## Version Numbering

```
AnonNet Browser 1.0.0
                 │ │ │
                 │ │ └─ Patch (bug fixes, minor updates)
                 │ └─── Minor (feature additions, improvements)
                 └───── Major (ESR version changes, breaking changes)
```

Example timeline:
- `1.0.0` - Initial release (Firefox ESR 128)
- `1.1.0` - Added circuit visualization
- `1.1.1` - Fixed credit display bug
- `2.0.0` - Migrated to Firefox ESR 140

## Testing

### Automated Tests

```bash
# Run all tests
./build/test.sh

# Test specific component
./build/test.sh --component proxy
./build/test.sh --component extension
./build/test.sh --component security
```

### Manual Test Checklist

- [ ] Browser launches successfully
- [ ] Proxy is hardcoded (cannot be changed in settings)
- [ ] Extension is installed and cannot be removed
- [ ] Clearnet sites are blocked
- [ ] .anon domains resolve correctly
- [ ] Credit balance displays correctly
- [ ] Network status updates in real-time
- [ ] Low credit warnings appear
- [ ] Updates work correctly
- [ ] About dialog shows correct branding

### Security Audit

Before each release:
1. Run `./build/security-audit.sh`
2. Verify all Tor Browser patches applied
3. Check for known vulnerabilities
4. Test fingerprinting resistance
5. Verify DNS leak protection

## Distribution

### Signing Releases

```bash
# Generate signing key (once)
gpg --full-generate-key

# Sign release
./packaging/sign-release.sh anonnet-browser-1.0.0-linux-x86_64.tar.gz
```

### Publishing

1. Build for all platforms
2. Run security audit
3. Sign all packages
4. Generate checksums
5. Upload to GitHub releases
6. Update update manifest
7. Announce on project channels

### Channels

- **Stable**: Tested, production-ready releases
- **Beta**: Pre-release testing (1-2 weeks before stable)
- **Nightly**: Daily builds from main branch (for developers)

## Comparison with Tor Browser

| Feature | Tor Browser | AnonNet Browser |
|---------|-------------|-----------------|
| **Base** | Firefox ESR | Firefox ESR |
| **Patches** | Tor Project patches | Tor patches + AnonNet |
| **Network** | Tor (.onion) | AnonNet (.anon) |
| **Proxy** | User-configurable | Hardcoded, immutable |
| **Extensions** | User can add | System add-on only |
| **Updates** | Tor Project | Self-hosted |
| **Clearnet** | Yes (via exit nodes) | No (blocked) |
| **Credits** | N/A | Built-in monitoring |
| **Maintenance** | Tor Project | Us (community) |

## Resources

- **Firefox ESR Releases**: https://www.mozilla.org/en-US/firefox/enterprise/
- **Tor Browser Design**: https://gitlab.torproject.org/tpo/applications/tor-browser
- **Tor Browser Patches**: https://gitlab.torproject.org/tpo/applications/tor-browser/-/tree/main/browser
- **Firefox Build Docs**: https://firefox-source-docs.mozilla.org/setup/
- **MozillaBuild**: https://wiki.mozilla.org/MozillaBuild

## Troubleshooting

### Build Fails

```bash
# Clean build directory
./build/clean.sh

# Update build tools
./build/update-tools.sh

# Check logs
cat build/build.log
```

### Patches Don't Apply

This usually happens after a Firefox ESR update.

```bash
# List failed patches
./build/list-failed-patches.sh

# Manually resolve conflicts
vim path/to/conflicted/file
./build/continue.sh
```

### Binary Won't Run

```bash
# Check dependencies
ldd ./dist/anonnet-browser

# Check for missing libraries
./build/check-deps.sh

# Run in debug mode
./dist/anonnet-browser --debug
```

## Contributing

We welcome contributions!

### Code Changes

1. Fork the repository
2. Create feature branch
3. Test thoroughly
4. Submit PR

### Reporting Issues

When reporting bugs, include:
- AnonNet Browser version
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Screenshots if relevant

### Security Issues

**DO NOT** file public issues for security vulnerabilities.

Email: security@anonnet.org (or private disclosure)

## License

AnonNet Browser inherits licenses from:
- **Firefox ESR**: Mozilla Public License 2.0
- **Tor Browser patches**: 3-clause BSD
- **AnonNet modifications**: MIT/Apache-2.0

See individual `LICENSE` files in each directory.

## Disclaimer

This is experimental software. Do not use for activities requiring strong anonymity guarantees.

For maximum security, use:
- **Official Tor Browser** for general anonymity
- **AnonNet Browser** only for accessing .anon services

**The browser is only as secure as the network it runs on.**

## Acknowledgments

- **Mozilla Foundation** for Firefox
- **Tor Project** for Tor Browser and privacy patches
- **AnonNet contributors** for the network implementation

---

**Last Updated**: 2025-11-21
**Maintained By**: AnonNet Project
**Status**: Active Development
