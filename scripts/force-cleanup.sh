#!/bin/bash
# Force cleanup - kills all AnonNet processes and frees ports

echo "🧹 Force cleaning up AnonNet processes..."

# Kill all anonnet-daemon processes
PIDS=$(pgrep -f anonnet-daemon || true)
if [ -n "$PIDS" ]; then
    echo "Killing anonnet-daemon processes: $PIDS"
    killall -9 anonnet-daemon 2>/dev/null || true
    sleep 2
fi

# Kill python test servers
pkill -f "python3 -m http.server" 2>/dev/null || true

# Remove PID files
echo "Removing PID files..."
rm -f ~/anonnet-test/*/bootstrap.pid 2>/dev/null || true
rm -f ~/anonnet-test/*/node*.pid 2>/dev/null || true

# Check if ports are free now
echo ""
echo "Checking ports..."
for port in 9000 9001 9002 9003 9004; do
    if lsof -i :$port >/dev/null 2>&1; then
        echo "⚠️  Port $port still in use"
        PID=$(lsof -t -i :$port 2>/dev/null)
        if [ -n "$PID" ]; then
            echo "   Killing PID $PID on port $port"
            kill -9 $PID 2>/dev/null || true
        fi
    else
        echo "✅ Port $port is free"
    fi
done

echo ""
echo "✅ Cleanup complete!"
echo ""
echo "You can now run: ~/anonnet-test/start-network.sh"
