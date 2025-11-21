#!/bin/bash
# AnonNet Quick Start Script

echo "🌐 AnonNet Quick Start"
echo "====================="
echo ""

# Check if daemon exists
if [ ! -f "./bin/anonnet-daemon" ]; then
    echo "❌ Error: anonnet-daemon not found"
    echo "   Make sure you're in the distribution directory"
    exit 1
fi

echo "What would you like to do?"
echo ""
echo "1) Browse .anon sites (run as proxy)"
echo "2) Host a .anon website (anonweb)"
echo "3) Run as relay (earn credits)"
echo "4) Run as bootstrap node"
echo "5) Show my credit balance"
echo ""
read -p "Choose (1-5): " choice

case $choice in
    1)
        echo ""
        echo "🚀 Starting AnonNet proxy..."
        echo "   SOCKS5: 127.0.0.1:9050"
        echo "   HTTP:   127.0.0.1:8118"
        echo ""
        echo "Configure your browser to use SOCKS5 proxy 127.0.0.1:9050"
        echo "Or load the extension from: ./browser/extension/"
        echo ""
        echo "Press Ctrl+C to stop"
        echo ""
        ./bin/anonnet-daemon proxy
        ;;
    2)
        echo ""
        read -p "What port is your web server running on? (default: 8080): " port
        port=${port:-8080}
        echo ""
        echo "🌐 Generating .anon domain for port $port..."
        ./bin/anonweb $port
        ;;
    3)
        echo ""
        echo "🔄 Starting as relay node (you'll earn credits)..."
        echo "   Make sure port 9090 is accessible from the internet"
        echo ""
        echo "Press Ctrl+C to stop"
        echo ""
        ./bin/anonnet-daemon --accept-relay
        ;;
    4)
        echo ""
        echo "🏁 Starting as bootstrap node..."
        echo "   Other nodes will use your address to join the network"
        echo "   Make sure port 9090 is accessible from the internet"
        echo ""
        echo "Your bootstrap address:"
        echo "   $(hostname -I | awk '{print $1}'):9090"
        echo ""
        echo "Press Ctrl+C to stop"
        echo ""
        ./bin/anonnet-daemon
        ;;
    5)
        echo ""
        echo "💰 Checking credit balance..."

        # Try common API ports
        for port in 19150 19151 19152 19153; do
            if curl -s -f http://localhost:$port/api/credits/balance > /dev/null 2>&1; then
                balance=$(curl -s http://localhost:$port/api/credits/balance | grep -o '"balance":[0-9]*' | cut -d: -f2)
                echo ""
                echo "   Balance: $balance credits"
                echo ""
                curl -s http://localhost:$port/api/credits/stats | python3 -m json.tool
                exit 0
            fi
        done

        echo ""
        echo "❌ Daemon not running or API not accessible"
        echo "   Start the daemon first with option 1, 3, or 4"
        ;;
    *)
        echo ""
        echo "Invalid choice"
        exit 1
        ;;
esac
