# AnonNet Deployment Package

Ready-to-use configurations and scripts for deploying AnonNet across 5 physical locations.

## 📦 Package Contents

```
deployment/
├── README.md                    # This file
├── bootstrap/                   # Laptop 1 - Network root
│   ├── config.toml             # Configuration
│   ├── start.sh                # Start script
│   ├── stop.sh                 # Stop script
│   └── monitor.sh              # Status monitor
├── service/                     # Laptop 2 - Website host
│   ├── config.toml
│   ├── start.sh
│   ├── stop.sh
│   ├── monitor.sh
│   └── register-service.sh     # Register .anon service
├── relay/                       # Laptops 3 & 4 - Traffic relays
│   ├── config.toml
│   ├── start.sh
│   ├── stop.sh
│   └── monitor.sh
└── client/                      # Laptop 5 - Access websites
    ├── config.toml
    ├── start.sh
    ├── stop.sh
    └── monitor.sh
```

## 🚀 Quick Start

### 1. Copy to Each Laptop

**Laptop 1 (Bootstrap):**
```bash
cp -r deployment/bootstrap ~/anonnet-node
cd ~/anonnet-node
```

**Laptop 2 (Service):**
```bash
cp -r deployment/service ~/anonnet-node
cd ~/anonnet-node
```

**Laptops 3 & 4 (Relays):**
```bash
cp -r deployment/relay ~/anonnet-node
cd ~/anonnet-node
```

**Laptop 5 (Client):**
```bash
cp -r deployment/client ~/anonnet-node
cd ~/anonnet-node
```

### 2. Configure

**Laptop 1 only:**
- No configuration needed (bootstrap is the root)
- Just run `./start.sh`
- Share the public IP:PORT with everyone

**Laptops 2, 3, 4, 5:**
- Edit `config.toml`
- Replace `BOOTSTRAP_ADDRESS_HERE` with Laptop 1's address
- Example: `bootstrap_nodes = ["1.2.3.4:9000"]`

### 3. Start Nodes

On each laptop:
```bash
./start.sh
```

Wait 60 seconds for network formation.

### 4. Verify

On each laptop:
```bash
./monitor.sh
```

Look for: `✅ Connected to X peer(s)` where X > 0

### 5. Register Service (Laptop 2 only)

```bash
# Create website
mkdir -p ~/website
cat > ~/website/index.html << 'HTML'
<html><body><h1>Hello from .anon!</h1></body></html>
HTML

# Start web server
python3 -m http.server 8080 -d ~/website &

# Register service
./register-service.sh

# Share the .anon address with Laptop 5
```

### 6. Access Service (Laptop 5 only)

```bash
# Get SOCKS5 port
cat data/socks5_port.txt

# Configure Firefox:
# Settings → Network → Manual proxy
# SOCKS v5: localhost:[PORT]
# ✅ Proxy DNS when using SOCKS v5

# Navigate to: http://YOUR_ANON_ADDRESS.anon/
```

## 📊 Scripts

### start.sh
- Starts the daemon
- Configures firewall
- Waits for network discovery
- Shows status

### stop.sh
- Gracefully stops daemon
- Preserves logs
- Safe to run anytime

### monitor.sh
- Shows network status
- Displays peer count
- Shows recent activity
- Node-specific info (SOCKS5 port, .anon address, etc.)

### register-service.sh (service node only)
- Registers website as .anon service
- Shows .anon address
- Monitors DHT replication

## 🔍 Troubleshooting

### Peers: 0

**On Laptop 1:**
```bash
# Check firewall
sudo ufw status

# Verify port forwarding works
# From another network: nc -zuv PUBLIC_IP 9000
```

**On other laptops:**
```bash
# Check bootstrap address in config.toml
cat config.toml | grep bootstrap

# Check logs
tail -50 *.log | grep -i "bootstrap\|connect"

# Wait 60 seconds for DHT discovery
```

### Service Not Found

**On Laptop 2:**
```bash
# Verify service registered
cat anon-address.txt

# Check DHT replication
tail *.log | grep -i "Sent STORE"
```

**On Laptop 5:**
```bash
# Wait 2 minutes after registration

# Check DHT lookup
tail *.log | grep -i "Querying.*nodes"
```

### Website Won't Load

```bash
# On Laptop 2: Check web server
curl http://localhost:8080

# On Laptop 5: Check SOCKS5
netstat -tulpn | grep $(cat data/socks5_port.txt)

# Check Firefox proxy settings
# SOCKS v5: localhost:[PORT]
# ✅ Proxy DNS = ON
```

## 📖 Full Documentation

See `../DEPLOYMENT_GUIDE.md` for complete deployment guide with:
- Detailed setup instructions
- Network topology diagrams
- Configuration examples
- Monitoring guides
- Security notes

## 💡 Tips

- **Bootstrap first**: Always start Laptop 1 before others
- **Wait for discovery**: Allow 60 seconds after starting each node
- **Check peers**: Should be > 0 on all nodes
- **Monitor logs**: `tail -f *.log` shows real-time activity
- **DHT propagation**: Wait 2-3 minutes after service registration

## 🔧 Maintenance

**Restart node:**
```bash
./stop.sh && sleep 2 && ./start.sh
```

**View live logs:**
```bash
tail -f *.log
```

**Clean data (reset):**
```bash
./stop.sh
rm -rf data/
mkdir data/
./start.sh
```

## ✅ Success Criteria

- ✅ All nodes show `Peers > 0`
- ✅ Laptop 2 shows "Sent STORE to node_XXX"
- ✅ Laptop 5 shows "Found descriptor from node_XXX"
- ✅ Firefox loads .anon website
- ✅ Encrypted multihop circuit working

---

**Ready to deploy? Start with Laptop 1 (bootstrap) and work your way through!** 🚀
