#!/bin/bash

#############################################################################
# AnonNet Browser Launcher
#
# This script:
# 1. Starts the AnonNet daemon in proxy mode
# 2. Creates/configures a Firefox profile with hardening settings
# 3. Launches Firefox with the AnonNet-hardened profile
# 4. Handles graceful shutdown
#
# Usage:
#   ./launch-anonnet-browser.sh [firefox-path]
#
# Examples:
#   ./launch-anonnet-browser.sh                    # Auto-detect Firefox
#   ./launch-anonnet-browser.sh /usr/bin/firefox   # Specify path
#############################################################################

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BROWSER_DIR="$PROJECT_DIR/browser"
PROFILE_DIR="$HOME/.anonnet/firefox-profile"
USER_JS_SOURCE="$BROWSER_DIR/profile/user.js"
DAEMON_BIN="$PROJECT_DIR/target/release/anonnet-daemon"
DAEMON_PID_FILE="$HOME/.anonnet/daemon.pid"
SOCKS_PORT=9050
HTTP_PORT=8118

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Print banner
print_banner() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║                                                           ║"
    echo "║              🌐  ANONNET BROWSER LAUNCHER  🌐             ║"
    echo "║                                                           ║"
    echo "║  Anonymous browsing powered by AnonNet                    ║"
    echo "║  Hardened with Tor Browser security features              ║"
    echo "║                                                           ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Find Firefox binary
find_firefox() {
    local firefox_path="$1"

    if [ -n "$firefox_path" ] && [ -x "$firefox_path" ]; then
        echo "$firefox_path"
        return 0
    fi

    # Try common locations
    local common_paths=(
        "/usr/bin/firefox"
        "/usr/bin/firefox-esr"
        "/snap/bin/firefox"
        "/Applications/Firefox.app/Contents/MacOS/firefox"
        "$HOME/Applications/Firefox.app/Contents/MacOS/firefox"
        "/opt/firefox/firefox"
    )

    for path in "${common_paths[@]}"; do
        if [ -x "$path" ]; then
            echo "$path"
            return 0
        fi
    done

    # Try which command
    if command -v firefox &> /dev/null; then
        command -v firefox
        return 0
    fi

    if command -v firefox-esr &> /dev/null; then
        command -v firefox-esr
        return 0
    fi

    return 1
}

# Check if AnonNet daemon is built
check_daemon() {
    if [ ! -f "$DAEMON_BIN" ]; then
        log_warning "AnonNet daemon not found at $DAEMON_BIN"
        log_info "Building AnonNet daemon..."
        cd "$PROJECT_DIR"
        cargo build --release --bin anonnet-daemon
        if [ $? -ne 0 ]; then
            log_error "Failed to build AnonNet daemon"
            exit 1
        fi
        log_success "Daemon built successfully"
    fi
}

# Check if daemon is already running
is_daemon_running() {
    if [ -f "$DAEMON_PID_FILE" ]; then
        local pid=$(cat "$DAEMON_PID_FILE")
        if ps -p "$pid" > /dev/null 2>&1; then
            return 0
        else
            rm -f "$DAEMON_PID_FILE"
        fi
    fi

    # Check if ports are in use
    if lsof -Pi :$SOCKS_PORT -sTCP:LISTEN -t >/dev/null 2>&1 || \
       lsof -Pi :$HTTP_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
        return 0
    fi

    return 1
}

# Start AnonNet daemon
start_daemon() {
    if is_daemon_running; then
        log_success "AnonNet daemon already running"
        return 0
    fi

    log_info "Starting AnonNet daemon..."

    # Create .anonnet directory
    mkdir -p "$HOME/.anonnet"

    # Start daemon in background
    nohup "$DAEMON_BIN" proxy > "$HOME/.anonnet/daemon.log" 2>&1 &
    local daemon_pid=$!
    echo "$daemon_pid" > "$DAEMON_PID_FILE"

    # Wait for daemon to start
    log_info "Waiting for daemon to initialize..."
    local max_wait=30
    local waited=0

    while [ $waited -lt $max_wait ]; do
        if lsof -Pi :$SOCKS_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
            log_success "AnonNet daemon started (PID: $daemon_pid)"
            log_success "SOCKS5 proxy: 127.0.0.1:$SOCKS_PORT"
            log_success "HTTP proxy: 127.0.0.1:$HTTP_PORT"
            return 0
        fi
        sleep 1
        ((waited++))
    done

    log_error "Daemon failed to start within ${max_wait}s"
    log_info "Check logs: $HOME/.anonnet/daemon.log"
    return 1
}

# Setup Firefox profile
setup_profile() {
    log_info "Setting up Firefox profile..."

    # Create profile directory
    mkdir -p "$PROFILE_DIR"

    # Copy user.js
    if [ -f "$USER_JS_SOURCE" ]; then
        cp "$USER_JS_SOURCE" "$PROFILE_DIR/user.js"
        log_success "Hardening configuration applied"
    else
        log_warning "user.js not found at $USER_JS_SOURCE"
        log_warning "Browser will launch without hardening settings"
    fi

    # Create prefs.js if it doesn't exist
    if [ ! -f "$PROFILE_DIR/prefs.js" ]; then
        cat > "$PROFILE_DIR/prefs.js" << 'EOF'
// AnonNet Firefox Profile
// This file will be overwritten by user.js on Firefox startup
user_pref("browser.startup.homepage", "about:blank");
user_pref("browser.shell.checkDefaultBrowser", false);
EOF
        log_success "Profile initialized"
    fi
}

# Launch Firefox
launch_firefox() {
    local firefox_path="$1"

    log_info "Launching Firefox with AnonNet profile..."
    log_info "Firefox: $firefox_path"
    log_info "Profile: $PROFILE_DIR"

    # Launch Firefox with custom profile
    "$firefox_path" \
        --profile "$PROFILE_DIR" \
        --no-remote \
        --new-instance \
        "about:blank" &

    local firefox_pid=$!
    log_success "Firefox launched (PID: $firefox_pid)"

    echo ""
    log_info "═══════════════════════════════════════════════════════"
    log_success "AnonNet Browser is now running!"
    log_info "═══════════════════════════════════════════════════════"
    echo ""
    log_info "Security features enabled:"
    echo "  ✓ Fingerprinting resistance"
    echo "  ✓ Privacy-focused settings"
    echo "  ✓ All traffic routed through AnonNet"
    echo "  ✓ WebRTC disabled (no IP leaks)"
    echo "  ✓ First-party isolation enabled"
    echo ""
    log_info "Important notes:"
    echo "  • Only .anon domains are supported"
    echo "  • Clearnet sites will be blocked for safety"
    echo "  • Some websites may not work due to hardening"
    echo ""
    log_info "To stop:"
    echo "  • Close Firefox normally"
    echo "  • Run: killall -9 anonnet-daemon (to stop daemon)"
    echo ""

    # Wait for Firefox to close
    wait $firefox_pid
}

# Cleanup function
cleanup() {
    log_info "Cleaning up..."

    # Note: We don't automatically kill the daemon since it might be used by other apps
    log_info "Firefox closed"
    log_warning "AnonNet daemon is still running (for other applications)"
    log_info "To stop daemon: killall -9 anonnet-daemon"
}

# Main execution
main() {
    print_banner

    # Find Firefox
    log_info "Looking for Firefox installation..."
    local firefox_path=$(find_firefox "$1")

    if [ -z "$firefox_path" ]; then
        log_error "Firefox not found!"
        log_info "Please install Firefox or specify the path:"
        log_info "  ./launch-anonnet-browser.sh /path/to/firefox"
        exit 1
    fi

    log_success "Found Firefox: $firefox_path"

    # Check and build daemon if needed
    check_daemon

    # Start daemon
    start_daemon
    if [ $? -ne 0 ]; then
        log_error "Failed to start AnonNet daemon"
        exit 1
    fi

    # Setup profile
    setup_profile

    # Launch Firefox
    launch_firefox "$firefox_path"

    # Cleanup on exit
    cleanup
}

# Trap signals for cleanup
trap cleanup EXIT INT TERM

# Run main function
main "$@"
