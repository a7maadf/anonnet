# AnonNet Hosting Guide

Complete guide to hosting websites and services on the AnonNet anonymous network.

## Table of Contents

1. [Quick Start](#quick-start)
2. [How Hidden Services Work](#how-hidden-services-work)
3. [Setting Up Your Service](#setting-up-your-service)
4. [Publishing to the Network](#publishing-to-the-network)
5. [Service Discovery](#service-discovery)
6. [Best Practices](#best-practices)
7. [Examples](#examples)
8. [Troubleshooting](#troubleshooting)

---

## Quick Start

```bash
# 1. Create your website/service (example: simple HTTP server)
cd ~/my-anon-site
python3 -m http.server 8080

# 2. Start AnonNet daemon in service mode
./target/release/anonnet-daemon node

# 3. Configure and publish your service (see below for details)
# This will generate a .anon address like: abc123def456.anon

# 4. Your service is now accessible at http://abc123def456.anon
```

---

## How Hidden Services Work

### Architecture Overview

```
Client                          Your Service
  |                                   |
  | 1. Lookup service descriptor      |
  |    in DHT                          |
  ↓                                    |
DHT ← 2. Published descriptor ← Service Registration
  |                                    |
  | 3. Build circuit to               |
  |    introduction points             |
  ↓                                    ↓
Rendezvous ← 4. Establish ← Introduction
  Point         connection      Points
  |                                    |
  └─── 5. Anonymous traffic ──────────┘
```

### Key Components

1. **Service Descriptor**
   - Public key (your service's identity)
   - Introduction points (entry nodes to your service)
   - Service metadata (ports, protocols)
   - Cryptographic signatures

2. **Introduction Points**
   - Randomly selected relay nodes
   - Act as initial contact points
   - Don't know your service's IP

3. **Rendezvous Points**
   - Meeting place for client and service
   - Neither party knows the other's location
   - Provides mutual anonymity

4. **.anon Address**
   - Derived from service's public key
   - Format: `[hash].anon` (e.g., `abc123def456.anon`)
   - Cryptographically verifiable
   - Cannot be forged or spoofed

---

## Setting Up Your Service

### Step 1: Create Your Website/Application

Your service can be **any** HTTP server, application, or protocol:

**Example 1: Static Website**
```bash
# Create website directory
mkdir -p ~/my-anon-site
cd ~/my-anon-site

# Create index.html
cat > index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>My Anonymous Site</title>
</head>
<body>
    <h1>Welcome to my .anon site!</h1>
    <p>This site is hosted anonymously on AnonNet.</p>
</body>
</html>
EOF

# Serve with Python
python3 -m http.server 8080
# Or Node.js: npx http-server -p 8080
# Or PHP: php -S 0.0.0.0:8080
```

**Example 2: Dynamic Application**
```bash
# Flask application
cat > app.py << 'EOF'
from flask import Flask

app = Flask(__name__)

@app.route('/')
def home():
    return '<h1>My Anonymous Service</h1>'

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=8080)
EOF

python3 app.py
```

**Example 3: Existing Application**
- WordPress, Ghost, or other CMS
- Forum software (Discourse, phpBB)
- E-commerce (WooCommerce, Magento)
- Custom backend APIs

**Important**: Bind to `127.0.0.1` (localhost) only, not `0.0.0.0`. The AnonNet daemon will proxy connections.

---

### Step 2: Configure Service Descriptor

Create a service configuration file:

```bash
# Create config directory
mkdir -p ~/.anonnet/services

# Create service config
cat > ~/.anonnet/services/my-site.toml << 'EOF'
[service]
name = "my-site"
description = "My anonymous website"

# Local service endpoint
local_host = "127.0.0.1"
local_port = 8080

# Service settings
protocol = "http"
public = true

# Introduction points (auto-selected if not specified)
num_introduction_points = 3

# Credit pricing
relay_price = 1  # Credits per MB

[identity]
# Auto-generated on first run
# Or specify existing key file path
# key_file = "~/.anonnet/services/my-site.key"
EOF
```

---

### Step 3: Start the Daemon as Service Node

```bash
# Full node mode (required for hosting)
./target/release/anonnet-daemon node
```

**Why full node mode?**
- Proxy mode is for browsing only
- Node mode participates in DHT, consensus, and service hosting
- Requires more resources but earns more credits

**Output:**
```
========================================
         AnonNet Node Status
========================================
Node ID:          a6e2949411ac30e0
Status:           Running
Peers:            12
Active Peers:     8
Circuits:         5
Active Circuits:  3
Bandwidth:        1024 bytes/sec
========================================
```

---

### Step 4: Publish Your Service

**Option 1: Using the API** (Recommended)

```bash
# Get API port
API_PORT=$(cat ./data/api_port.txt)

# Register service
curl -X POST http://127.0.0.1:$API_PORT/api/services/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-site",
    "local_host": "127.0.0.1",
    "local_port": 8080,
    "protocol": "http",
    "introduction_points": 3
  }'

# Response:
{
  "service_address": "abc123def456.anon",
  "public_key": "...",
  "introduction_points": [
    "node1.anon",
    "node2.anon",
    "node3.anon"
  ],
  "published": true
}
```

**Option 2: Using Configuration File**

```bash
# Place config in ~/.anonnet/services/
# Daemon auto-discovers and publishes on startup
./target/release/anonnet-daemon node --service-config ~/.anonnet/services/my-site.toml
```

**Option 3: Using CLI** (Future feature)

```bash
# Future feature
./target/release/anonnet-cli service publish \
  --name "my-site" \
  --port 8080 \
  --protocol http
```

---

## Publishing to the Network

### Automatic Publishing

Once configured, the daemon:

1. **Generates Identity** (if not exists)
   - Creates Ed25519 keypair
   - Derives .anon address from public key
   - Saves to `~/.anonnet/services/my-site.key`

2. **Selects Introduction Points**
   - Chooses 3 reliable relay nodes (configurable)
   - Establishes circuits to each
   - Tests connectivity

3. **Creates Service Descriptor**
   - Public key
   - Introduction point addresses
   - Service metadata
   - Timestamp
   - Cryptographic signature

4. **Publishes to DHT**
   - Stores descriptor on multiple nodes
   - Replicates across network
   - Re-publishes periodically (every 6 hours)

5. **Monitors Service Health**
   - Tests introduction point availability
   - Rotates failing introduction points
   - Updates descriptor when needed

---

## Service Discovery

### How Clients Find Your Service

When someone visits `http://abc123def456.anon`:

1. **DHT Lookup**
   - Client queries DHT for `abc123def456`
   - DHT returns service descriptor
   - Descriptor contains introduction points

2. **Circuit Building**
   - Client builds circuit to introduction point
   - Sends introduction cell
   - Introduction point notifies your service

3. **Rendezvous Establishment**
   - Client chooses rendezvous point
   - Your service connects to rendezvous point
   - Rendezvous point joins circuits

4. **Connection Established**
   - Client ← → Rendezvous ← → Your Service
   - End-to-end encryption
   - Mutual anonymity

---

## Best Practices

### Security

1. **Never Leak Your IP**
   - Bind services to `127.0.0.1` only
   - Don't include identifying info in content
   - Disable server headers (Server: Apache/2.4...)
   - Don't embed analytics (Google Analytics, etc.)

2. **Use HTTPS Internally** (Optional but recommended)
   ```bash
   # Generate self-signed cert
   openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

   # Configure service
   local_port = 8443
   protocol = "https"
   ```

3. **Sanitize User Input**
   - Prevent XSS, SQL injection
   - Validate all uploads
   - Use CSP headers

4. **Monitor Access Logs**
   ```bash
   # Check for suspicious activity
   tail -f ~/.anonnet/services/my-site/access.log
   ```

---

### Performance

1. **Optimize Service Response Time**
   - Cache static assets
   - Minimize database queries
   - Use CDN for large files (through AnonNet)

2. **Introduction Point Selection**
   ```toml
   # More introduction points = higher availability
   # But costs more credits
   num_introduction_points = 5  # Default: 3
   ```

3. **Resource Limits**
   ```toml
   # Limit concurrent connections
   max_connections = 100

   # Bandwidth limits (bytes/sec)
   max_bandwidth = 1048576  # 1 MB/s
   ```

---

### Reliability

1. **Keep Service Running**
   ```bash
   # Use systemd (Linux)
   sudo systemctl enable anonnet-daemon
   sudo systemctl start anonnet-daemon

   # Use screen/tmux
   screen -S anonnet
   ./target/release/anonnet-daemon node
   # Ctrl+A, D to detach
   ```

2. **Monitor Uptime**
   ```bash
   # Setup health check
   while true; do
     curl -s http://127.0.0.1:8080/health || echo "Service down!"
     sleep 60
   done
   ```

3. **Auto-Restart on Failure**
   ```bash
   # systemd service file
   [Service]
   Restart=always
   RestartSec=10
   ```

---

### Credits Management

1. **Maintain Sufficient Balance**
   ```bash
   # Check balance regularly
   API_PORT=$(cat ./data/api_port.txt)
   curl http://127.0.0.1:$API_PORT/api/credits/balance

   # Set alerts
   if [ $(curl -s http://127.0.0.1:$API_PORT/api/credits/balance | jq .balance) -lt 100 ]; then
     echo "Low credits warning!"
   fi
   ```

2. **Pricing Strategy**
   ```toml
   # Charge users for access (optional)
   [pricing]
   relay_price = 2      # 2 credits per MB
   connection_fee = 10  # 10 credits per connection
   ```

3. **Earn While Hosting**
   - Your node earns credits by relaying
   - Hosting services attracts more traffic
   - More traffic = more relay opportunities

---

## Examples

### Example 1: Anonymous Blog

```bash
# 1. Setup Ghost blog
npm install ghost-cli -g
ghost install local --port 8080

# 2. Configure service
cat > ~/.anonnet/services/blog.toml << 'EOF'
[service]
name = "blog"
description = "Anonymous blog"
local_host = "127.0.0.1"
local_port = 8080
protocol = "http"
num_introduction_points = 3
EOF

# 3. Start daemon
./target/release/anonnet-daemon node

# 4. Get your .anon address
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/services/list

# 5. Share your address
echo "Visit my blog at: http://abc123def456.anon"
```

---

### Example 2: File Sharing Service

```python
# app.py - Simple file sharing
from flask import Flask, request, send_file
import os

app = Flask(__name__)
UPLOAD_DIR = './uploads'
os.makedirs(UPLOAD_DIR, exist_ok=True)

@app.route('/')
def home():
    files = os.listdir(UPLOAD_DIR)
    return f"<h1>Files</h1><ul>{''.join(f'<li><a href=/files/{f}>{f}</a></li>' for f in files)}</ul>"

@app.route('/upload', methods=['POST'])
def upload():
    file = request.files['file']
    file.save(os.path.join(UPLOAD_DIR, file.filename))
    return 'Uploaded!'

@app.route('/files/<filename>')
def download(filename):
    return send_file(os.path.join(UPLOAD_DIR, filename))

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=8080)
```

```bash
# Run and publish
python3 app.py &
./target/release/anonnet-daemon node --service-config ~/.anonnet/services/fileshare.toml
```

---

### Example 3: Anonymous Forum

```bash
# Use Discourse or phpBB
# 1. Install forum software
docker run -d -p 127.0.0.1:8080:80 discourse/discourse

# 2. Configure AnonNet service
# ... same as above examples ...

# 3. Disable external links in forum settings
# 4. Disable user registration with email (use anonymous signup)
```

---

### Example 4: API Service

```javascript
// api.js - REST API
const express = require('express');
const app = express();

app.use(express.json());

let data = [];

app.get('/api/data', (req, res) => {
  res.json(data);
});

app.post('/api/data', (req, res) => {
  data.push(req.body);
  res.json({ success: true });
});

app.listen(8080, '127.0.0.1', () => {
  console.log('API running on 127.0.0.1:8080');
});
```

---

## Troubleshooting

### Service Not Accessible

**Problem**: Clients can't connect to your .anon address

**Solution**:

```bash
# 1. Verify service is running locally
curl http://127.0.0.1:8080

# 2. Check service is registered
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/services/list

# 3. Verify descriptor is published
curl http://127.0.0.1:$API_PORT/api/services/my-site/descriptor

# 4. Check introduction points are reachable
curl http://127.0.0.1:$API_PORT/api/services/my-site/health

# 5. Check logs
tail -f ~/.anonnet/daemon.log
```

---

### Introduction Points Failing

**Problem**: "Introduction point unreachable"

**Solution**:

```bash
# Rotate introduction points
API_PORT=$(cat ./data/api_port.txt)
curl -X POST http://127.0.0.1:$API_PORT/api/services/my-site/rotate-intro-points

# Or increase number of introduction points
# Edit config: num_introduction_points = 5
```

---

### Low Traffic / No Connections

**Problem**: Nobody is visiting your service

**Possible causes**:
1. Service not discoverable (descriptor not published)
2. Network too small (no peers)
3. Introduction points failed
4. Service is slow to respond

**Solution**:

```bash
# Check network size
API_PORT=$(cat ./data/api_port.txt)
curl http://127.0.0.1:$API_PORT/api/network/status

# Monitor connections
curl http://127.0.0.1:$API_PORT/api/services/my-site/stats

# Output:
{
  "connections": 5,
  "requests_total": 142,
  "bytes_transferred": 1048576,
  "uptime": 3600
}
```

---

### Credits Depleting Too Fast

**Problem**: Hosting service drains credits quickly

**Solution**:

1. **Increase relay participation** (earn more)
   ```toml
   # In anonnet.toml
   [relay]
   enabled = true
   bandwidth = 10485760  # 10 MB/s
   ```

2. **Charge users for access**
   ```toml
   [pricing]
   relay_price = 2
   connection_fee = 5
   ```

3. **Optimize service**
   - Cache responses
   - Compress assets
   - Reduce bandwidth usage

---

## Advanced Topics

### Custom Domain Names

While `.anon` addresses are cryptographic hashes, you can create memorable names using a naming service:

```bash
# Register human-readable name (future feature)
./target/release/anonnet-cli name register \
  --name "mysite" \
  --address "abc123def456.anon" \
  --price 100

# Now accessible at both:
# - http://abc123def456.anon (canonical)
# - http://mysite.anon (alias)
```

---

### Multi-Service Hosting

Host multiple services on one node:

```toml
# ~/.anonnet/services/services.toml
[[service]]
name = "website"
local_port = 8080

[[service]]
name = "api"
local_port = 8081

[[service]]
name = "files"
local_port = 8082
```

Each gets its own `.anon` address.

---

### Load Balancing

Distribute traffic across multiple backends:

```toml
[service]
name = "mysite"
protocol = "http"

[[backends]]
host = "127.0.0.1"
port = 8080
weight = 2

[[backends]]
host = "127.0.0.1"
port = 8081
weight = 1
```

---

### Custom Protocols

Not just HTTP - host any TCP service:

```toml
[service]
name = "ssh"
local_port = 22
protocol = "tcp"

[service]
name = "database"
local_port = 5432
protocol = "postgres"
```

---

## Next Steps

- **[Browser Usage Guide](BROWSER_USAGE.md)**: Learn how users access your service
- **[Architecture Overview](ARCHITECTURE.md)**: Deep dive into how it all works
- **[API Reference](API_REFERENCE.md)**: Full API documentation
