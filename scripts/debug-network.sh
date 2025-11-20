#!/bin/bash
# AnonNet Debug Script - Diagnose network issues

echo "🔍 AnonNet Network Diagnostics"
echo "========================================"
echo ""

# Check if daemon binary exists
echo "1. Checking daemon binary..."
DAEMON_PATH="${HOME}/Documents/anonnet/target/release/anonnet-daemon"
if [ -f "$DAEMON_PATH" ]; then
    echo "   ✅ Daemon found at: $DAEMON_PATH"
else
    echo "   ❌ Daemon not found at: $DAEMON_PATH"
    echo "   Try: cd ~/Documents/anonnet && cargo build --release"
fi
echo ""

# Check for running processes
echo "2. Checking for running anonnet processes..."
PIDS=$(pgrep -f anonnet-daemon || true)
if [ -z "$PIDS" ]; then
    echo "   ℹ️  No anonnet-daemon processes running"
else
    echo "   ⚠️  Found running processes:"
    ps aux | grep anonnet-daemon | grep -v grep
fi
echo ""

# Check ports
echo "3. Checking if ports are in use..."
for port in 9000 9001 9002 9003 9004; do
    if lsof -i :$port >/dev/null 2>&1; then
        echo "   ⚠️  Port $port is in use:"
        lsof -i :$port 2>/dev/null | tail -n +2
    else
        if nc -z 127.0.0.1 $port 2>/dev/null; then
            echo "   ⚠️  Port $port appears to be in use"
        else
            echo "   ✅ Port $port is free"
        fi
    fi
done
echo ""

# Check test directories
echo "4. Checking test directory structure..."
TEST_DIR="${HOME}/anonnet-test"
if [ -d "$TEST_DIR" ]; then
    echo "   ✅ Test directory exists: $TEST_DIR"

    for node_dir in bootstrap node1 node2 node3 node4; do
        if [ -d "$TEST_DIR/$node_dir" ]; then
            echo "   ✅ $node_dir/ exists"

            # Check for config
            if [ -f "$TEST_DIR/$node_dir/anonnet.toml" ]; then
                echo "      ✅ anonnet.toml exists"
            else
                echo "      ❌ anonnet.toml missing"
            fi

            # Check for data directory
            if [ -d "$TEST_DIR/$node_dir/data" ]; then
                echo "      ✅ data/ directory exists"

                # Check for port files
                if [ -f "$TEST_DIR/$node_dir/data/api_port.txt" ]; then
                    API_PORT=$(cat "$TEST_DIR/$node_dir/data/api_port.txt")
                    echo "      ✅ API port file exists: $API_PORT"
                else
                    echo "      ℹ️  No API port file (node hasn't started successfully)"
                fi
            else
                echo "      ℹ️  data/ directory doesn't exist yet"
            fi
        else
            echo "   ❌ $node_dir/ missing"
        fi
    done
else
    echo "   ❌ Test directory not found: $TEST_DIR"
    echo "   Run: ${HOME}/Documents/anonnet/scripts/setup-test-network.sh"
fi
echo ""

# Check logs
echo "5. Checking recent logs..."
if [ -d "$TEST_DIR/bootstrap" ]; then
    if [ -f "$TEST_DIR/bootstrap/bootstrap.log" ]; then
        echo "   📋 Last 5 lines of bootstrap.log:"
        tail -5 "$TEST_DIR/bootstrap/bootstrap.log" | sed 's/^/      /'
    else
        echo "   ℹ️  No bootstrap.log found"
    fi
fi
echo ""

# Recommendations
echo "6. Recommendations:"
echo ""

if [ ! -f "$DAEMON_PATH" ]; then
    echo "   🔧 Build the daemon first:"
    echo "      cd ~/Documents/anonnet && cargo build --release"
    echo ""
fi

if [ -n "$PIDS" ]; then
    echo "   🔧 Kill existing processes:"
    echo "      killall -9 anonnet-daemon"
    echo "      # Or manually: kill -9 $PIDS"
    echo ""
fi

if nc -z 127.0.0.1 9000 2>/dev/null; then
    echo "   🔧 Port 9000 is still in use. Find what's using it:"
    echo "      lsof -i :9000"
    echo "      # Then kill it: kill -9 <PID>"
    echo ""
fi

echo "========================================"
