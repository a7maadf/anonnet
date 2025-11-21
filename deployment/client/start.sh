#!/bin/bash
# Start Client Node

set -e

echo "🚀 Starting AnonNet Client"
echo "=========================="
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
if [ -f client.pid ]; then
    OLD_PID=$(cat client.pid)
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
anonnet-daemon --config config.toml > client.log 2>&1 &
DAEMON_PID=$!
echo $DAEMON_PID > client.pid

echo "Daemon started (PID: $DAEMON_PID)"
echo ""

# Wait for DHT discovery
echo "Waiting for DHT discovery (60 seconds)..."
sleep 60

# Check peers
if [ -f data/api_port.txt ]; then
    API_PORT=$(cat data/api_port.txt)
    SOCKS_PORT=$(cat data/socks5_port.txt)
    STATS=$(curl -s http://localhost:$API_PORT/api/stats)
    PEERS=$(echo "$STATS" | jq -r '.peers // 0')

    echo "✅ Client node running!"
    echo ""
    echo "📊 Status:"
    echo "$STATS" | jq .
    echo ""

    if [ "$PEERS" -eq 0 ]; then
        echo "⚠️  No peers connected!"
        echo "   Check bootstrap address in config.toml"
        echo "   Wait 1-2 minutes for DHT discovery"
    else
        echo "✅ Connected to $PEERS peer(s)"
    fi

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "🌐 SOCKS5 Proxy Ready"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  localhost:$SOCKS_PORT"
    echo ""
    echo "Firefox Configuration:"
    echo "  1. Settings → Network Settings"
    echo "  2. Manual proxy configuration"
    echo "  3. SOCKS v5 Host: localhost"
    echo "  4. Port: $SOCKS_PORT"
    echo "  5. ✅ Proxy DNS when using SOCKS v5"
    echo ""
    echo "Then navigate to: http://YOUR_ANON_ADDRESS.anon/"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "💡 Monitor: tail -f client.log"
    echo "🛑 Stop: ./stop.sh"
    echo "📊 Status: ./monitor.sh"
else
    echo "⚠️ Daemon started but API not ready. Check client.log"
fi
