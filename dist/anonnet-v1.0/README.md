# AnonNet Distribution v1.0

## What's Included

This distribution contains everything you need to run AnonNet:

### Binaries (`bin/`)

- **anonnet-daemon** - Main daemon (8.6 MB)
  - Runs as relay node
  - Runs as SOCKS5/HTTP proxy
  - REST API server

- **anonweb** - Easy .anon hosting tool (1.8 MB)
  - Generate .anon domains
  - One-command website hosting
  - Like Python's `http.server` for .anon

### Browser (`browser/`)

- **extension/** - Browser extension with credit monitoring
- **profile/** - Hardened Firefox profile (user.js)
- **scripts/** - Browser launch scripts
- **fork/** - Full browser fork build system (for advanced users)

## Quick Start

### 1. Run as Proxy (Browse .anon sites)

```bash
./bin/anonnet-daemon proxy
```

Then configure your browser to use SOCKS5 proxy: `127.0.0.1:9050`

Or use the browser extension for automatic configuration.

### 2. Run as Bootstrap Node

```bash
./bin/anonnet-daemon --bootstrap
# or
./bin/anonnet-daemon bootstrap
```

### 3. Run as Relay (Earn Credits)

```bash
./bin/anonnet-daemon --accept-relay
# or
./bin/anonnet-daemon --relay
```

### 4. Host a .anon Website

```bash
# Start your web server
python3 -m http.server 8080

# Generate .anon domain
./bin/anonweb 8080

# Follow the instructions to register your service
```

## System Requirements

- **OS**: Linux (recommended), macOS, Windows
- **RAM**: 512 MB minimum, 1 GB recommended
- **Disk**: 100 MB for binaries, 1 GB for data
- **Network**: Open port 9090 for P2P (configurable)

## Default Ports

- **9090**: P2P network communication
- **9050**: SOCKS5 proxy (Tor-compatible)
- **8118**: HTTP proxy
- **19150**: REST API (auto-selected free port)

## Configuration

Create `config.toml`:

```toml
listen_addr = "0.0.0.0"
listen_port = 9090
bootstrap_nodes = [
    # Add bootstrap node addresses here
    "bootstrap1.example.com:9090",
    "bootstrap2.example.com:9090"
]
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
```

## Browser Setup

### Option 1: Extension (Recommended)

1. Open Firefox
2. Go to `about:debugging`
3. Click "Load Temporary Add-on"
4. Select `browser/extension/manifest.json`
5. Extension will auto-configure proxy

### Option 2: Manual Firefox Config

1. Copy hardened profile:
   ```bash
   cp browser/profile/user.js ~/.mozilla/firefox/YOUR_PROFILE/
   ```
2. Restart Firefox

### Option 3: Browser Fork (Advanced)

See `browser/fork/README.md` for building a custom browser.

## Architecture

```
┌─────────────────┐
│  Browser/Client │
└────────┬────────┘
         │ SOCKS5 (9050)
         ▼
┌─────────────────┐
│  AnonNet Daemon │◄──────┐
│   (Local Node)  │       │ P2P (9090)
└────────┬────────┘       │
         │                │
         │ P2P Network    │
         ▼                ▼
  ┌──────────┐    ┌──────────┐
  │ Relay 1  │◄──►│ Relay 2  │
  └──────────┘    └──────────┘
         │                │
         │   3-hop circuit│
         ▼                ▼
  ┌──────────────────────┐
  │   .anon Service      │
  └──────────────────────┘
```

## Features

✅ Anonymous 3-hop circuits (Tor-like)
✅ .anon hidden services (no exit nodes)
✅ P2P DHT for service discovery
✅ Credit-based economy (relay to earn)
✅ QUIC transport (modern, fast)
✅ Browser extension with monitoring
✅ Compatible with Tor Browser hardening
✅ Flexible circuit creation (1-3 hops for early network)

## Credit System

- **Earn Credits**: Run as relay, forward traffic
- **Spend Credits**: Use network to browse .anon sites
- **Initial Balance**: 1000 credits (from Proof-of-Work)
- **Rate**: 1000 credits per GB relayed

Monitor your balance via browser extension or API.

## API Endpoints

REST API runs on port 19150 (or auto-selected):

```bash
# Get credit balance
curl http://localhost:19150/api/credits/balance

# Get network status
curl http://localhost:19150/api/network/status

# Get active circuits
curl http://localhost:19150/api/circuits/active

# Register .anon service
curl -X POST http://localhost:19150/api/services/register \
  -H 'Content-Type: application/json' \
  -d '{"local_host": "127.0.0.1", "local_port": 8080, "ttl_hours": 24}'
```

## Security Notes

⚠️ **This is experimental software**

- Use Tor Browser for maximum anonymity guarantees
- AnonNet is for .anon services only (no clearnet access)
- Early network may use 1-2 hop circuits (reduced anonymity)
- Browser extension shows security warnings
- Keep your private keys safe (.anonweb/, data/)

## Troubleshooting

**Daemon won't start:**
```bash
# Check if port is in use
sudo netstat -tulpn | grep 9090

# Try a different port
./bin/anonnet-daemon --port 9091
```

**Can't browse .anon sites:**
```bash
# Check daemon is running
ps aux | grep anonnet-daemon

# Check proxy port
cat data/socks5_port.txt

# Verify proxy in browser settings
```

**Low on credits:**
```bash
# Run as relay to earn credits
./bin/anonnet-daemon --relay

# Check balance
curl http://localhost:19150/api/credits/balance
```

## Building from Source

If you want to build from source:

```bash
# Clone repository
git clone https://github.com/a7maadf/anonnet
cd anonnet

# Build
cargo build --release

# Binaries in target/release/
```

## Support

- **GitHub**: https://github.com/a7maadf/anonnet
- **Issues**: https://github.com/a7maadf/anonnet/issues
- **Discussions**: https://github.com/a7maadf/anonnet/discussions

## License

Dual MIT/Apache-2.0

## Acknowledgments

- Tor Project for privacy design principles
- Mozilla Firefox for browser foundation
- Rust community for excellent libraries

---

**Welcome to AnonNet!** 🌐🔒

Start with: `./bin/anonnet-daemon proxy` and browse to a .anon site!
