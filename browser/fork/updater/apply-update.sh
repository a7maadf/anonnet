#!/bin/bash
set -e

# AnonNet Browser Update Installer
# Downloads and installs browser updates

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Current installation directory
INSTALL_DIR="${INSTALL_DIR:-/opt/anonnet-browser}"
BACKUP_DIR="${BACKUP_DIR:-$HOME/.anonnet/browser-backups}"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  AnonNet Browser Update Installer${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if running as root (needed for system-wide install)
if [ "$INSTALL_DIR" = "/opt/anonnet-browser" ] && [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Root access required for system-wide update${NC}"
    echo "Run with sudo: sudo $0"
    exit 1
fi

# Parse arguments
UPDATE_URL="$1"
UPDATE_HASH="$2"

if [ -z "$UPDATE_URL" ]; then
    echo -e "${RED}Usage: $0 <update-url> [expected-hash]${NC}"
    echo ""
    echo "Example:"
    echo "  $0 https://github.com/.../anonnet-browser-1.1.0-linux-x86_64.tar.gz sha256:abc123..."
    exit 1
fi

# Create temporary download directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo -e "${YELLOW}[1/6] Downloading update...${NC}"
cd "$TEMP_DIR"

UPDATE_FILE=$(basename "$UPDATE_URL")
curl -L -o "$UPDATE_FILE" "$UPDATE_URL" || {
    echo -e "${RED}Download failed!${NC}"
    exit 1
}

echo -e "${GREEN}✓ Download complete${NC}"

# Verify hash if provided
if [ -n "$UPDATE_HASH" ]; then
    echo -e "${YELLOW}[2/6] Verifying integrity...${NC}"

    ACTUAL_HASH=$(sha256sum "$UPDATE_FILE" | awk '{print $1}')
    EXPECTED_HASH=$(echo "$UPDATE_HASH" | cut -d: -f2)

    if [ "$ACTUAL_HASH" != "$EXPECTED_HASH" ]; then
        echo -e "${RED}Hash mismatch!${NC}"
        echo "Expected: $EXPECTED_HASH"
        echo "Actual:   $ACTUAL_HASH"
        exit 1
    fi

    echo -e "${GREEN}✓ Integrity verified${NC}"
else
    echo -e "${YELLOW}[2/6] Skipping integrity check (no hash provided)${NC}"
fi

# Extract update
echo -e "${YELLOW}[3/6] Extracting update...${NC}"

if [[ "$UPDATE_FILE" == *.tar.gz ]]; then
    tar -xzf "$UPDATE_FILE"
elif [[ "$UPDATE_FILE" == *.zip ]]; then
    unzip -q "$UPDATE_FILE"
else
    echo -e "${RED}Unknown archive format${NC}"
    exit 1
fi

# Find extracted directory
EXTRACTED_DIR=$(find . -maxdepth 1 -type d -name "anonnet-browser-*" | head -n 1)

if [ -z "$EXTRACTED_DIR" ]; then
    echo -e "${RED}Could not find extracted browser directory${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Extraction complete${NC}"

# Backup current installation
echo -e "${YELLOW}[4/6] Backing up current installation...${NC}"

if [ -d "$INSTALL_DIR" ]; then
    mkdir -p "$BACKUP_DIR"
    BACKUP_NAME="backup-$(date +%Y%m%d-%H%M%S)"

    cp -r "$INSTALL_DIR" "$BACKUP_DIR/$BACKUP_NAME"
    echo -e "${GREEN}✓ Backup created: $BACKUP_DIR/$BACKUP_NAME${NC}"

    # Keep only last 3 backups
    ls -t "$BACKUP_DIR" | tail -n +4 | xargs -I {} rm -rf "$BACKUP_DIR/{}"
else
    echo -e "${YELLOW}No existing installation to backup${NC}"
fi

# Stop browser if running
echo -e "${YELLOW}[5/6] Checking for running instances...${NC}"

if pgrep -f "anonnet-browser" > /dev/null; then
    echo -e "${YELLOW}Browser is running. Please close it before updating.${NC}"
    read -p "Close browser and press Enter to continue..."

    # Wait for browser to close
    while pgrep -f "anonnet-browser" > /dev/null; do
        sleep 1
    done
fi

echo -e "${GREEN}✓ No running instances${NC}"

# Install update
echo -e "${YELLOW}[6/6] Installing update...${NC}"

# Remove old installation (except profile data)
if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
fi

# Move new version
mkdir -p "$(dirname "$INSTALL_DIR")"
mv "$EXTRACTED_DIR" "$INSTALL_DIR"

# Set permissions
chmod +x "$INSTALL_DIR/anonnet-browser"
chmod +x "$INSTALL_DIR/firefox" 2>/dev/null || true

echo -e "${GREEN}✓ Installation complete${NC}"

# Summary
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}  Update Installed Successfully!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Installation: $INSTALL_DIR"
echo "Backup:       $BACKUP_DIR/$(ls -t "$BACKUP_DIR" | head -n 1)"
echo ""
echo "You can now start AnonNet Browser:"
echo "  $INSTALL_DIR/anonnet-browser"
echo ""
echo -e "${YELLOW}Note: If anything goes wrong, restore from backup:${NC}"
echo "  sudo rm -rf $INSTALL_DIR"
echo "  sudo cp -r $BACKUP_DIR/$(ls -t "$BACKUP_DIR" | head -n 1) $INSTALL_DIR"
