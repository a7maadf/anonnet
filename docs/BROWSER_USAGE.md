# AnonNet Browser Usage Guide

Complete guide to using the AnonNet browser for anonymous .anon browsing.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Installation](#installation)
3. [Launching the Browser](#launching-the-browser)
4. [Installing the Extension](#installing-the-extension)
5. [Browsing .anon Sites](#browsing-anon-sites)
6. [Credit System](#credit-system)
7. [Troubleshooting](#troubleshooting)

---

## Quick Start

```bash
# 1. Build the daemon (first time only)
cd /path/to/anonnet
cargo build --release --bin anonnet-daemon

# 2. Launch the browser (starts daemon + Firefox)
./browser/scripts/launch-anonnet-browser.sh

# 3. Install the extension in Firefox
# Open about:debugging#/runtime/this-firefox
# Click "Load Temporary Add-on"
# Select browser/extension/manifest.json

# 4. Browse .anon sites!
# Example: http://marketplace.anon
```

---

## Installation

### Prerequisites

1. **Rust toolchain** (for building the daemon)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Firefox** (or Firefox ESR)
   - Linux: `sudo apt install firefox` or `sudo dnf install firefox`
   - macOS: Download from [mozilla.org](https://www.mozilla.org/firefox/)
   - Windows: Download from [mozilla.org](https://www.mozilla.org/firefox/)

3. **Build tools** (Linux only)
   ```bash
   # Ubuntu/Debian
   sudo apt install build-essential pkg-config libssl-dev

   # Fedora/RHEL
   sudo dnf install gcc pkg-config openssl-devel
   ```

### Build the Daemon

```bash
cd /path/to/anonnet
cargo build --release --bin anonnet-daemon
```

This creates the daemon binary at `target/release/anonnet-daemon`.

---

## Launching the Browser

### Option 1: Automated Launcher (Recommended)

The launcher script handles everything automatically:

```bash
./browser/scripts/launch-anonnet-browser.sh
```

**What it does:**
1. ✅ Checks if daemon is built (builds if needed)
2. ✅ Starts the AnonNet daemon in background
3. ✅ Creates hardened Firefox profile with Tor Browser settings
4. ✅ Configures SOCKS5 proxy with auto-discovered port
5. ✅ Launches Firefox with the hardened profile

**Output:**
```
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║              🌐  ANONNET BROWSER LAUNCHER  🌐             ║
║                                                           ║
║  Anonymous browsing powered by AnonNet                    ║
║  Hardened with Tor Browser security features              ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝

[INFO] AnonNet daemon already running
[SUCCESS] AnonNet daemon started (PID: 12345)
[SUCCESS] SOCKS5 proxy: 127.0.0.1:53175
[SUCCESS] HTTP proxy: 127.0.0.1:53177
[SUCCESS] Hardening configuration applied (SOCKS5: 53175)
[INFO] Launching Firefox with AnonNet profile...
[SUCCESS] Firefox launched (PID: 12346)
```

### Option 2: Manual Setup

If you prefer manual control:

```bash
# 1. Start the daemon
./target/release/anonnet-daemon proxy

# Daemon will output:
# ╔═══════════════════════════════════════════════════╗
# ║   SOCKS5 Proxy Started on 127.0.0.1:53175   ║
# ╚═══════════════════════════════════════════════════╝

# 2. Check port files
cat ./data/socks5_port.txt  # e.g., 53175
cat ./data/http_port.txt    # e.g., 53177
cat ./data/api_port.txt     # e.g., 53176

# 3. Launch Firefox with custom profile
firefox --profile ~/.anonnet/firefox-profile --no-remote

# 4. Configure Firefox proxy manually
# Settings → Network Settings → Manual proxy configuration
# SOCKS Host: 127.0.0.1, Port: [value from socks5_port.txt]
# SOCKS v5: ✓
# Proxy DNS when using SOCKS v5: ✓
```

---

## Installing the Extension

The browser extension displays your credit balance and enforces .anon-only browsing.

### Steps

1. **Open Firefox Developer Tools**
   - Type in address bar: `about:debugging#/runtime/this-firefox`
   - Or: Menu → More Tools → Developer Tools → Three-dot menu → about:debugging

2. **Load the Extension**
   - Click **"Load Temporary Add-on..."**
   - Navigate to: `/path/to/anonnet/browser/extension/`
   - Select: `manifest.json`
   - Click **"Open"**

3. **Verify Installation**
   - You should see the AnonNet icon in the toolbar
   - Click it to see credit balance and network stats

### Extension Features

- **Credit Balance Display**: Shows your current credits
- **Network Statistics**: Peers, circuits, bandwidth usage
- **Earning/Spending Rates**: Real-time credit flow
- **.anon-only Enforcement**: Blocks all clearnet sites automatically
- **Auto Port Discovery**: Finds daemon API automatically

---

## Browsing .anon Sites

### How .anon Addresses Work

AnonNet uses `.anon` top-level domain for hidden services:

```
Format: [service-name].anon
Examples:
  - marketplace.anon
  - forum.anon
  - blog.anon
  - social.anon
```

Each `.anon` address is a cryptographic identifier derived from the service's public key.

### Browsing Steps

1. **Ensure daemon is running**
   ```bash
   # Check if running
   ps aux | grep anonnet-daemon

   # Or check port files exist
   ls -la ./data/*.txt
   ```

2. **Open Firefox** (with AnonNet profile)

3. **Visit a .anon site**
   - Type in address bar: `http://example.anon`
   - Press Enter

4. **First Connection**
   - Browser sends request through SOCKS5 proxy (daemon)
   - Daemon looks up service descriptor in DHT
   - Builds multi-hop circuit to the service
   - Establishes connection
   - Page loads!

### What Happens Behind the Scenes

```
You → Firefox → SOCKS5 Proxy → AnonNet Daemon
                                      ↓
                                   DHT Lookup
                                      ↓
                              Find Service Descriptor
                                      ↓
                              Build Circuit (3 hops)
                                      ↓
                              Connect to Service
                                      ↓
                              Relay Traffic ← → Service
```

### Blocked: Clearnet Sites

The daemon **automatically blocks** all clearnet sites for your safety:

- ❌ `http://google.com` → **BLOCKED**
- ❌ `https://github.com` → **BLOCKED**
- ❌ `http://192.168.1.1` → **BLOCKED**
- ✅ `http://marketplace.anon` → **ALLOWED**

**Why?** AnonNet is designed for anonymous .anon services only. Mixing clearnet and anonymous traffic can compromise anonymity.

---

## Credit System

### Overview

AnonNet uses a credit-based economy to incentivize network participation:

- **Earn credits** by relaying traffic for others
- **Spend credits** when using the network (browsing, hosting)
- **Balance** is tracked by the consensus ledger

### Viewing Your Balance

**Option 1: Browser Extension**
- Click the AnonNet icon in toolbar
- View real-time balance and stats

**Option 2: API Endpoint**
```bash
# Read the API port
API_PORT=$(cat ./data/api_port.txt)

# Get credit balance
curl http://127.0.0.1:$API_PORT/api/credits/balance

# Output:
{
  "balance": 1500,
  "reserved": 50,
  "available": 1450
}
```

**Option 3: Command Line**
```bash
# Future feature - CLI tool
./target/release/anonnet-cli credits balance
```

### Earning Credits

You automatically earn credits by:

1. **Relaying Traffic**: When your node is part of someone else's circuit
2. **Hosting Services**: When others access your .anon sites
3. **DHT Participation**: Storing service descriptors
4. **Bootstrap Nodes**: Helping new nodes join the network

**Rate**: Configurable, typically ~1 credit per MB relayed

### Spending Credits

You spend credits when:

1. **Building Circuits**: Each hop costs credits
2. **Browsing .anon Sites**: Traffic through your circuits
3. **Publishing Services**: Registering in DHT
4. **DHT Lookups**: Finding service descriptors

**Rate**: Configurable, typically ~1 credit per MB transferred

### Initial Balance

New nodes receive an initial credit allocation to bootstrap participation:

- **Default**: 1000 credits
- **Bootstrap**: Enough for ~1 GB of traffic
- **Replenishment**: Earn more by relaying

### Monitoring

```bash
# Get detailed stats
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/credits/stats

# Output:
{
  "balance": 1450,
  "earned_total": 500,
  "spent_total": 50,
  "earning_rate": 10.5,
  "spending_rate": 2.3,
  "transactions_count": 47
}
```

---

## Troubleshooting

### Daemon Won't Start

**Problem**: `Address already in use (os error 48)`

**Solution**: The daemon auto-selects free ports. This error should not occur. If it does:

```bash
# Clean up old port files
rm -rf ./data/

# Restart daemon
./target/release/anonnet-daemon proxy
```

---

### Firefox Can't Connect

**Problem**: "Unable to connect" or "Proxy server is refusing connections"

**Solution**: Verify SOCKS5 proxy settings:

```bash
# 1. Check daemon is running
ps aux | grep anonnet-daemon

# 2. Check port files exist
cat ./data/socks5_port.txt

# 3. Test SOCKS5 proxy
SOCKS_PORT=$(cat ./data/socks5_port.txt)
curl --proxy socks5h://127.0.0.1:$SOCKS_PORT http://example.anon
```

**Fix Firefox settings**:
1. Settings → Network Settings
2. Manual proxy configuration
3. SOCKS Host: `127.0.0.1`
4. Port: [value from `socks5_port.txt`]
5. ✓ SOCKS v5
6. ✓ Proxy DNS when using SOCKS v5

---

### Extension Shows "API Not Available"

**Problem**: Extension can't connect to daemon API

**Solution**:

```bash
# 1. Check API port
cat ./data/api_port.txt

# 2. Test API health endpoint
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/health

# Should return: {"status":"healthy"}

# 3. If no response, restart daemon
pkill anonnet-daemon
./target/release/anonnet-daemon proxy
```

---

### .anon Site Not Loading

**Problem**: "Service descriptor not found" or timeout

**Possible causes**:

1. **Service is offline**: The .anon service is not running
2. **DHT not synchronized**: Your node hasn't joined the network yet
3. **No peers**: Network is empty (bootstrap node needed)

**Solution**:

```bash
# Check network status
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/network/status

# Output:
{
  "peers": 5,           # Should be > 0
  "circuits": 3,        # Should be > 0
  "is_connected": true
}
```

**Wait for DHT sync**: New nodes need 30-60 seconds to discover peers.

---

### Clearnet Site Blocked

**Problem**: "Clearnet access blocked" error

**Solution**: This is **intentional**. AnonNet only supports .anon sites for anonymity.

If you need clearnet access:
- Use a separate Firefox profile
- Or disable the proxy for clearnet browsing

**Not recommended**: Mixing clearnet and anonymous traffic compromises anonymity.

---

### Low Credit Balance

**Problem**: "Insufficient credits" errors

**Solution**:

```bash
# Check balance
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/credits/balance

# Earn more credits by:
# 1. Keep your node running (relay for others)
# 2. Host services (others will pay you)
# 3. Join as a reliable relay node
```

---

### Logs and Debugging

```bash
# View daemon logs (if launched manually)
tail -f ~/.anonnet/daemon.log

# View daemon logs (if launched by script)
./target/release/anonnet-daemon proxy

# Enable verbose logging
RUST_LOG=debug ./target/release/anonnet-daemon proxy
```

---

## Advanced Configuration

### Custom Firefox Path

```bash
# Specify Firefox location
./browser/scripts/launch-anonnet-browser.sh /custom/path/to/firefox
```

### Persistent Extension

To keep the extension installed permanently:

1. Sign the extension (requires Mozilla account)
2. Or use Firefox Developer Edition / Nightly (no signing required)

### Custom Hardening

Edit `browser/profile/user.js` to customize settings:

```javascript
// Example: Change fingerprinting resistance
user_pref("privacy.resistFingerprinting", true);

// Example: Disable WebRTC
user_pref("media.peerconnection.enabled", false);
```

Then restart Firefox or run the launcher again.

---

## Next Steps

- **[Hosting Websites Guide](HOSTING_GUIDE.md)**: Learn how to host your own .anon sites
- **[Architecture Overview](ARCHITECTURE.md)**: Understand how AnonNet works
- **[Developer Guide](DEVELOPER_GUIDE.md)**: Build applications on AnonNet
