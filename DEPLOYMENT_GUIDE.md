# AnonNet Distributed Deployment Guide

**Complete guide for deploying AnonNet across multiple physical locations with encrypted multihop routing.**

---

## 🎯 Overview

This guide walks you through deploying a **real distributed anonymous network** across 5 physical locations. Each node runs on a separate laptop at a different location, creating a true P2P network with:

- ✅ **Distributed DHT** - Service discovery across all nodes
- ✅ **Multihop circuits** - Encrypted routing through relay nodes
- ✅ **Anonymous services** - .anon websites accessible via SOCKS5
- ✅ **No central authority** - Fully decentralized P2P architecture

---

## 📊 Network Topology

```
┌─────────────────┐
│   Laptop 1      │ Friend A's House
│   BOOTSTRAP     │ Public IP: AAA.AAA.AAA.AAA
│   Port: 9000    │ Role: Network root, peer discovery
└────────┬────────┘
         │
         ├──────────────┬──────────────┬──────────────┐
         │              │              │              │
    ┌────▼─────┐   ┌───▼──────┐   ┌──▼───────┐  ┌──▼───────┐
    │ Laptop 2 │   │ Laptop 3 │   │ Laptop 4 │  │ Laptop 5 │
    │ SERVICE  │   │  RELAY   │   │  RELAY   │  │  CLIENT  │
    │Friend B  │   │Friend C  │   │Friend D  │  │Friend E  │
    │Port:9000 │   │Port:9000 │   │Port:9000 │  │Port:9000 │
    └──────────┘   └──────────┘   └──────────┘  └──────────┘
    Hosts .anon     Relays        Relays         Accesses
    website         traffic       traffic        website

Circuit Flow (encrypted hops):
Laptop 5 → Laptop 4 → Laptop 3 → Laptop 2 (Service)
```

---

## 🛠️ Prerequisites

### On Each Laptop:

1. **Operating System**: Ubuntu 22.04+ (or Debian-based Linux)
2. **Internet Connection**: Stable broadband or mobile hotspot
3. **Port Forwarding**: Ability to forward UDP port 9000 (or use ngrok)
4. **Disk Space**: 1 GB minimum
5. **RAM**: 1 GB minimum

### Required Information:

- **Laptop 1 (Bootstrap)**: Public IP address or ngrok URL
- **Network Access**: All laptops can reach the bootstrap node

---

## 📦 Installation (Run on ALL 5 Laptops)

### Step 1: Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install build tools
sudo apt install -y build-essential git curl pkg-config libssl-dev jq net-tools

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Step 2: Clone and Build AnonNet

```bash
# Clone repository
cd ~
git clone https://github.com/a7maadf/anonnet
cd anonnet

# Checkout the deployment branch
git checkout claude/review-and-test-infrastructure-01QKC3HFaGmw3hdStiHY8ZWb

# Build release binary
cargo build --release

# Copy binary to system path
sudo cp target/release/anonnet-daemon /usr/local/bin/
sudo chmod +x /usr/local/bin/anonnet-daemon

# Verify
anonnet-daemon --version
```

---

## 🔧 Configuration

### Laptop 1: Bootstrap Node (Friend A's House)

**Purpose**: Network root, enables peer discovery

**1. Get Public IP:**

```bash
# Get your public IP
PUBLIC_IP=$(curl -s ifconfig.me)
echo "Bootstrap Public IP: $PUBLIC_IP"

# Save this IP - all other laptops need it!
echo $PUBLIC_IP > ~/bootstrap-ip.txt
```

**2. Configure Port Forwarding:**

Option A: Router Port Forwarding
- Log into your router (usually 192.168.1.1)
- Forward UDP port 9000 → This laptop's local IP
- Restart router if needed

Option B: Use ngrok (if port forwarding not possible)
```bash
# Install ngrok
wget https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-linux-amd64.tgz
tar xvzf ngrok-v3-stable-linux-amd64.tgz
sudo mv ngrok /usr/local/bin/

# Sign up at ngrok.com and get auth token
ngrok config add-authtoken YOUR_TOKEN_HERE

# Expose port 9000
ngrok tcp 9000
# Note the URL: tcp://X.tcp.ngrok.io:XXXXX
```

**3. Create Configuration:**

```bash
# Create config directory
mkdir -p ~/anonnet-bootstrap/data

# Create config file
cat > ~/anonnet-bootstrap/config.toml <<'EOF'
# Bootstrap Node Configuration
# This is the network root - all other nodes connect here

listen_addr = "0.0.0.0"
listen_port = 9000
bootstrap_nodes = []
accept_relay = true
max_peers = 100
data_dir = "./data"
verbose = false
EOF
```

**4. Configure Firewall:**

```bash
# Allow incoming UDP on port 9000
sudo ufw allow 9000/udp
sudo ufw enable
sudo ufw status
```

**5. Start Bootstrap Node:**

```bash
cd ~/anonnet-bootstrap
anonnet-daemon --config config.toml > bootstrap.log 2>&1 &
echo $! > bootstrap.pid

# Wait for startup
sleep 5

# Verify it's running
cat data/api_port.txt  # Should show API port
curl -s http://localhost:$(cat data/api_port.txt)/api/stats | jq
```

**6. Share Bootstrap Address:**

Send to all friends:
- If using public IP: `YOUR_PUBLIC_IP:9000`
- If using ngrok: `X.tcp.ngrok.io:XXXXX` (from ngrok terminal)

---

### Laptop 2: Service Host (Friend B's House)

**Purpose**: Hosts the .anon website that others will access

**1. Create Configuration:**

```bash
mkdir -p ~/anonnet-service/data

# Replace BOOTSTRAP_ADDRESS with value from Laptop 1
cat > ~/anonnet-service/config.toml <<'EOF'
# Service Host Node Configuration
# Hosts .anon websites and relays traffic

listen_addr = "0.0.0.0"
listen_port = 9000
bootstrap_nodes = ["BOOTSTRAP_ADDRESS"]  # ← Replace this!
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF

# Example: bootstrap_nodes = ["1.2.3.4:9000"]
# Or: bootstrap_nodes = ["0.tcp.ngrok.io:12345"]
```

**2. Configure Firewall:**

```bash
sudo ufw allow 9000/udp
sudo ufw enable
```

**3. Start Service Node:**

```bash
cd ~/anonnet-service
anonnet-daemon --config config.toml > service.log 2>&1 &
echo $! > service.pid

# Wait for DHT discovery
sleep 30

# Check peers (should be 1 or more)
API_PORT=$(cat data/api_port.txt)
curl -s http://localhost:$API_PORT/api/stats | jq '.peers'
```

**4. Create Test Website:**

```bash
# Create website directory
mkdir -p ~/website

# Create test page
cat > ~/website/index.html <<'HTML'
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Welcome to AnonNet!</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Courier New', monospace;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #fff;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .container {
            max-width: 800px;
            background: rgba(0, 0, 0, 0.3);
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
        }
        h1 { font-size: 2.5em; margin-bottom: 20px; }
        .status { color: #4ade80; font-size: 1.2em; margin: 20px 0; }
        .info { background: rgba(255, 255, 255, 0.1); padding: 20px; border-radius: 5px; margin: 20px 0; }
        .info h3 { margin-bottom: 10px; }
        .info p { line-height: 1.6; }
        code { background: rgba(0, 0, 0, 0.5); padding: 2px 6px; border-radius: 3px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🎉 Welcome to AnonNet!</h1>
        <div class="status">✅ Connection Successful!</div>

        <div class="info">
            <h3>🔒 Privacy Protected</h3>
            <p>You are accessing this website through an encrypted multihop circuit:</p>
            <p><code>Your Device → Relay → Relay → Service</code></p>
        </div>

        <div class="info">
            <h3>🌐 Anonymous Service</h3>
            <p>This is a <strong>.anon</strong> hidden service. The server's real IP address is protected by the AnonNet protocol.</p>
        </div>

        <div class="info">
            <h3>🚀 What's Happening</h3>
            <p>• Your request is encrypted with multiple layers (onion routing)</p>
            <p>• Each hop only knows the previous and next node</p>
            <p>• Service descriptor retrieved via distributed DHT</p>
            <p>• No central authority - fully P2P network</p>
        </div>

        <div class="info">
            <h3>📊 Technical Details</h3>
            <p>• Protocol: AnonNet v1.0</p>
            <p>• Encryption: ChaCha20-Poly1305</p>
            <p>• DHT: Kademlia-based</p>
            <p>• Transport: QUIC over UDP</p>
        </div>
    </div>
</body>
</html>
HTML

# Start web server
python3 -m http.server 8080 -d ~/website > ~/web.log 2>&1 &
echo $! > ~/web.pid
```

**5. Register .anon Service:**

```bash
# Register the website as a .anon service
API_PORT=$(cat ~/anonnet-service/data/api_port.txt)
curl -X POST http://localhost:$API_PORT/api/services/register \
  -H "Content-Type: application/json" \
  -d '{"local_host":"127.0.0.1","local_port":8080,"ttl_hours":24}'

# Save the .anon address
curl -X POST http://localhost:$API_PORT/api/services/register \
  -H "Content-Type: application/json" \
  -d '{"local_host":"127.0.0.1","local_port":8080,"ttl_hours":24}' \
  | jq -r '.anon_address' > ~/anon-address.txt

# Display it
echo "Your .anon address:"
cat ~/anon-address.txt
```

**IMPORTANT:** Share this .anon address with the person at Laptop 5!

---

### Laptop 3 & 4: Relay Nodes (Friends C & D)

**Purpose**: Relay encrypted traffic, provide additional network hops

**Configuration (Same for both):**

```bash
mkdir -p ~/anonnet-relay/data

# Replace BOOTSTRAP_ADDRESS with value from Laptop 1
cat > ~/anonnet-relay/config.toml <<'EOF'
# Relay Node Configuration
# Relays encrypted traffic for multihop circuits

listen_addr = "0.0.0.0"
listen_port = 9000
bootstrap_nodes = ["BOOTSTRAP_ADDRESS"]  # ← Replace this!
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF
```

**Firewall:**

```bash
sudo ufw allow 9000/udp
sudo ufw enable
```

**Start Relay:**

```bash
cd ~/anonnet-relay
anonnet-daemon --config config.toml > relay.log 2>&1 &
echo $! > relay.pid

# Wait for DHT discovery
sleep 30

# Check peers
API_PORT=$(cat data/api_port.txt)
curl -s http://localhost:$API_PORT/api/stats | jq '.peers'
```

---

### Laptop 5: Client Node (Friend E's House)

**Purpose**: Access the .anon website via encrypted multihop circuit

**1. Create Configuration:**

```bash
mkdir -p ~/anonnet-client/data

# Replace BOOTSTRAP_ADDRESS with value from Laptop 1
cat > ~/anonnet-client/config.toml <<'EOF'
# Client Node Configuration
# Access .anon services via SOCKS5 proxy

listen_addr = "0.0.0.0"
listen_port = 9000
bootstrap_nodes = ["BOOTSTRAP_ADDRESS"]  # ← Replace this!
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF
```

**2. Configure Firewall:**

```bash
sudo ufw allow 9000/udp
sudo ufw enable
```

**3. Start Client:**

```bash
cd ~/anonnet-client
anonnet-daemon --config config.toml > client.log 2>&1 &
echo $! > client.pid

# Wait for DHT discovery and descriptor replication
sleep 60

# Check peers
API_PORT=$(cat data/api_port.txt)
curl -s http://localhost:$API_PORT/api/stats | jq '.peers'
```

**4. Access the .anon Website:**

```bash
# Get SOCKS5 port
SOCKS_PORT=$(cat data/socks5_port.txt)
echo "SOCKS5 Proxy: localhost:$SOCKS_PORT"

# Get .anon address from Laptop 2
# (Friend B should share this with you)
ANON_ADDRESS="paste-anon-address-here.anon"

# Test access via curl
curl --socks5-hostname localhost:$SOCKS_PORT http://$ANON_ADDRESS/

# Or use Firefox
echo "Firefox SOCKS5 Proxy Settings:"
echo "  Host: localhost"
echo "  Port: $SOCKS_PORT"
echo "  ✅ Check 'Proxy DNS when using SOCKS v5'"
echo ""
echo "Then navigate to: http://$ANON_ADDRESS/"
```

**5. Configure Firefox:**

1. Open Firefox
2. Settings → Network Settings → Manual proxy configuration
3. SOCKS v5 Host: `localhost`
4. Port: (value from `socks5_port.txt`)
5. ✅ Check "Proxy DNS when using SOCKS v5"
6. Navigate to: `http://YOUR_ANON_ADDRESS.anon/`

---

## 🔍 Verification & Monitoring

### Check Network Status (Run on any laptop)

```bash
#!/bin/bash
# Save as check-status.sh

API_PORT=$(cat data/api_port.txt 2>/dev/null)
if [ -z "$API_PORT" ]; then
    echo "❌ Daemon not running"
    exit 1
fi

echo "🌐 AnonNet Status"
echo "================="
STATS=$(curl -s http://localhost:$API_PORT/api/stats)
echo "$STATS" | jq .

PEERS=$(echo "$STATS" | jq -r '.peers // 0')
echo ""
echo "👥 Peers: $PEERS"

if [ "$PEERS" -gt 0 ]; then
    echo "✅ Connected to network!"
else
    echo "⚠️  No peers - check bootstrap address and network"
fi
```

### Monitor DHT Activity (Laptop 2 - Service Host)

```bash
# Watch descriptor replication
tail -f service.log | grep -i "Replicating descriptor\|Sent STORE"

# Should show:
# INFO Replicating descriptor abc...xyz.anon to 4 nodes via DHT
# DEBUG Sent STORE to node_123...
```

### Monitor DHT Lookups (Laptop 5 - Client)

```bash
# Watch service discovery
tail -f client.log | grep -i "Querying.*nodes\|Found descriptor"

# Should show:
# INFO Querying 4 nodes for descriptor abc...xyz.anon
# INFO Found descriptor abc...xyz.anon from node node_456...
```

### Monitor Circuit Creation (All nodes)

```bash
# Watch circuit activity
tail -f *.log | grep -i "circuit\|relay"
```

---

## 🐛 Troubleshooting

### Peers: 0 (Not connecting to network)

**Laptop 1 (Bootstrap):**
- Check firewall: `sudo ufw status`
- Verify port forwarding works: `netstat -tulpn | grep 9000`
- Test public accessibility: From another network, `nc -zuv PUBLIC_IP 9000`

**Other Laptops:**
- Verify bootstrap address is correct in `config.toml`
- Check logs: `tail -50 *.log | grep -i "bootstrap\|connect"`
- Ping bootstrap: `ping BOOTSTRAP_IP`
- Wait 60 seconds for DHT discovery

### Service Descriptor Not Found

**On Laptop 2:**
- Verify service is registered: `curl http://localhost:API_PORT/api/services/list`
- Check DHT replication logs: `grep "Replicating descriptor" service.log`
- Verify peers > 0: `curl http://localhost:API_PORT/api/stats | jq '.peers'`

**On Laptop 5:**
- Wait 2-3 minutes after service registration (DHT propagation)
- Check DHT lookup logs: `grep "Querying.*nodes" client.log`
- Verify .anon address is correct (52 chars + .anon)

### Website Won't Load

1. **Check service is running:**
   ```bash
   # On Laptop 2
   curl http://localhost:8080
   # Should show HTML
   ```

2. **Check SOCKS5 proxy:**
   ```bash
   # On Laptop 5
   netstat -tulpn | grep $(cat data/socks5_port.txt)
   # Should show LISTEN
   ```

3. **Check descriptor lookup:**
   ```bash
   # On Laptop 5
   tail -20 client.log | grep -i descriptor
   ```

4. **Test direct connection:**
   ```bash
   # On Laptop 5
   curl --socks5-hostname localhost:SOCKS_PORT http://ANON_ADDRESS.anon/
   ```

### Performance Issues

- **Slow loading**: Normal for multihop - 3-5 second delay expected
- **Timeouts**: Increase DHT discovery time, check relay node connectivity
- **High CPU**: Expected during initial DHT bootstrap, should stabilize

---

## 📊 Expected Network Behavior

### Startup Timeline

| Time | Event |
|------|-------|
| T+0s | Daemon starts |
| T+5s | QUIC endpoint ready |
| T+10s | Connected to bootstrap |
| T+30s | DHT routing table populated |
| T+60s | Full network discovery complete |

### After Service Registration (Laptop 2)

| Time | Event |
|------|-------|
| T+0s | Service registered locally |
| T+1s | STORE messages sent to 4 nodes |
| T+5s | Descriptors replicated network-wide |
| T+10s | All nodes have cached descriptor |

### When Accessing Service (Laptop 5)

| Time | Event |
|------|-------|
| T+0s | SOCKS5 receives .anon request |
| T+1s | DHT lookup queries 4 nodes |
| T+2s | Descriptor found, cached locally |
| T+3s | Circuit created through relays |
| T+5s | Connection established, page loads |

---

## 🎯 Success Criteria

✅ **All laptops show Peers > 0**
✅ **Laptop 2 shows "Sent STORE to node_XXX"**
✅ **Laptop 5 shows "Found descriptor from node_XXX"**
✅ **Firefox loads the .anon website**
✅ **Multihop circuit visible in logs**

---

## 📝 Maintenance

### Stop Daemon

```bash
kill $(cat *.pid)
```

### Restart Daemon

```bash
# Same start command as initial setup
cd ~/anonnet-*/
anonnet-daemon --config config.toml > *.log 2>&1 &
echo $! > *.pid
```

### View Live Logs

```bash
tail -f *.log
```

### Clean Data (Reset)

```bash
# CAUTION: Deletes all data including keys
rm -rf data/
mkdir data/
```

---

## 🔐 Security Notes

1. **Service Registration**: Only register services you control
2. **Port Forwarding**: Only forward UDP 9000, nothing else
3. **Firewall**: Use `ufw` to restrict other ports
4. **SSH Access**: Disable password auth, use keys only
5. **Updates**: Keep system updated with `sudo apt update && sudo apt upgrade`

---

## 📞 Support

**Logs Location**: `~/anonnet-*/`
**Check Status**: `curl http://localhost:API_PORT/api/stats`
**Monitor**: `tail -f *.log`

**Common Issues**:
- Peers: 0 → Check bootstrap address and firewall
- Service not found → Wait 2 minutes after registration
- Slow loading → Normal for multihop (3-5 seconds)

---

## 🎉 Success!

If you can access the .anon website from Laptop 5, congratulations! You've successfully deployed a **real distributed anonymous network** with:

- ✅ Distributed DHT service discovery
- ✅ Encrypted multihop circuits
- ✅ Anonymous .anon hidden services
- ✅ P2P architecture across 5 physical locations

**Your network is now operational!** 🚀
