# AnonNet Quick Reference

Fast reference for common tasks and commands.

## Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/a7maadf/anonnet.git
cd anonnet
cargo build --release
```

---

## Daemon Commands

```bash
# Start proxy mode (for browsing)
./target/release/anonnet-daemon proxy

# Start full node mode (for hosting)
./target/release/anonnet-daemon node

# Show help
./target/release/anonnet-daemon help

# Show version
./target/release/anonnet-daemon version
```

---

## Port Discovery

```bash
# Daemon auto-selects free ports, saved to ./data/

# SOCKS5 proxy port
cat ./data/socks5_port.txt

# HTTP proxy port
cat ./data/http_port.txt

# REST API port
cat ./data/api_port.txt
```

---

## Browser Usage

### Quick Launch

```bash
# Automated launcher (recommended)
./browser/scripts/launch-anonnet-browser.sh

# Manual launch
firefox --profile ~/.anonnet/firefox-profile --no-remote
```

### Extension Installation

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Select `browser/extension/manifest.json`

### Browse .anon Sites

```
Format: http://[service-address].anon
Examples:
  - http://marketplace.anon
  - http://forum.anon
  - http://blog.anon
```

---

## API Endpoints

```bash
# Get API port
API_PORT=$(cat ./data/api_port.txt)

# Health check
curl http://127.0.0.1:$API_PORT/health

# Credit balance
curl http://127.0.0.1:$API_PORT/api/credits/balance

# Credit stats
curl http://127.0.0.1:$API_PORT/api/credits/stats

# Network status
curl http://127.0.0.1:$API_PORT/api/network/status

# Active circuits
curl http://127.0.0.1:$API_PORT/api/circuits/active
```

---

## Hosting Services

### Setup Service

```bash
# 1. Create your website
python3 -m http.server 8080

# 2. Start daemon in node mode
./target/release/anonnet-daemon node
```

### Publish Service

```bash
# Using API
API_PORT=$(cat ./data/api_port.txt)

curl -X POST http://127.0.0.1:$API_PORT/api/services/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-site",
    "local_host": "127.0.0.1",
    "local_port": 8080,
    "protocol": "http"
  }'

# Response includes your .anon address:
# {"service_address": "abc123def456.anon", ...}
```

---

## Command Line Usage

### With SOCKS5 Proxy

```bash
SOCKS_PORT=$(cat ./data/socks5_port.txt)

# curl
curl --proxy socks5h://localhost:$SOCKS_PORT http://example.anon

# wget
wget --proxy=on --http-proxy=localhost:$SOCKS_PORT http://example.anon

# git
git config --global http.proxy http://localhost:$SOCKS_PORT
```

### With HTTP Proxy

```bash
HTTP_PORT=$(cat ./data/http_port.txt)

curl --proxy http://localhost:$HTTP_PORT http://example.anon
```

---

## Troubleshooting

### Daemon Won't Start

```bash
# Check logs
tail -f ~/.anonnet/daemon.log

# Remove old port files
rm -rf ./data/

# Rebuild
cargo build --release --bin anonnet-daemon

# Restart
./target/release/anonnet-daemon proxy
```

### Can't Connect to .anon Site

```bash
# 1. Verify daemon is running
ps aux | grep anonnet-daemon

# 2. Check network status
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/network/status

# Should show: {"peers": > 0, "circuits": > 0, "is_connected": true}

# 3. Wait for DHT sync (30-60 seconds after startup)
```

### Extension Shows "API Not Available"

```bash
# Test API health
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/health

# Should return: {"status":"healthy"}

# If no response, restart daemon
pkill anonnet-daemon
./target/release/anonnet-daemon proxy
```

### Low Credits

```bash
# Check balance
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/credits/balance

# Earn more by:
# 1. Keep node running (relay for others)
# 2. Run full node mode (more relay opportunities)
# 3. Host services (get paid by visitors)
```

---

## Configuration Files

### Daemon Config

```bash
~/.anonnet/
├── daemon.pid          # Daemon process ID
├── daemon.log          # Daemon logs
└── firefox-profile/    # Browser profile
```

### Service Config

```bash
~/.anonnet/services/
└── my-site.toml        # Service configuration
```

Example service config:

```toml
[service]
name = "my-site"
local_host = "127.0.0.1"
local_port = 8080
protocol = "http"
num_introduction_points = 3
```

---

## Security Checklist

### For Browsing

- ✅ Use the hardened browser profile
- ✅ Install the browser extension
- ✅ Disable JavaScript on sensitive sites
- ✅ Clear cookies regularly
- ✅ Don't mix clearnet and .anon browsing
- ✅ Verify .anon addresses before trusting

### For Hosting

- ✅ Bind services to 127.0.0.1 only
- ✅ Don't leak identifying information
- ✅ Disable server headers (Apache/Nginx version)
- ✅ Use HTTPS internally (optional but recommended)
- ✅ Monitor access logs for suspicious activity
- ✅ Keep daemon updated

---

## Performance Tips

### Faster Browsing

```bash
# Pre-build circuits (in full node mode)
API_PORT=$(cat ./data/api_port.txt)
curl -X POST http://127.0.0.1:$API_PORT/api/circuits/pool/prebuild \
  -d '{"count": 5}'
```

### Optimize Service

```python
# Use caching
from flask import Flask
from flask_caching import Cache

app = Flask(__name__)
cache = Cache(app, config={'CACHE_TYPE': 'simple'})

@app.route('/')
@cache.cached(timeout=60)
def home():
    return 'Cached response'
```

### Monitor Performance

```bash
# Get statistics
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/network/status | jq .

# Output:
# {
#   "peers": 12,
#   "circuits": 5,
#   "bandwidth": 1048576,
#   "uptime": 3600
# }
```

---

## Development

### Build & Test

```bash
# Build
cargo build --release

# Run tests
cargo test --all

# Run specific test
cargo test test_circuit_creation

# Build docs
cargo doc --no-deps --open

# Format code
cargo fmt

# Lint
cargo clippy --all-targets
```

### Enable Debug Logging

```bash
# Verbose output
RUST_LOG=debug ./target/release/anonnet-daemon proxy

# Specific module
RUST_LOG=anonnet_core::circuit=debug ./target/release/anonnet-daemon proxy

# Very verbose
RUST_LOG=trace ./target/release/anonnet-daemon proxy
```

---

## Useful Commands

### System Integration

```bash
# Run as systemd service (Linux)
sudo tee /etc/systemd/system/anonnet.service << EOF
[Unit]
Description=AnonNet Daemon
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=/path/to/anonnet/target/release/anonnet-daemon proxy
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable anonnet
sudo systemctl start anonnet
```

### Backup & Restore

```bash
# Backup identity
cp -r ./data/identity.json ~/backup/

# Restore identity
cp ~/backup/identity.json ./data/

# Backup service keys
cp -r ~/.anonnet/services ~/backup/

# Restore service keys
cp -r ~/backup/services ~/.anonnet/
```

---

## Links

- **[Browser Usage Guide](BROWSER_USAGE.md)** - Complete browser documentation
- **[Hosting Guide](HOSTING_GUIDE.md)** - Host .anon websites
- **[Architecture Overview](ARCHITECTURE.md)** - How it works
- **[Main README](../README.md)** - Project overview

---

## Support

- **Issues**: https://github.com/a7maadf/anonnet/issues
- **Repository**: https://github.com/a7maadf/anonnet
