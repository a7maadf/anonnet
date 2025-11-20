#!/bin/bash
# Monitor AnonNet Node Status

echo "🔍 AnonNet Node Monitor"
echo "======================="
echo ""

# Detect node type
if [ -f bootstrap.pid ]; then
    NODE_TYPE="Bootstrap"
    LOG_FILE="bootstrap.log"
elif [ -f service.pid ]; then
    NODE_TYPE="Service"
    LOG_FILE="service.log"
elif [ -f relay.pid ]; then
    NODE_TYPE="Relay"
    LOG_FILE="relay.log"
elif [ -f client.pid ]; then
    NODE_TYPE="Client"
    LOG_FILE="client.log"
else
    echo "❌ No daemon running (no PID file found)"
    exit 1
fi

echo "Node Type: $NODE_TYPE"
echo ""

# Check if daemon is running
PID=$(cat *.pid 2>/dev/null)
if ! kill -0 $PID 2>/dev/null; then
    echo "❌ Daemon not running (PID $PID not found)"
    echo "   Start with: ./start.sh"
    exit 1
fi

echo "✅ Daemon running (PID: $PID)"
echo ""

# Get API stats
if [ -f data/api_port.txt ]; then
    API_PORT=$(cat data/api_port.txt)
    echo "📊 Network Statistics:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    STATS=$(curl -s http://localhost:$API_PORT/api/stats)
    echo "$STATS" | jq .
    echo ""

    PEERS=$(echo "$STATS" | jq -r '.peers // 0')
    if [ "$PEERS" -gt 0 ]; then
        echo "✅ Connected to $PEERS peer(s)"
    else
        echo "⚠️  No peers connected"
        echo "   Check bootstrap address and wait 60 seconds"
    fi
else
    echo "⚠️  API port not found"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Show recent activity
echo ""
echo "📝 Recent Activity (last 10 lines):"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
tail -10 $LOG_FILE
echo ""

# Node-specific info
if [ "$NODE_TYPE" = "Service" ]; then
    if [ -f anon-address.txt ]; then
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "📋 .anon Address:"
        cat anon-address.txt
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "🔍 DHT Replication Activity:"
        grep -i "Replicating descriptor\|Sent STORE" $LOG_FILE | tail -5 || echo "  (none)"
    fi
elif [ "$NODE_TYPE" = "Client" ]; then
    if [ -f data/socks5_port.txt ]; then
        SOCKS_PORT=$(cat data/socks5_port.txt)
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "🌐 SOCKS5 Proxy:"
        echo "   localhost:$SOCKS_PORT"
        echo ""
        echo "Firefox Settings:"
        echo "  → Manual proxy configuration"
        echo "  → SOCKS v5 Host: localhost"
        echo "  → Port: $SOCKS_PORT"
        echo "  → ✅ Proxy DNS when using SOCKS v5"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "🔍 DHT Lookup Activity:"
        grep -i "Querying.*nodes\|Found descriptor from node" $LOG_FILE | tail -5 || echo "  (none)"
    fi
fi

echo ""
echo "💡 Commands:"
echo "  Live logs: tail -f $LOG_FILE"
echo "  Stop: ./stop.sh"
echo "  Restart: ./stop.sh && ./start.sh"
