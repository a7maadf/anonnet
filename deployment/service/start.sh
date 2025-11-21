#!/bin/bash
# Start Service Host Node

set -e

echo "🚀 Starting AnonNet Service Host"
echo "================================="
echo ""

# Check config
if grep -q "BOOTSTRAP_ADDRESS_HERE" config.toml; then
    echo "❌ ERROR: You must edit config.toml first!"
    echo "   Replace 'BOOTSTRAP_ADDRESS_HERE' with actual bootstrap address"
    echo "   Example: bootstrap_nodes = [\"1.2.3.4:9000\"]"
    exit 1
fi

# Check if daemon exists
if ! command -v anonnet-daemon &> /dev/null; then
    echo "❌ anonnet-daemon not found. Please build and install first."
    exit 1
fi

# Create data directory
mkdir -p data

# Stop existing daemon
if [ -f service.pid ]; then
    OLD_PID=$(cat service.pid)
    if kill -0 $OLD_PID 2>/dev/null; then
        echo "Stopping existing daemon..."
        kill $OLD_PID
        sleep 2
    fi
fi

# Configure firewall
echo "Configuring firewall..."
sudo ufw allow 9000/udp 2>/dev/null || true

# Start daemon
echo "Starting daemon..."
anonnet-daemon --config config.toml > service.log 2>&1 &
DAEMON_PID=$!
echo $DAEMON_PID > service.pid

echo "Daemon started (PID: $DAEMON_PID)"
echo ""

# Wait for DHT discovery
echo "Waiting for DHT discovery (60 seconds)..."
sleep 60

# Check peers
if [ -f data/api_port.txt ]; then
    API_PORT=$(cat data/api_port.txt)
    STATS=$(curl -s http://localhost:$API_PORT/api/stats)
    PEERS=$(echo "$STATS" | jq -r '.peers // 0')

    echo "✅ Service node running!"
    echo ""
    echo "📊 Status:"
    echo "$STATS" | jq .
    echo ""

    if [ "$PEERS" -eq 0 ]; then
        echo "⚠️  No peers connected!"
        echo "   Check bootstrap address in config.toml"
        echo "   Check bootstrap node is running"
        echo "   Wait 1-2 minutes for DHT discovery"
    else
        echo "✅ Connected to $PEERS peer(s)"
    fi

    echo ""
    echo "📋 Next Steps:"
    echo "  1. Create website: See DEPLOYMENT_GUIDE.md"
    echo "  2. Start web server: python3 -m http.server 8080 -d ~/website &"
    echo "  3. Register service: ./register-service.sh"
    echo ""
    echo "💡 Monitor: tail -f service.log"
    echo "🛑 Stop: ./stop.sh"
else
    echo "⚠️ Daemon started but API not ready. Check service.log"
fi
