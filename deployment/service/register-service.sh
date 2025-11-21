#!/bin/bash
# Register .anon Service

set -e

echo "📝 Registering .anon Service"
echo "============================"
echo ""

# Check API port exists
if [ ! -f data/api_port.txt ]; then
    echo "❌ Daemon not running. Start it first with ./start.sh"
    exit 1
fi

API_PORT=$(cat data/api_port.txt)

# Check web server is running
if ! nc -z localhost 8080 2>/dev/null; then
    echo "❌ Web server not running on port 8080"
    echo "   Start it with: python3 -m http.server 8080 -d ~/website &"
    exit 1
fi

echo "Registering service..."
RESULT=$(curl -s -X POST http://localhost:$API_PORT/api/services/register \
  -H "Content-Type: application/json" \
  -d '{"local_host":"127.0.0.1","local_port":8080,"ttl_hours":24}')

ANON_ADDRESS=$(echo "$RESULT" | jq -r '.anon_address')

if [ "$ANON_ADDRESS" = "null" ] || [ -z "$ANON_ADDRESS" ]; then
    echo "❌ Registration failed!"
    echo "$RESULT" | jq .
    exit 1
fi

# Save address
echo "$ANON_ADDRESS" > anon-address.txt

echo "✅ Service registered!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 .anon Address:"
echo "   $ANON_ADDRESS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "IMPORTANT: Share this address with the person at Laptop 5 (Client)"
echo ""
echo "Full details:"
echo "$RESULT" | jq .
echo ""
echo "📊 Check DHT replication:"
tail -20 service.log | grep -i "Replicating descriptor\|Sent STORE" || echo "  (No replication logs yet - wait 30 seconds)"
