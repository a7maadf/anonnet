# AnonNet Browser Integration

## Overview

AnonNet Browser is a hardened Firefox configuration that integrates Tor Browser's privacy and security features with the AnonNet anonymous network. It provides a complete browsing solution with:

- **Tor Browser hardening**: All major privacy and security features from Tor Browser
- **Automatic proxy configuration**: Pre-configured to use AnonNet's SOCKS5 and HTTP proxies
- **Fingerprinting resistance**: Comprehensive protection against browser fingerprinting
- **Traffic isolation**: First-party isolation and strict tracking protection
- **.anon domain support**: Native support for AnonNet's .anon services

---

## Quick Start

### Linux / macOS

```bash
# 1. Build AnonNet daemon (if not already built)
cd /path/to/anonnet
cargo build --release --bin anonnet-daemon

# 2. Launch AnonNet Browser
./browser/scripts/launch-anonnet-browser.sh

# Or specify Firefox path
./browser/scripts/launch-anonnet-browser.sh /usr/bin/firefox
```

### Windows

```batch
REM 1. Build AnonNet daemon (if not already built)
cd C:\path\to\anonnet
cargo build --release --bin anonnet-daemon

REM 2. Launch AnonNet Browser
browser\scripts\launch-anonnet-browser.bat

REM Or specify Firefox path
browser\scripts\launch-anonnet-browser.bat "C:\Program Files\Mozilla Firefox\firefox.exe"
```

---

## What the Launcher Does

The launcher script automates the entire setup process:

1. **Finds Firefox**: Auto-detects Firefox installation or uses specified path
2. **Builds daemon**: Builds AnonNet daemon if not already compiled
3. **Starts daemon**: Launches AnonNet daemon in proxy mode (ports 9050, 8118)
4. **Creates profile**: Sets up a dedicated Firefox profile at `~/.anonnet/firefox-profile`
5. **Applies hardening**: Copies the hardened `user.js` configuration
6. **Launches browser**: Opens Firefox with the AnonNet-hardened profile

---

## Security Features

### Privacy Protections (from Tor Browser)

| Feature | Status | Description |
|---------|--------|-------------|
| **Fingerprinting Resistance** | ✅ Enabled | `privacy.resistFingerprinting = true` |
| **First-Party Isolation** | ✅ Enabled | Isolates cookies and storage per domain |
| **Canvas Fingerprinting** | ✅ Blocked | Prevents canvas-based tracking |
| **WebGL** | ✅ Disabled | Blocks GPU fingerprinting |
| **WebRTC** | ✅ Disabled | Prevents IP address leaks |
| **Audio API** | ✅ Disabled | Blocks audio fingerprinting |
| **Geolocation** | ✅ Disabled | No location tracking |
| **Battery API** | ✅ Disabled | No battery status leaks |

### Network Security

| Feature | Status | Description |
|---------|--------|-------------|
| **SOCKS5 Proxy** | ✅ Configured | All traffic via 127.0.0.1:9050 |
| **DNS over SOCKS** | ✅ Enabled | DNS queries routed through proxy |
| **IPv6** | ✅ Disabled | Prevents IPv6 leaks |
| **DNS Prefetch** | ✅ Disabled | No DNS prefetching |
| **Link Prefetch** | ✅ Disabled | No link prefetching |
| **HTTPS-Only Mode** | ✅ Enabled | Forces HTTPS connections |

### Anti-Tracking

| Feature | Status | Description |
|---------|--------|-------------|
| **Enhanced Tracking Protection** | ✅ Strict | Maximum tracking protection |
| **Third-Party Cookies** | ✅ Blocked | No cross-site cookies |
| **Referer Header** | ✅ Limited | Only same-origin referers |
| **Do Not Track** | ✅ Enabled | DNT header sent |
| **Service Workers** | ✅ Disabled | Prevents worker-based tracking |

### Data Collection

| Feature | Status | Description |
|---------|--------|-------------|
| **Telemetry** | ✅ Disabled | No data sent to Mozilla |
| **Crash Reports** | ✅ Disabled | No crash data collected |
| **Safebrowsing** | ✅ Disabled | No URL hashing to Google |
| **Pocket** | ✅ Disabled | No Pocket integration |
| **Firefox Sync** | ✅ Disabled | No account sync |

---

## Manual Installation

If you prefer to set up Firefox manually without the launcher:

### Step 1: Find Your Firefox Profile Directory

**Linux:**
```bash
~/.mozilla/firefox/[random-string].default/
```

**macOS:**
```bash
~/Library/Application Support/Firefox/Profiles/[random-string].default/
```

**Windows:**
```
%APPDATA%\Mozilla\Firefox\Profiles\[random-string].default\
```

### Step 2: Copy user.js

```bash
# Copy the hardened configuration
cp browser/profile/user.js /path/to/firefox/profile/user.js
```

### Step 3: Start AnonNet Daemon

```bash
# Start the daemon in proxy mode
cargo run --release --bin anonnet-daemon proxy
```

### Step 4: Restart Firefox

Close and reopen Firefox. The settings in `user.js` will be automatically applied.

### Step 5: Verify Configuration

1. Open Firefox
2. Type `about:config` in the address bar
3. Search for `network.proxy.socks_port`
4. Verify it shows `9050`
5. Search for `privacy.resistFingerprinting`
6. Verify it shows `true`

---

## Configuration Details

### Proxy Settings

The browser is configured to route **all traffic** through AnonNet:

```javascript
// SOCKS5 proxy (primary)
user_pref("network.proxy.type", 1);
user_pref("network.proxy.socks", "127.0.0.1");
user_pref("network.proxy.socks_port", 9050);
user_pref("network.proxy.socks_version", 5);

// DNS through SOCKS (critical!)
user_pref("network.proxy.socks_remote_dns", true);
```

### Hardening Sections

The `user.js` file includes 17 comprehensive hardening sections:

1. **AnonNet Branding & Updates** - Disable auto-updates
2. **Proxy Configuration** - AnonNet SOCKS5/HTTP proxy settings
3. **Privacy & Fingerprinting** - Tor Browser's RFP (Resist Fingerprinting)
4. **DNS & Network Privacy** - DNS leak protection
5. **HTTP & HTTPS Security** - TLS hardening, HTTPS-only mode
6. **Cookies & Storage** - First-party isolation, strict cookie policies
7. **WebRTC & Media** - Disable WebRTC to prevent IP leaks
8. **JavaScript & Web APIs** - Disable dangerous APIs
9. **Location & Sensors** - Block geolocation and sensors
10. **Telemetry & Data Collection** - Disable all telemetry
11. **Search & Suggestions** - Disable search suggestions
12. **Browser Features** - Disable risky features (PDF.js, WASM)
13. **Font & Rendering** - Limit font fingerprinting
14. **UI & UX Hardening** - Minimal browser UI
15. **Download Protection** - Safe download handling
16. **AnonNet Specific** - .anon domain handling
17. **Miscellaneous** - Additional security tweaks

---

## Browser Compatibility

### Supported Browsers

| Browser | Support | Notes |
|---------|---------|-------|
| **Firefox** | ✅ Full | Recommended (v115+ ESR) |
| **Firefox ESR** | ✅ Full | Long-term support release |
| **Librewolf** | ✅ Full | Privacy-focused Firefox fork |
| **Tor Browser** | ⚠️ Partial | Already has hardening, may conflict |
| **Chrome/Chromium** | ❌ No | Different config format |

### Recommended: Firefox ESR

We recommend **Firefox ESR (Extended Support Release)** for:

- Longer stability period
- Less frequent updates
- Better compatibility with hardening settings
- Used as base for Tor Browser

**Installation:**

```bash
# Debian/Ubuntu
sudo apt install firefox-esr

# Fedora
sudo dnf install firefox-esr

# macOS (Homebrew)
brew install --cask firefox-esr

# Windows
# Download from: https://www.mozilla.org/firefox/enterprise/
```

---

## Testing Your Setup

### 1. Verify Proxy Connection

```bash
# The daemon should be listening on both ports
netstat -an | grep LISTEN | grep -E '9050|8118'

# Should show:
# tcp4  0  0  127.0.0.1.9050  *.*  LISTEN
# tcp4  0  0  127.0.0.1.8118  *.*  LISTEN
```

### 2. Test Browser Fingerprinting

Visit these sites to verify your browser hardening:

- **https://coveryourtracks.eff.org/** - EFF's fingerprinting test
  - Should show "Strong protection against tracking"

- **https://browserleaks.com/** - Comprehensive leak testing
  - Check: WebRTC (should be blocked)
  - Check: Canvas (should be randomized)
  - Check: Fonts (should show limited fonts)

- **https://ipleak.net/** - IP and DNS leak test
  - Your real IP should NOT appear anywhere
  - All connections should go through AnonNet

### 3. Test .anon Domain Resolution

Once you have a .anon service running:

```
http://yourservice.anon
```

The browser should:
- Route through AnonNet proxy
- Resolve the .anon address
- Display the service content

### 4. Verify Clearnet Blocking

Try visiting a regular website:

```
https://www.google.com
```

Expected behavior:
- The AnonNet proxy should block this (clearnet protection)
- You'll see a proxy error message

---

## Troubleshooting

### Firefox Won't Start

**Problem:** Script fails to find Firefox

**Solution:**
```bash
# Specify Firefox path manually
./launch-anonnet-browser.sh /usr/bin/firefox

# Or install Firefox
sudo apt install firefox  # Debian/Ubuntu
sudo dnf install firefox  # Fedora
brew install --cask firefox  # macOS
```

### Proxy Connection Failed

**Problem:** "Proxy server is refusing connections"

**Solutions:**
1. **Check daemon is running:**
   ```bash
   ps aux | grep anonnet-daemon
   ```

2. **Restart daemon:**
   ```bash
   killall anonnet-daemon
   cargo run --release --bin anonnet-daemon proxy
   ```

3. **Check ports:**
   ```bash
   lsof -i :9050
   lsof -i :8118
   ```

### Settings Not Applied

**Problem:** Browser doesn't seem hardened

**Solutions:**
1. **Verify user.js exists:**
   ```bash
   ls -la ~/.anonnet/firefox-profile/user.js
   ```

2. **Check it's being read:**
   - Open Firefox
   - Go to `about:config`
   - Search for `privacy.resistFingerprinting`
   - Should be `true`

3. **Force refresh:**
   ```bash
   # Delete prefs.js (Firefox will rebuild it)
   rm ~/.anonnet/firefox-profile/prefs.js

   # Restart Firefox
   ```

### Websites Breaking

**Problem:** Some websites don't work properly

**Explanation:** The hardening is **intentionally strict** and may break functionality on some sites.

**Solutions:**

1. **Temporary fix** - Use a regular Firefox profile for that site
2. **Security levels** - Selectively enable features in `about:config`:
   ```
   javascript.enabled = true  (already enabled)
   webgl.disabled = false  (re-enable WebGL - reduces privacy!)
   media.peerconnection.enabled = true  (re-enable WebRTC - IP leak risk!)
   ```

3. **Site-specific** - Create exceptions for trusted sites only

**Warning:** Modifying these settings reduces your privacy and anonymity!

### .anon Sites Not Loading

**Problem:** Can't access .anon services

**Checklist:**
1. ✅ AnonNet daemon is running
2. ✅ Proxy settings are correct (9050)
3. ✅ DNS is routed through SOCKS (`network.proxy.socks_remote_dns = true`)
4. ✅ The .anon service is actually online
5. ✅ You're connected to AnonNet network (check peer count)

---

## Customization

### Security Levels

You can adjust the security level by modifying `user.js`:

#### Level 1: Maximum Privacy (Default)
- Everything disabled
- Maximum fingerprinting resistance
- Some sites will break

#### Level 2: Balanced
```javascript
// Re-enable these in user.js or about:config
user_pref("media.webaudio.enabled", true);  // Audio APIs
user_pref("dom.indexedDB.enabled", true);   // IndexedDB
user_pref("webgl.disabled", false);         // WebGL (fingerprint risk!)
```

#### Level 3: Compatibility
```javascript
// Additional relaxations
user_pref("privacy.firstparty.isolate", false);  // Breaks some login flows
user_pref("dom.serviceWorkers.enabled", true);   // PWA support
user_pref("dom.storage.enabled", true);          // LocalStorage
```

**Warning:** Lower security levels reduce anonymity!

### Custom User Agent

Edit in `user.js`:

```javascript
// Windows 10 + Firefox 128 (default)
user_pref("general.useragent.override", "Mozilla/5.0 (Windows NT 10.0; rv:128.0) Gecko/20100101 Firefox/128.0");

// Or match your OS
user_pref("general.useragent.override", "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0");
```

### Add Bookmarks

Create a bookmarks file at `~/.anonnet/firefox-profile/bookmarks.html`:

```html
<!DOCTYPE NETSCAPE-Bookmark-file-1>
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks Menu</H1>
<DL><p>
    <DT><H3>AnonNet Services</H3>
    <DL><p>
        <DT><A HREF="http://yourservice.anon">My .anon Service</A>
        <DT><A HREF="http://forum.anon">AnonNet Forum</A>
    </DL><p>
</DL><p>
```

---

## Comparison with Tor Browser

| Feature | Tor Browser | AnonNet Browser |
|---------|-------------|-----------------|
| **Base** | Firefox ESR | Firefox (any version) |
| **Network** | Tor Network | AnonNet |
| **Proxy** | Built-in (9050) | AnonNet daemon (9050) |
| **Fingerprinting Protection** | ✅ RFP enabled | ✅ RFP enabled |
| **NoScript** | ✅ Built-in | ❌ Not included (add manually) |
| **HTTPS Everywhere** | ✅ Built-in | ⚠️ HTTPS-only mode |
| **Clearnet Access** | ✅ Allowed | ❌ Blocked |
| **Hidden Services** | .onion | .anon |
| **Updates** | Automatic | Manual |
| **Branding** | Tor | AnonNet |

### Key Differences

1. **Network**: Tor Browser uses Tor network, AnonNet Browser uses AnonNet
2. **Extensions**: Tor Browser includes NoScript, AnonNet Browser doesn't (you can add it)
3. **Clearnet**: Tor allows clearnet, AnonNet blocks it (by design)
4. **Updates**: Tor has auto-updates, AnonNet uses system Firefox updates

---

## Security Considerations

### Threat Model

**Protected Against:**
- ✅ Browser fingerprinting
- ✅ Cookie tracking
- ✅ Cross-site tracking
- ✅ Canvas fingerprinting
- ✅ WebRTC IP leaks
- ✅ DNS leaks
- ✅ IPv6 leaks
- ✅ Font fingerprinting

**NOT Protected Against:**
- ❌ Global passive adversary (monitoring all network traffic)
- ❌ Compromised AnonNet nodes (same as Tor threat model)
- ❌ Browser exploits (keep Firefox updated!)
- ❌ User behavior correlation (typing style, browsing patterns)

### Best Practices

1. **Keep Firefox Updated**: Security patches are critical
2. **Don't Modify Settings**: Changing settings increases fingerprint uniqueness
3. **Use Full Screen Carefully**: Window size can be a fingerprint
4. **Don't Install Extensions**: Each extension changes your fingerprint
5. **Clear Data Regularly**: Even though it clears on shutdown, clear manually too
6. **Don't Login to Accounts**: Logging in links your identity across sessions
7. **One Site Per Session**: Use separate browser instances for unrelated sites

### Recommended Extensions (Optional)

Only install if absolutely necessary (reduces anonymity set):

- **uBlock Origin**: Additional ad/tracker blocking
- **NoScript**: JavaScript control (similar to Tor Browser)
- **LocalCDN**: Prevents CDN-based tracking

---

## Advanced Usage

### Multiple Profiles

Run multiple isolated browser instances:

```bash
# Create profile 1
./launch-anonnet-browser.sh

# Create profile 2 (in another terminal)
firefox --profile ~/.anonnet/firefox-profile-2 --no-remote &
cp browser/profile/user.js ~/.anonnet/firefox-profile-2/
```

### Circuit Isolation

For maximum privacy, use different browser instances for different services:

```bash
# Instance 1: Service A
./launch-anonnet-browser.sh &

# Instance 2: Service B
firefox --profile ~/.anonnet/firefox-profile-2 --no-remote &
```

This prevents cross-service correlation.

### Custom Proxy Port

Edit `user.js` if your daemon uses different ports:

```javascript
user_pref("network.proxy.socks_port", 9150);  // Custom SOCKS port
```

---

## Development

### Building the Configuration

The `user.js` file is maintained in `browser/profile/user.js`.

To add new hardening settings:

1. Edit `browser/profile/user.js`
2. Add preference with documentation
3. Test in Firefox
4. Verify it doesn't break critical functionality

### Testing Changes

```bash
# 1. Modify user.js
nano browser/profile/user.js

# 2. Delete existing profile
rm -rf ~/.anonnet/firefox-profile

# 3. Relaunch browser
./browser/scripts/launch-anonnet-browser.sh

# 4. Verify in about:config
```

### Contributing

See hardening improvements? Please contribute!

1. Test the setting thoroughly
2. Ensure it doesn't break major sites
3. Document the privacy/security benefit
4. Submit a pull request

---

## Resources

### Official Documentation
- [Tor Browser Design](https://2019.www.torproject.org/projects/torbrowser/design/)
- [Firefox Privacy Guide](https://support.mozilla.org/en-US/kb/firefox-privacy-guide)
- [arkenfox user.js](https://github.com/arkenfox/user.js)

### Testing Tools
- [Cover Your Tracks](https://coveryourtracks.eff.org/) - EFF fingerprinting test
- [BrowserLeaks](https://browserleaks.com/) - Comprehensive leak tests
- [IP Leak](https://ipleak.net/) - IP/DNS leak testing

### Privacy Projects
- [Tor Project](https://www.torproject.org/)
- [Tails OS](https://tails.boum.org/)
- [Whonix](https://www.whonix.org/)

---

## FAQ

### Q: Why Firefox and not Chrome?

**A:** Firefox has better privacy controls and is the base for Tor Browser. Chrome is developed by Google, which has financial incentives for tracking.

### Q: Can I use this with Tor instead of AnonNet?

**A:** Yes! Just change the proxy settings in `user.js` to point to Tor's ports. But you should use official Tor Browser instead.

### Q: Will this make me completely anonymous?

**A:** No. Anonymity requires:
- Technical protections (this provides)
- Operational security (your behavior)
- Trust in the network (AnonNet/Tor)

### Q: Can I access clearnet sites?

**A:** No, AnonNet blocks clearnet for user safety. Only .anon services are supported.

### Q: Why are some sites broken?

**A:** Hardening prioritizes privacy over functionality. Many tracking-heavy sites won't work. This is intentional.

### Q: Can I still use regular Firefox?

**A:** Yes! AnonNet Browser uses a separate profile. Your regular Firefox is unaffected.

### Q: How do I update Firefox?

**A:** Use your system package manager:
```bash
sudo apt update && sudo apt upgrade firefox
```

### Q: Is this as secure as Tor Browser?

**A:** It has similar hardening settings, but Tor Browser receives more security scrutiny and has additional patches. For maximum security, use official Tor Browser with Tor network.

---

## Support

For issues with:
- **Browser config**: Open issue at https://github.com/a7maadf/anonnet/issues
- **AnonNet daemon**: See main README.md
- **Firefox bugs**: Report to Mozilla

---

**Last updated:** November 2025
**Version:** 1.0.0
**Compatible with:** Firefox 115+, Firefox ESR 115+
