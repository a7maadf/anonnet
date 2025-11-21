#!/bin/bash
#############################################################################
# AnonNet macOS Build Script
#
# This script builds AnonNet binaries for macOS (both Intel and Apple Silicon)
#
# Requirements:
# - macOS 11.0 or later
# - Rust toolchain installed (via rustup)
# - Xcode Command Line Tools
#
# Usage:
#   ./build-macos.sh
#
#############################################################################

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔══════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                                                      ║${NC}"
echo -e "${BLUE}║          AnonNet macOS Build Script                 ║${NC}"
echo -e "${BLUE}║                                                      ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo -e "${RED}Error: This script must be run on macOS${NC}"
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust is not installed${NC}"
    echo ""
    echo "Install Rust from: https://rustup.rs"
    echo "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo -e "${GREEN}✓ Rust found:${NC} $(cargo --version)"
echo ""

# Detect architecture
ARCH=$(uname -m)
if [[ "$ARCH" == "arm64" ]]; then
    echo -e "${BLUE}Building for Apple Silicon (ARM64)...${NC}"
    TARGET="aarch64-apple-darwin"
elif [[ "$ARCH" == "x86_64" ]]; then
    echo -e "${BLUE}Building for Intel (x86_64)...${NC}"
    TARGET="x86_64-apple-darwin"
else
    echo -e "${RED}Error: Unknown architecture: $ARCH${NC}"
    exit 1
fi

# Add target if not already added
rustup target add $TARGET

echo ""
echo -e "${YELLOW}[1/4] Building release binaries...${NC}"
cargo build --release --target $TARGET --bin anonnet-daemon --bin anonweb

echo ""
echo -e "${YELLOW}[2/4] Creating distribution directory...${NC}"
DIST_DIR="dist/anonnet-v1.0-macos-$ARCH"
mkdir -p "$DIST_DIR/bin"
mkdir -p "$DIST_DIR/browser"
mkdir -p "$DIST_DIR/docs"

echo ""
echo -e "${YELLOW}[3/4] Copying binaries...${NC}"
cp "target/$TARGET/release/anonnet-daemon" "$DIST_DIR/bin/"
cp "target/$TARGET/release/anonweb" "$DIST_DIR/bin/"

# Copy browser components
echo "Copying browser components..."
cp -r browser/extension "$DIST_DIR/browser/"
cp -r browser/profile "$DIST_DIR/browser/"
cp -r browser/scripts "$DIST_DIR/browser/"
cp -r browser/fork "$DIST_DIR/browser/"

# Copy documentation
echo "Copying documentation..."
cp dist/anonnet-v1.0/docs/README.md "$DIST_DIR/docs/"
cp dist/anonnet-v1.0/start.sh "$DIST_DIR/"

# Make scripts executable
chmod +x "$DIST_DIR/bin/"*
chmod +x "$DIST_DIR/start.sh"
chmod +x "$DIST_DIR/browser/scripts/"*.sh

echo ""
echo -e "${YELLOW}[4/4] Creating tarball...${NC}"
cd dist
if [[ "$ARCH" == "arm64" ]]; then
    tar -czf "anonnet-v1.0-macos-apple-silicon.tar.gz" "anonnet-v1.0-macos-$ARCH/"
    shasum -a 256 "anonnet-v1.0-macos-apple-silicon.tar.gz" > "anonnet-v1.0-macos-apple-silicon.tar.gz.sha256"
    TARBALL="anonnet-v1.0-macos-apple-silicon.tar.gz"
else
    tar -czf "anonnet-v1.0-macos-intel.tar.gz" "anonnet-v1.0-macos-$ARCH/"
    shasum -a 256 "anonnet-v1.0-macos-intel.tar.gz" > "anonnet-v1.0-macos-intel.tar.gz.sha256"
    TARBALL="anonnet-v1.0-macos-intel.tar.gz"
fi
cd ..

# Get file size
SIZE=$(du -h "dist/$TARBALL" | cut -f1)

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                                                      ║${NC}"
echo -e "${GREEN}║          Build Complete!                             ║${NC}"
echo -e "${GREEN}║                                                      ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}Distribution package:${NC} dist/$TARBALL ($SIZE)"
echo -e "${BLUE}Checksum file:${NC} dist/$TARBALL.sha256"
echo ""
echo -e "${GREEN}Binary sizes:${NC}"
ls -lh "$DIST_DIR/bin/"
echo ""
echo -e "${YELLOW}Bootstrap node configured: 37.114.50.194:9090${NC}"
echo ""
echo -e "${BLUE}To test locally:${NC}"
echo "  cd $DIST_DIR"
echo "  ./start.sh"
echo ""
echo -e "${BLUE}To distribute:${NC}"
echo "  dist/$TARBALL"
echo ""
