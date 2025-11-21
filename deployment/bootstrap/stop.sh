#!/bin/bash
# Stop AnonNet Daemon (works for any node type)

echo "🛑 Stopping AnonNet Daemon"
echo "=========================="
echo ""

# Find PID file
PID_FILE=$(ls *.pid 2>/dev/null | head -1)

if [ -z "$PID_FILE" ]; then
    echo "No PID file found. Daemon may not be running."
    exit 0
fi

PID=$(cat $PID_FILE)

if kill -0 $PID 2>/dev/null; then
    echo "Stopping daemon (PID: $PID)..."
    kill $PID
    sleep 2

    # Force kill if still running
    if kill -0 $PID 2>/dev/null; then
        echo "Force stopping..."
        kill -9 $PID
    fi

    echo "✅ Daemon stopped"
else
    echo "Daemon not running (PID $PID not found)"
fi

# Clean up PID file
rm -f $PID_FILE

echo ""
echo "Logs preserved in *.log files"
echo "To restart: ./start.sh"
