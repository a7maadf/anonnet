# AnonNet Browser

A hardened Firefox browser pre-configured for anonymous browsing on the AnonNet network, featuring Tor Browser's privacy and security hardening.

## Quick Start

### Linux / macOS
```bash
./scripts/launch-anonnet-browser.sh
```

### Windows
```batch
scripts\launch-anonnet-browser.bat
```

## What You Get

✅ **Tor Browser Hardening**: All privacy features from Tor Browser
✅ **Zero Configuration**: Auto-configured proxy settings
✅ **Fingerprinting Protection**: Comprehensive anti-fingerprinting
✅ **IP Leak Prevention**: WebRTC, DNS, and IPv6 leak protection
✅ **Privacy by Default**: First-party isolation, strict tracking protection
✅ **.anon Support**: Native support for AnonNet hidden services

## Features

### Privacy & Security
- **Resist Fingerprinting (RFP)**: Makes all users look identical
- **First-Party Isolation**: Cookies and storage isolated per domain
- **Canvas Fingerprinting**: Blocked
- **WebGL**: Disabled (prevents GPU fingerprinting)
- **WebRTC**: Disabled (prevents IP leaks)
- **Audio API**: Disabled (prevents audio fingerprinting)
- **Geolocation**: Disabled
- **Telemetry**: Completely disabled

### Network Security
- **SOCKS5 Proxy**: Pre-configured (127.0.0.1:9050)
- **DNS over SOCKS**: All DNS queries through proxy
- **IPv6**: Disabled (prevents leaks)
- **HTTPS-Only**: Forces encrypted connections
- **No DNS Prefetch**: Prevents DNS leaks
- **No Link Prefetch**: Prevents connection leaks

### Anti-Tracking
- **Enhanced Tracking Protection**: Strict mode
- **Third-Party Cookies**: Blocked
- **Referer Header**: Limited to same-origin
- **Service Workers**: Disabled
- **Clear on Shutdown**: All data cleared when closing

## Directory Structure

```
browser/
├── profile/
│   └── user.js              # Hardened Firefox configuration (700+ settings)
├── scripts/
│   ├── launch-anonnet-browser.sh   # Linux/macOS launcher
│   └── launch-anonnet-browser.bat  # Windows launcher
├── docs/
│   └── BROWSER_INTEGRATION.md      # Comprehensive documentation
└── README.md                        # This file
```

## Requirements

- **Firefox** (v115+ recommended, or Firefox ESR)
- **AnonNet daemon** (built with `cargo build --release`)
- **Operating System**: Linux, macOS, or Windows

## Installation

### Option 1: Automatic (Recommended)

The launcher script handles everything:

```bash
# Linux/macOS
./scripts/launch-anonnet-browser.sh

# Windows
scripts\launch-anonnet-browser.bat
```

This will:
1. Find/verify Firefox installation
2. Build AnonNet daemon if needed
3. Start AnonNet proxy (ports 9050, 8118)
4. Create hardened Firefox profile
5. Launch browser

### Option 2: Manual

1. **Start AnonNet daemon:**
   ```bash
   cargo run --release --bin anonnet-daemon proxy
   ```

2. **Find your Firefox profile:**
   - Linux: `~/.mozilla/firefox/[profile]/`
   - macOS: `~/Library/Application Support/Firefox/Profiles/[profile]/`
   - Windows: `%APPDATA%\Mozilla\Firefox\Profiles\[profile]\`

3. **Copy hardening config:**
   ```bash
   cp profile/user.js /path/to/firefox/profile/user.js
   ```

4. **Restart Firefox**

## Usage

### Basic Usage

1. **Launch the browser:**
   ```bash
   ./scripts/launch-anonnet-browser.sh
   ```

2. **Verify proxy connection:**
   - Go to `about:config`
   - Search `network.proxy.socks_port`
   - Should show `9050`

3. **Browse .anon sites:**
   ```
   http://yourservice.anon
   ```

### Testing Your Setup

1. **Check fingerprinting protection:**
   - Visit: https://coveryourtracks.eff.org/
   - Should show "Strong protection against tracking"

2. **Check for leaks:**
   - Visit: https://browserleaks.com/
   - WebRTC should be blocked
   - Canvas should be randomized

3. **Verify proxy:**
   ```bash
   # Daemon should be listening
   netstat -an | grep 9050
   ```

## Configuration

### Security Levels

The default configuration is **maximum privacy**. Some sites may break.

To adjust, edit `profile/user.js` or modify in `about:config`:

**Level 1 - Maximum (Default):**
- All hardening enabled
- Some sites will break
- Maximum anonymity

**Level 2 - Balanced:**
```javascript
// Re-enable in about:config
media.webaudio.enabled = true
dom.indexedDB.enabled = true
```

**Level 3 - Compatibility:**
```javascript
// Additional features
privacy.firstparty.isolate = false
dom.serviceWorkers.enabled = true
```

⚠️ **Warning**: Lower security = less anonymity!

### Proxy Configuration

Default ports:
- **SOCKS5**: 127.0.0.1:9050
- **HTTP**: 127.0.0.1:8118

To change, edit `profile/user.js`:
```javascript
user_pref("network.proxy.socks_port", 9050);  // Your port
```

## Troubleshooting

### Browser Won't Start

```bash
# Specify Firefox path manually
./scripts/launch-anonnet-browser.sh /usr/bin/firefox
```

### Proxy Connection Failed

```bash
# Check daemon is running
ps aux | grep anonnet-daemon

# Restart daemon
killall anonnet-daemon
cargo run --release --bin anonnet-daemon proxy
```

### Settings Not Applied

```bash
# Verify user.js exists
ls -la ~/.anonnet/firefox-profile/user.js

# Check in Firefox: about:config
# Search: privacy.resistFingerprinting
# Should be: true
```

### Sites Breaking

This is **expected**. The hardening is intentionally strict.

**Options:**
1. Use regular Firefox for that site (less secure)
2. Selectively enable features (reduces privacy)
3. Find alternative .anon service

## Comparison: Tor Browser vs AnonNet Browser

| Feature | Tor Browser | AnonNet Browser |
|---------|-------------|-----------------|
| Network | Tor | AnonNet |
| Hardening | ✅ | ✅ |
| NoScript | ✅ Built-in | ❌ Add manually |
| HTTPS Everywhere | ✅ Built-in | ✅ HTTPS-only mode |
| Clearnet Access | ✅ | ❌ Blocked |
| Hidden Services | .onion | .anon |
| Updates | Auto | Manual |

## Security Considerations

### Protected Against
✅ Browser fingerprinting
✅ Cookie tracking
✅ Cross-site tracking
✅ WebRTC IP leaks
✅ DNS leaks
✅ IPv6 leaks

### NOT Protected Against
❌ Global network surveillance
❌ Compromised nodes
❌ Browser exploits (keep updated!)
❌ User behavior correlation

### Best Practices
1. ✅ Keep Firefox updated
2. ✅ Don't modify settings
3. ✅ Don't install extensions
4. ✅ Clear data regularly
5. ✅ Use separate instances for different services
6. ❌ Don't login to personal accounts
7. ❌ Don't use same browser for clearnet + .anon

## Advanced Usage

### Multiple Isolated Sessions

```bash
# Session 1
./scripts/launch-anonnet-browser.sh &

# Session 2 (separate profile)
firefox --profile ~/.anonnet/firefox-profile-2 --no-remote &
```

### Custom Extensions (Reduces Anonymity!)

If you must add extensions:
- **uBlock Origin**: Ad/tracker blocking
- **NoScript**: JavaScript control
- **LocalCDN**: CDN privacy

⚠️ Each extension makes you more unique!

## Documentation

- **Full Guide**: [docs/BROWSER_INTEGRATION.md](docs/BROWSER_INTEGRATION.md)
- **AnonNet Docs**: [../README.md](../README.md)
- **Tor Browser Design**: https://2019.www.torproject.org/projects/torbrowser/design/

## Resources

### Testing Tools
- [Cover Your Tracks](https://coveryourtracks.eff.org/) - Fingerprinting test
- [BrowserLeaks](https://browserleaks.com/) - Comprehensive tests
- [IP Leak](https://ipleak.net/) - IP/DNS leak test

### Privacy Resources
- [Tor Browser](https://www.torproject.org/download/)
- [arkenfox user.js](https://github.com/arkenfox/user.js)
- [PrivacyTools](https://www.privacytools.io/)

## Contributing

Found a hardening improvement? Please contribute!

1. Test thoroughly
2. Ensure compatibility
3. Document the benefit
4. Submit PR

## License

Same as AnonNet: Dual MIT/Apache-2.0

## Support

- **Issues**: https://github.com/a7maadf/anonnet/issues
- **Docs**: See [docs/BROWSER_INTEGRATION.md](docs/BROWSER_INTEGRATION.md)

---

**⚠️ Security Notice**: This is experimental software. Do not use for activities requiring strong anonymity guarantees. For maximum security, use official Tor Browser with Tor network.

**✨ Privacy First**: Your privacy is paramount. All settings prioritize anonymity over convenience.
