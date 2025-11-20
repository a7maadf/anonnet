# AnonNet Infrastructure Testing Guide

**Purpose**: Test the entire AnonNet infrastructure with zero external users by creating a local test network.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Phase 1: Single Node Testing](#phase-1-single-node-testing)
3. [Phase 2: Multi-Node Local Network](#phase-2-multi-node-local-network)
4. [Phase 3: .anon Service Hosting](#phase-3-anon-service-hosting)
5. [Phase 4: Browser Integration](#phase-4-browser-integration)
6. [Phase 5: Credit System Testing](#phase-5-credit-system-testing)
7. [Phase 6: End-to-End Integration](#phase-6-end-to-end-integration)
8. [Automated Testing](#automated-testing)
9. [Performance Testing](#performance-testing)
10. [Troubleshooting](#troubleshooting)

---

## Prerequisites

```bash
# 1. Build the project
cd /home/user/anonnet
cargo build --release

# 2. Verify tests pass
cargo test --all

# 3. Create testing directories
mkdir -p ~/anonnet-test/{node1,node2,node3,node4,bootstrap}
mkdir -p ~/test-websites/site1
```

---

## Phase 1: Single Node Testing

**Goal**: Verify a single node can start, initialize, and respond to API calls.

### Step 1.1: Create Bootstrap Config

```bash
# Create bootstrap node configuration
cd ~/anonnet-test/bootstrap

cat > anonnet.toml << 'EOF'
listen_addr = "127.0.0.1"
listen_port = 9000
bootstrap_nodes = []
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF
```

### Step 1.2: Start Bootstrap Node

```bash
# Terminal 1: Bootstrap node (acts as initial peer discovery point)
cd ~/anonnet-test/bootstrap
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 | tee bootstrap.log
```

**Expected output:**
```
========================================
         AnonNet Node Status
========================================
Node ID:          [16-char hex]
Status:           Running
Peers:            0 (bootstrap has no peers initially)
Active Peers:     0
Circuits:         0
Active Circuits:  0
Bandwidth:        0 bytes/sec
Listen Address:   127.0.0.1:9000
========================================

SOCKS5 proxy:     127.0.0.1:[auto-port]
HTTP proxy:       127.0.0.1:[auto-port]
API server:       127.0.0.1:[auto-port]
```

### Step 1.3: Test API Endpoints

```bash
# In another terminal, read the API port
API_PORT=$(cat ~/anonnet-test/bootstrap/data/api_port.txt)

# Test health endpoint
curl http://127.0.0.1:$API_PORT/health
# Expected: {"status":"healthy"}

# Get credit balance
curl http://127.0.0.1:$API_PORT/api/credits/balance
# Expected: {"balance":1000,"node_id":"..."}

# Get network status
curl http://127.0.0.1:$API_PORT/api/network/status
# Expected: {"peers":0,"circuits":0,"bandwidth":0}

# Get active circuits
curl http://127.0.0.1:$API_PORT/api/circuits/active
# Expected: {"circuits":[]}
```

### Step 1.4: Test Proxy Startup

```bash
# Test SOCKS5 proxy
SOCKS_PORT=$(cat ~/anonnet-test/bootstrap/data/socks5_port.txt)
echo $SOCKS_PORT
# Should print a port number (e.g., 53175)

# Test HTTP proxy
HTTP_PORT=$(cat ~/anonnet-test/bootstrap/data/http_port.txt)
echo $HTTP_PORT
# Should print a port number
```

**✅ Success criteria:**
- Node starts without errors
- All API endpoints respond
- Port files are created
- Logs show "Running" status

---

## Phase 2: Multi-Node Local Network

**Goal**: Create a local network with 4 nodes + 1 bootstrap to test peer discovery and circuit building.

### Step 2.1: Create Node Configs

```bash
# Create Node 1 config (relay)
cd ~/anonnet-test/node1
cat > anonnet.toml << 'EOF'
listen_addr = "127.0.0.1"
listen_port = 9001
bootstrap_nodes = ["127.0.0.1:9000"]
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF

# Create Node 2 config (relay)
cd ~/anonnet-test/node2
cat > anonnet.toml << 'EOF'
listen_addr = "127.0.0.1"
listen_port = 9002
bootstrap_nodes = ["127.0.0.1:9000"]
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF

# Create Node 3 config (relay)
cd ~/anonnet-test/node3
cat > anonnet.toml << 'EOF'
listen_addr = "127.0.0.1"
listen_port = 9003
bootstrap_nodes = ["127.0.0.1:9000"]
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF

# Create Node 4 config (client)
cd ~/anonnet-test/node4
cat > anonnet.toml << 'EOF'
listen_addr = "127.0.0.1"
listen_port = 9004
bootstrap_nodes = ["127.0.0.1:9000"]
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF
```

### Step 2.2: Start Multiple Nodes

```bash
# Terminal 2: Node 1 (relay)
cd ~/anonnet-test/node1
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 | tee node1.log

# Terminal 3: Node 2 (relay)
cd ~/anonnet-test/node2
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 | tee node2.log

# Terminal 4: Node 3 (relay)
cd ~/anonnet-test/node3
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 | tee node3.log

# Terminal 5: Node 4 (client)
cd ~/anonnet-test/node4
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 | tee node4.log
```

### Step 2.3: Verify Peer Discovery

Wait 30 seconds for DHT discovery, then check peer counts:

```bash
# Check each node's peer count
for port in $(cat ~/anonnet-test/node*/data/api_port.txt); do
  echo "Node on API port $port:"
  curl -s http://127.0.0.1:$port/api/network/status | jq '.peers'
done
```

**Expected output:**
```
Node on API port 53180: 3-4
Node on API port 53181: 3-4
Node on API port 53182: 3-4
Node on API port 53183: 3-4
```

### Step 2.4: Monitor Network Formation

```bash
# Watch logs in real-time
tail -f ~/anonnet-test/node1/node1.log | grep -i "peer\|dht\|circuit"
```

**Look for:**
- `[DHT] Added peer to routing table`
- `[Peer] Connected to peer [node-id]`
- `[Circuit] Building circuit with 3 hops`

**✅ Success criteria:**
- All nodes have 3-4 peers
- DHT routing tables populated
- No connection errors in logs

---

## Phase 3: .anon Service Hosting

**Goal**: Generate a .anon domain and host a test website.

### Step 3.1: Create Test Website

```bash
# Create a simple test website
cd ~/test-websites/site1

cat > index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Test .anon Site</title>
    <style>
        body { font-family: Arial; max-width: 800px; margin: 50px auto; }
        .success { color: green; font-size: 24px; }
    </style>
</head>
<body>
    <h1 class="success">✅ AnonNet Infrastructure Test</h1>
    <p>If you can see this, the .anon service is working!</p>
    <p>Node ID: <code id="node-id">Loading...</code></p>
    <p>Service Address: <code id="service-addr">Loading...</code></p>
    <script>
        // Display test info
        document.getElementById('node-id').textContent =
            window.location.hostname;
    </script>
</body>
</html>
EOF

# Start HTTP server
python3 -m http.server 8080 2>&1 | tee server.log &
echo $! > server.pid

# Test local access
curl http://127.0.0.1:8080
# Should show HTML content
```

### Step 3.2: Configure .anon Service

```bash
# Create service configuration
cat > ~/test-websites/site1/service.toml << 'EOF'
[service]
name = "test-site-1"
description = "Infrastructure test website"
local_host = "127.0.0.1"
local_port = 8080
protocol = "http"
public = true
num_introduction_points = 3

[identity]
# Will auto-generate on first run
EOF
```

### Step 3.3: Publish Service

We'll use Node1 to host the service:

```bash
# Option A: Using API (programmatic)
NODE1_API=$(cat ~/anonnet-test/node1/data/api_port.txt)

curl -X POST http://127.0.0.1:$NODE1_API/api/services/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-site-1",
    "local_host": "127.0.0.1",
    "local_port": 8080,
    "protocol": "http",
    "introduction_points": 3
  }' | jq

# Option B: Restart node with service config
# Stop node1, add --service-config flag, restart
```

**Expected response:**
```json
{
  "service_address": "abc123def456ghi789jkl.anon",
  "public_key": "ed25519:...",
  "introduction_points": [
    "node-id-1",
    "node-id-2",
    "node-id-3"
  ],
  "published": true,
  "descriptor_hash": "..."
}
```

### Step 3.4: Verify Service Published

```bash
# Save the .anon address
ANON_ADDR="abc123def456ghi789jkl.anon"  # Replace with actual

# Check service is listed
curl http://127.0.0.1:$NODE1_API/api/services/list | jq

# Get service descriptor
curl http://127.0.0.1:$NODE1_API/api/services/test-site-1/descriptor | jq

# Check introduction points health
curl http://127.0.0.1:$NODE1_API/api/services/test-site-1/health | jq
```

**✅ Success criteria:**
- Service address generated (format: `[hash].anon`)
- 3 introduction points selected
- Descriptor published to DHT
- Service appears in list

---

## Phase 4: Browser Integration

**Goal**: Access the .anon service through the SOCKS5/HTTP proxy using a browser.

### Step 4.1: Manual Browser Test

```bash
# Get Node4's proxy ports (we'll use this as client)
NODE4_SOCKS=$(cat ~/anonnet-test/node4/data/socks5_port.txt)
NODE4_HTTP=$(cat ~/anonnet-test/node4/data/http_port.txt)

echo "SOCKS5 proxy: 127.0.0.1:$NODE4_SOCKS"
echo "HTTP proxy:   127.0.0.1:$NODE4_HTTP"
```

**Configure Firefox manually:**
1. Open Firefox
2. Settings → Network Settings → Manual proxy configuration
3. SOCKS Host: `127.0.0.1`, Port: `$NODE4_SOCKS`
4. Select "SOCKS v5"
5. Enable "Proxy DNS when using SOCKS v5"
6. Save

**Visit the .anon site:**
```
http://abc123def456ghi789jkl.anon
```

### Step 4.2: Test with curl (Easier)

```bash
# Test SOCKS5 proxy
curl --proxy socks5h://127.0.0.1:$NODE4_SOCKS \
  http://$ANON_ADDR

# Test HTTP proxy
curl --proxy http://127.0.0.1:$NODE4_HTTP \
  http://$ANON_ADDR

# Should return the HTML content
```

### Step 4.3: Test AnonNet Browser

```bash
# Launch hardened browser with Node4's proxies
cd /home/user/anonnet
ANONNET_SOCKS_PORT=$NODE4_SOCKS \
ANONNET_HTTP_PORT=$NODE4_HTTP \
./browser/scripts/launch-anonnet-browser.sh
```

**In browser:**
1. Navigate to `http://$ANON_ADDR`
2. Install extension from `browser/extension/`
3. Check extension shows credit balance
4. Verify network stats in extension

### Step 4.4: Test Clearnet Blocking

```bash
# Try to access clearnet address (should fail)
curl --proxy socks5h://127.0.0.1:$NODE4_SOCKS \
  http://example.com

# Expected: Connection rejected or error
```

**✅ Success criteria:**
- .anon site loads successfully
- Content displays correctly
- Clearnet addresses blocked
- Browser extension shows stats

---

## Phase 5: Credit System Testing

**Goal**: Verify credits are earned and spent correctly.

### Step 5.1: Check Initial Balances

```bash
# Check all nodes' initial credits
for node_dir in ~/anonnet-test/node*; do
  API_PORT=$(cat $node_dir/data/api_port.txt)
  echo "Node: $node_dir"
  curl -s http://127.0.0.1:$API_PORT/api/credits/balance | jq
  echo ""
done
```

**Expected:** All nodes start with 1000 credits (or more if PoW difficulty > 8)

### Step 5.2: Generate Traffic

```bash
# From Node4 (client), make multiple requests to service
for i in {1..10}; do
  curl --proxy socks5h://127.0.0.1:$NODE4_SOCKS \
    http://$ANON_ADDR > /dev/null 2>&1
  echo "Request $i completed"
  sleep 2
done
```

### Step 5.3: Check Credit Changes

```bash
# Check Node4 (client - should spend credits)
NODE4_API=$(cat ~/anonnet-test/node4/data/api_port.txt)
curl http://127.0.0.1:$NODE4_API/api/credits/stats | jq

# Expected:
# {
#   "balance": 990,  # Decreased
#   "earned": 0,
#   "spent": 10,
#   "transactions": [...]
# }

# Check relay nodes (should earn credits)
NODE1_API=$(cat ~/anonnet-test/node1/data/api_port.txt)
curl http://127.0.0.1:$NODE1_API/api/credits/stats | jq

# Expected:
# {
#   "balance": 1003,  # Increased
#   "earned": 3,
#   "spent": 0,
#   "transactions": [...]
# }
```

### Step 5.4: Monitor Consensus

```bash
# Check validator set
curl http://127.0.0.1:$NODE1_API/api/consensus/validators | jq

# Check latest block
curl http://127.0.0.1:$NODE1_API/api/consensus/latest-block | jq

# Expected:
# {
#   "block_number": 5,
#   "timestamp": ...,
#   "transactions": [...],
#   "validator": "..."
# }
```

**✅ Success criteria:**
- Client node balance decreases
- Relay nodes balance increases
- Credits earned ≈ credits spent (conservation)
- Transactions recorded in blockchain

---

## Phase 6: End-to-End Integration

**Goal**: Full workflow test - multiple clients, multiple services, realistic usage.

### Step 6.1: Host Multiple Services

```bash
# Create second website
mkdir ~/test-websites/site2
cd ~/test-websites/site2

cat > index.html << 'EOF'
<h1>Site 2: File Sharing</h1>
<ul>
  <li><a href="/file1.txt">File 1</a></li>
  <li><a href="/file2.txt">File 2</a></li>
</ul>
EOF

echo "Test file 1" > file1.txt
echo "Test file 2" > file2.txt

python3 -m http.server 8081 &

# Publish on Node2
NODE2_API=$(cat ~/anonnet-test/node2/data/api_port.txt)
curl -X POST http://127.0.0.1:$NODE2_API/api/services/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-site-2",
    "local_host": "127.0.0.1",
    "local_port": 8081,
    "protocol": "http",
    "introduction_points": 3
  }' | jq

# Save the new .anon address
ANON_ADDR_2="xyz789abc123def456ghi.anon"  # Replace with actual
```

### Step 6.2: Test Circuit Isolation

Verify that different services use different circuits:

```bash
# Request site1
curl --proxy socks5h://127.0.0.1:$NODE4_SOCKS http://$ANON_ADDR

# Check active circuits
curl http://127.0.0.1:$NODE4_API/api/circuits/active | jq
# Note the circuit IDs

# Request site2
curl --proxy socks5h://127.0.0.1:$NODE4_SOCKS http://$ANON_ADDR_2

# Check circuits again - should see new circuit
curl http://127.0.0.1:$NODE4_API/api/circuits/active | jq
```

### Step 6.3: Concurrent Load Test

```bash
# Simulate multiple simultaneous users
for i in {1..20}; do
  (
    # Random site
    SITE=$([ $((RANDOM % 2)) -eq 0 ] && echo $ANON_ADDR || echo $ANON_ADDR_2)
    curl --proxy socks5h://127.0.0.1:$NODE4_SOCKS http://$SITE \
      > /dev/null 2>&1
    echo "Request $i to $SITE completed"
  ) &
done

wait
echo "All concurrent requests completed"

# Check network stats
curl http://127.0.0.1:$NODE4_API/api/network/status | jq
```

### Step 6.4: Service Discovery Test

Test that a fresh client can discover published services:

```bash
# Create Node 5 config
mkdir ~/anonnet-test/node5
cd ~/anonnet-test/node5

cat > anonnet.toml << 'EOF'
listen_addr = "127.0.0.1"
listen_port = 9005
bootstrap_nodes = ["127.0.0.1:9000"]
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF

# Start node
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 | tee node5.log &

sleep 30  # Wait for DHT sync

# Try to access services from Node5
NODE5_SOCKS=$(cat ~/anonnet-test/node5/data/socks5_port.txt)
curl --proxy socks5h://127.0.0.1:$NODE5_SOCKS http://$ANON_ADDR
```

**✅ Success criteria:**
- Multiple services accessible
- Different circuits for different services
- Concurrent requests handled
- New nodes discover existing services via DHT

---

## Automated Testing

Create test scripts for repeated testing:

### Script 1: Network Startup

```bash
cat > ~/anonnet-test/start-network.sh << 'EOF'
#!/bin/bash
set -e

echo "Starting AnonNet test network..."

# Start bootstrap
cd ~/anonnet-test/bootstrap
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 >> bootstrap.log &
echo $! > bootstrap.pid

sleep 5

# Start relay nodes
for i in 1 2 3; do
  cd ~/anonnet-test/node$i
  /home/user/anonnet/target/release/anonnet-daemon node 2>&1 >> node$i.log &
  echo $! > node$i.pid
  sleep 2
done

# Start client node
cd ~/anonnet-test/node4
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 >> node4.log &
echo $! > node4.pid

echo "Network started. Waiting 30s for DHT..."
sleep 30

echo "✅ Network ready!"
EOF

chmod +x ~/anonnet-test/start-network.sh
```

### Script 2: Network Shutdown

```bash
cat > ~/anonnet-test/stop-network.sh << 'EOF'
#!/bin/bash

echo "Stopping AnonNet test network..."

for node_dir in ~/anonnet-test/*/; do
  if [ -f "$node_dir/bootstrap.pid" ]; then
    kill $(cat "$node_dir/bootstrap.pid") 2>/dev/null || true
  fi
  for i in 1 2 3 4 5; do
    if [ -f "$node_dir/node$i.pid" ]; then
      kill $(cat "$node_dir/node$i.pid") 2>/dev/null || true
    fi
  done
done

# Kill test web servers
pkill -f "python3 -m http.server" || true

echo "✅ Network stopped"
EOF

chmod +x ~/anonnet-test/stop-network.sh
```

### Script 3: Health Check

```bash
cat > ~/anonnet-test/health-check.sh << 'EOF'
#!/bin/bash

echo "AnonNet Network Health Check"
echo "================================"

for node_dir in ~/anonnet-test/node*; do
  if [ -f "$node_dir/data/api_port.txt" ]; then
    API_PORT=$(cat "$node_dir/data/api_port.txt")
    NODE_NAME=$(basename "$node_dir")

    echo -n "$NODE_NAME: "
    HEALTH=$(curl -s http://127.0.0.1:$API_PORT/health 2>/dev/null)

    if [ $? -eq 0 ]; then
      PEERS=$(curl -s http://127.0.0.1:$API_PORT/api/network/status | jq -r '.peers')
      CIRCUITS=$(curl -s http://127.0.0.1:$API_PORT/api/network/status | jq -r '.circuits')
      CREDITS=$(curl -s http://127.0.0.1:$API_PORT/api/credits/balance | jq -r '.balance')
      echo "✅ Healthy | Peers: $PEERS | Circuits: $CIRCUITS | Credits: $CREDITS"
    else
      echo "❌ Unhealthy or not responding"
    fi
  fi
done
EOF

chmod +x ~/anonnet-test/health-check.sh
```

### Script 4: E2E Test

```bash
cat > ~/anonnet-test/e2e-test.sh << 'EOF'
#!/bin/bash
set -e

echo "Running End-to-End Test..."

# Get client proxy port
NODE4_SOCKS=$(cat ~/anonnet-test/node4/data/socks5_port.txt)

# Test service access
echo -n "Testing .anon service access... "
RESPONSE=$(curl -s --proxy socks5h://127.0.0.1:$NODE4_SOCKS \
  http://abc123def456ghi789jkl.anon)  # Replace with actual address

if echo "$RESPONSE" | grep -q "AnonNet Infrastructure Test"; then
  echo "✅ PASS"
else
  echo "❌ FAIL"
  exit 1
fi

# Test clearnet blocking
echo -n "Testing clearnet blocking... "
if curl -s --proxy socks5h://127.0.0.1:$NODE4_SOCKS \
  http://example.com 2>&1 | grep -q "rejected\|failed\|error"; then
  echo "✅ PASS"
else
  echo "❌ FAIL (clearnet not blocked)"
  exit 1
fi

echo "✅ All tests passed!"
EOF

chmod +x ~/anonnet-test/e2e-test.sh
```

---

## Performance Testing

### Latency Test

```bash
cat > ~/anonnet-test/latency-test.sh << 'EOF'
#!/bin/bash

NODE4_SOCKS=$(cat ~/anonnet-test/node4/data/socks5_port.txt)
ANON_ADDR="abc123def456ghi789jkl.anon"  # Replace

echo "Testing latency over 10 requests..."

for i in {1..10}; do
  START=$(date +%s%N)
  curl -s --proxy socks5h://127.0.0.1:$NODE4_SOCKS \
    http://$ANON_ADDR > /dev/null
  END=$(date +%s%N)

  LATENCY=$(( (END - START) / 1000000 ))  # Convert to ms
  echo "Request $i: ${LATENCY}ms"
done
EOF

chmod +x ~/anonnet-test/latency-test.sh
```

### Bandwidth Test

```bash
cat > ~/anonnet-test/bandwidth-test.sh << 'EOF'
#!/bin/bash

# Create large test file
dd if=/dev/urandom of=~/test-websites/site1/largefile.bin bs=1M count=10

NODE4_SOCKS=$(cat ~/anonnet-test/node4/data/socks5_port.txt)
ANON_ADDR="abc123def456ghi789jkl.anon"  # Replace

echo "Testing bandwidth with 10MB download..."

START=$(date +%s)
curl -s --proxy socks5h://127.0.0.1:$NODE4_SOCKS \
  http://$ANON_ADDR/largefile.bin -o /tmp/download.bin
END=$(date +%s)

DURATION=$((END - START))
SIZE=$(stat -f%z /tmp/download.bin)
BW=$((SIZE / DURATION / 1024))  # KB/s

echo "Downloaded ${SIZE} bytes in ${DURATION}s"
echo "Bandwidth: ${BW} KB/s"
EOF

chmod +x ~/anonnet-test/bandwidth-test.sh
```

---

## Troubleshooting

### Issue: Nodes can't discover each other

**Check:**
```bash
# Verify bootstrap node is running
curl http://127.0.0.1:$(cat ~/anonnet-test/bootstrap/data/api_port.txt)/health

# Check firewall (if using real network instead of localhost)
# Make sure UDP/TCP ports 9000-9004 are open

# Check logs for connection errors
grep -i "error\|failed" ~/anonnet-test/node1/node1.log
```

### Issue: Circuit building fails

**Check:**
```bash
# Verify sufficient peers
curl http://127.0.0.1:$NODE4_API/api/network/status

# Need at least 3 peers for 3-hop circuit
# If peers < 3, wait longer or add more nodes

# Check circuit logs
grep -i "circuit" ~/anonnet-test/node4/node4.log
```

### Issue: .anon service not accessible

**Check:**
```bash
# Verify service published
curl http://127.0.0.1:$NODE1_API/api/services/list

# Check introduction points
curl http://127.0.0.1:$NODE1_API/api/services/test-site-1/health

# Verify local HTTP server running
curl http://127.0.0.1:8080

# Check service logs
tail -f ~/test-websites/site1/server.log
```

### Issue: Credits not updating

**Check:**
```bash
# Verify consensus is running
curl http://127.0.0.1:$NODE1_API/api/consensus/latest-block

# Check validator set (need 21+ validators for production)
curl http://127.0.0.1:$NODE1_API/api/consensus/validators

# For testing with <21 nodes, consensus may be in development mode
# Check logs for consensus messages
grep -i "consensus\|validator\|block" ~/anonnet-test/node1/node1.log
```

---

## Success Checklist

After completing all phases:

- [ ] Single node starts and responds to API
- [ ] Multi-node network forms (4+ peers)
- [ ] DHT routing tables populated
- [ ] .anon domain generated
- [ ] Service descriptor published to DHT
- [ ] Service accessible via SOCKS5 proxy
- [ ] Service accessible via HTTP proxy
- [ ] Browser integration works
- [ ] Clearnet addresses blocked
- [ ] Credits decrease on client
- [ ] Credits increase on relays
- [ ] Consensus producing blocks
- [ ] Multiple services hosted
- [ ] Circuit isolation verified
- [ ] New nodes discover existing services
- [ ] Automated scripts work

---

## Next Steps

After successful infrastructure testing:

1. **Document Results**: Create test report with metrics
2. **Fix Issues**: Address any failures or performance problems
3. **Optimize**: Tune parameters based on test results
4. **Production Deploy**: Set up real network with external nodes
5. **Security Audit**: Test attack resistance
6. **User Testing**: Invite beta testers

---

## Quick Start Command Sequence

For quick testing:

```bash
# 1. Start network
~/anonnet-test/start-network.sh

# 2. Check health
~/anonnet-test/health-check.sh

# 3. Start test website
cd ~/test-websites/site1 && python3 -m http.server 8080 &

# 4. Publish service (use actual Node1 API port)
NODE1_API=$(cat ~/anonnet-test/node1/data/api_port.txt)
curl -X POST http://127.0.0.1:$NODE1_API/api/services/register \
  -H "Content-Type: application/json" \
  -d '{"name":"test","local_host":"127.0.0.1","local_port":8080,"protocol":"http"}' \
  | jq -r '.service_address'

# 5. Test access (use actual addresses/ports)
NODE4_SOCKS=$(cat ~/anonnet-test/node4/data/socks5_port.txt)
curl --proxy socks5h://127.0.0.1:$NODE4_SOCKS http://[service-address].anon

# 6. Run E2E test
~/anonnet-test/e2e-test.sh

# 7. Clean up
~/anonnet-test/stop-network.sh
```
