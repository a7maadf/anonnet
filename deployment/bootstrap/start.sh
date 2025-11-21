#!/bin/bash
# Start Bootstrap Node

set -e

echo "🚀 Starting AnonNet Bootstrap Node"
echo "==================================="
echo ""

# Check if daemon exists
if ! command -v anonnet-daemon &> /dev/null; then
    echo "❌ anonnet-daemon not found. Please build and install first."
    echo "   Run: cargo build --release && sudo cp target/release/anonnet-daemon /usr/local/bin/"
    exit 1
fi

# Create data directory
mkdir -p data

# Stop existing daemon
if [ -f bootstrap.pid ]; then
    OLD_PID=$(cat bootstrap.pid)
    if kill -0 $OLD_PID 2>/dev/null; then
        echo "Stopping existing daemon (PID: $OLD_PID)..."
        kill $OLD_PID
        sleep 2
    fi
fi

# Configure firewall
echo "Configuring firewall..."
sudo ufw allow 9000/udp 2>/dev/null || true

# Get public IP
PUBLIC_IP=$(curl -s ifconfig.me || echo "UNKNOWN")
echo "Your public IP: $PUBLIC_IP"
echo ""

echo "IMPORTANT: Share this bootstrap address with all friends:"
echo "  $PUBLIC_IP:9000"
echo ""
echo "If behind NAT, set up port forwarding:"
echo "  Router → Forward UDP port 9000 → This laptop"
echo ""
echo "Or use ngrok:"
echo "  ngrok tcp 9000"
echo ""

# Start daemon
echo "Starting daemon..."
anonnet-daemon --config config.toml > bootstrap.log 2>&1 &
DAEMON_PID=$!
echo $DAEMON_PID > bootstrap.pid

echo "Daemon started (PID: $DAEMON_PID)"
echo ""

# Wait for startup
echo "Waiting for startup..."
sleep 5

# Check if running
if ! kill -0 $DAEMON_PID 2>/dev/null; then
    echo "❌ Daemon failed to start. Check bootstrap.log"
    tail -20 bootstrap.log
    exit 1
fi

# Get API port
if [ -f data/api_port.txt ]; then
    API_PORT=$(cat data/api_port.txt)
    echo "✅ Bootstrap node running!"
    echo ""
    echo "📊 Status:"
    curl -s http://localhost:$API_PORT/api/stats | jq . 2>/dev/null || echo "API not ready yet"
    echo ""
    echo "📋 Info:"
    echo "  API Port: $API_PORT"
    echo "  Log file: bootstrap.log"
    echo "  PID file: bootstrap.pid"
    echo ""
    echo "💡 Monitor: tail -f bootstrap.log"
    echo "🛑 Stop: ./stop.sh"
else
    echo "⚠️ Daemon started but API not ready. Check bootstrap.log"
fi
