#!/bin/bash
# AnonNet Test Network Setup Script
# Creates directory structure and configuration files for local testing

set -e

echo "🚀 Setting up AnonNet test network infrastructure..."

# Create test directories
echo "📁 Creating test directories..."
mkdir -p ~/anonnet-test/{bootstrap,node1,node2,node3,node4}
mkdir -p ~/test-websites/site1

# Create bootstrap config
echo "⚙️  Creating bootstrap node config..."
cat > ~/anonnet-test/bootstrap/anonnet.toml << 'EOF'
listen_addr = "127.0.0.1"
listen_port = 9000
bootstrap_nodes = []
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF

# Create node configs
for i in 1 2 3 4; do
  echo "⚙️  Creating node$i config..."
  cat > ~/anonnet-test/node$i/anonnet.toml << EOF
listen_addr = "127.0.0.1"
listen_port = 900$i
bootstrap_nodes = ["127.0.0.1:9000"]
accept_relay = true
max_peers = 50
data_dir = "./data"
verbose = false
EOF
done

# Create test website
echo "🌐 Creating test website..."
cat > ~/test-websites/site1/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>AnonNet Infrastructure Test</title>
    <style>
        body {
            font-family: 'Courier New', monospace;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            background: #1a1a1a;
            color: #00ff00;
        }
        .success {
            color: #00ff00;
            font-size: 32px;
            text-align: center;
            margin: 40px 0;
        }
        .info-box {
            background: #2a2a2a;
            border: 2px solid #00ff00;
            padding: 20px;
            margin: 20px 0;
            border-radius: 5px;
        }
        code {
            background: #000;
            padding: 2px 8px;
            border-radius: 3px;
        }
        h1, h2 {
            text-align: center;
        }
    </style>
</head>
<body>
    <h1 class="success">✅ SUCCESS!</h1>
    <h2>AnonNet Infrastructure Test Page</h2>

    <div class="info-box">
        <h3>🎉 Congratulations!</h3>
        <p>If you can see this page, your AnonNet infrastructure is working correctly!</p>
    </div>

    <div class="info-box">
        <h3>📊 What This Means:</h3>
        <ul>
            <li>✅ .anon service generation working</li>
            <li>✅ DHT service publication successful</li>
            <li>✅ Circuit building operational</li>
            <li>✅ SOCKS5/HTTP proxy functional</li>
            <li>✅ End-to-end anonymous routing working</li>
        </ul>
    </div>

    <div class="info-box">
        <h3>🔒 Privacy Status:</h3>
        <p>Your connection is routing through multiple anonymous hops:</p>
        <p><code>You → Entry Node → Middle Node → Exit Node → This Service</code></p>
        <p>Neither the service nor intermediate nodes know your real IP address.</p>
    </div>

    <div class="info-box">
        <h3>⏱️ Timestamp:</h3>
        <p id="timestamp"></p>
    </div>

    <script>
        document.getElementById('timestamp').textContent =
            'Page loaded: ' + new Date().toLocaleString();
    </script>
</body>
</html>
EOF

# Create network management scripts
echo "📜 Creating network management scripts..."

cat > ~/anonnet-test/start-network.sh << 'EOF'
#!/bin/bash
set -e

echo "🚀 Starting AnonNet test network..."

# Start bootstrap
echo "Starting bootstrap node..."
cd ~/anonnet-test/bootstrap
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 >> bootstrap.log &
echo $! > bootstrap.pid
echo "  ✅ Bootstrap PID: $(cat bootstrap.pid)"

sleep 5

# Start relay nodes
for i in 1 2 3; do
  echo "Starting relay node $i..."
  cd ~/anonnet-test/node$i
  /home/user/anonnet/target/release/anonnet-daemon node 2>&1 >> node$i.log &
  echo $! > node$i.pid
  echo "  ✅ Node $i PID: $(cat node$i.pid)"
  sleep 2
done

# Start client node
echo "Starting client node 4..."
cd ~/anonnet-test/node4
/home/user/anonnet/target/release/anonnet-daemon node 2>&1 >> node4.log &
echo $! > node4.pid
echo "  ✅ Node 4 PID: $(cat node4.pid)"

echo ""
echo "⏳ Waiting 30 seconds for DHT discovery and network formation..."
sleep 30

echo ""
echo "✅ Network ready!"
echo ""
echo "Next steps:"
echo "  1. Run: ~/anonnet-test/health-check.sh"
echo "  2. Start test website: python3 -m http.server 8080 -d ~/test-websites/site1"
echo "  3. Publish service using Node 1's API"
EOF

cat > ~/anonnet-test/stop-network.sh << 'EOF'
#!/bin/bash

echo "🛑 Stopping AnonNet test network..."

# Stop all nodes
for node_dir in ~/anonnet-test/*/; do
  if [ -f "$node_dir/bootstrap.pid" ]; then
    PID=$(cat "$node_dir/bootstrap.pid")
    echo "Stopping bootstrap (PID $PID)..."
    kill $PID 2>/dev/null || true
  fi
  for i in 1 2 3 4 5; do
    if [ -f "$node_dir/node$i.pid" ]; then
      PID=$(cat "$node_dir/node$i.pid")
      echo "Stopping node $i (PID $PID)..."
      kill $PID 2>/dev/null || true
    fi
  done
done

# Kill test web servers
echo "Stopping test web servers..."
pkill -f "python3 -m http.server" || true

echo "✅ Network stopped"
EOF

cat > ~/anonnet-test/health-check.sh << 'EOF'
#!/bin/bash

echo "🏥 AnonNet Network Health Check"
echo "========================================"
echo ""

HEALTHY=0
UNHEALTHY=0

for node_dir in ~/anonnet-test/{bootstrap,node*}; do
  if [ -d "$node_dir" ] && [ -f "$node_dir/data/api_port.txt" ]; then
    API_PORT=$(cat "$node_dir/data/api_port.txt")
    NODE_NAME=$(basename "$node_dir")

    printf "%-12s " "$NODE_NAME:"

    if curl -s http://127.0.0.1:$API_PORT/health >/dev/null 2>&1; then
      STATUS=$(curl -s http://127.0.0.1:$API_PORT/api/network/status 2>/dev/null)
      if [ $? -eq 0 ]; then
        PEERS=$(echo "$STATUS" | jq -r '.peers // 0' 2>/dev/null || echo "0")
        CIRCUITS=$(echo "$STATUS" | jq -r '.circuits // 0' 2>/dev/null || echo "0")
        CREDITS=$(curl -s http://127.0.0.1:$API_PORT/api/credits/balance 2>/dev/null | jq -r '.balance // 0' || echo "0")
        printf "✅ Healthy | Peers: %-3s | Circuits: %-3s | Credits: %s\n" "$PEERS" "$CIRCUITS" "$CREDITS"
        HEALTHY=$((HEALTHY + 1))
      else
        echo "⚠️  Responding but API errors"
        UNHEALTHY=$((UNHEALTHY + 1))
      fi
    else
      echo "❌ Not responding"
      UNHEALTHY=$((UNHEALTHY + 1))
    fi
  fi
done

echo ""
echo "========================================"
echo "Summary: $HEALTHY healthy, $UNHEALTHY unhealthy"
echo "========================================"
EOF

chmod +x ~/anonnet-test/*.sh

echo ""
echo "✅ Test network setup complete!"
echo ""
echo "Directory structure created:"
echo "  ~/anonnet-test/bootstrap/    - Bootstrap node"
echo "  ~/anonnet-test/node1/         - Relay node 1"
echo "  ~/anonnet-test/node2/         - Relay node 2"
echo "  ~/anonnet-test/node3/         - Relay node 3"
echo "  ~/anonnet-test/node4/         - Client node"
echo "  ~/test-websites/site1/        - Test website"
echo ""
echo "Management scripts created:"
echo "  ~/anonnet-test/start-network.sh  - Start all nodes"
echo "  ~/anonnet-test/stop-network.sh   - Stop all nodes"
echo "  ~/anonnet-test/health-check.sh   - Check node status"
echo ""
echo "Next steps:"
echo "  1. Build AnonNet: cd /home/user/anonnet && cargo build --release"
echo "  2. Start network: ~/anonnet-test/start-network.sh"
echo "  3. Check health:  ~/anonnet-test/health-check.sh"
echo ""
echo "For full testing guide, see: /home/user/anonnet/TESTING_INFRASTRUCTURE.md"
