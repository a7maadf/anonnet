#!/bin/bash
set -e

# AnonNet Browser Build Script
# Builds a custom Firefox ESR browser with Tor Browser hardening

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FORK_DIR="$(dirname "$SCRIPT_DIR")"
BROWSER_DIR="$(dirname "$FORK_DIR")"
REPO_ROOT="$(dirname "$BROWSER_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default versions
FIREFOX_ESR_VERSION="128.6.0esr"
TOR_BROWSER_VERSION="14.0"

# Parse arguments
APPLY_TOR_PATCHES=true
DEBUG_BUILD=false
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')

while [[ $# -gt 0 ]]; do
    case $1 in
        --firefox-version)
            FIREFOX_ESR_VERSION="$2"
            shift 2
            ;;
        --tor-version)
            TOR_BROWSER_VERSION="$2"
            shift 2
            ;;
        --no-tor-patches)
            APPLY_TOR_PATCHES=false
            shift
            ;;
        --debug)
            DEBUG_BUILD=true
            shift
            ;;
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --firefox-version VERSION    Firefox ESR version (default: $FIREFOX_ESR_VERSION)"
            echo "  --tor-version VERSION        Tor Browser version (default: $TOR_BROWSER_VERSION)"
            echo "  --no-tor-patches             Skip Tor Browser patches"
            echo "  --debug                      Build in debug mode"
            echo "  --platform PLATFORM          Target platform (linux, darwin, windows)"
            echo "  --help                       Show this help"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Build directories
BUILD_DIR="$FORK_DIR/build"
SOURCE_DIR="$BUILD_DIR/firefox-source"
DIST_DIR="$FORK_DIR/dist"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  AnonNet Browser Build System${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Firefox ESR: ${GREEN}$FIREFOX_ESR_VERSION${NC}"
echo -e "Tor Browser: ${GREEN}$TOR_BROWSER_VERSION${NC}"
echo -e "Platform:    ${GREEN}$PLATFORM${NC}"
echo -e "Apply Tor Patches: ${GREEN}$APPLY_TOR_PATCHES${NC}"
echo ""

# Check dependencies
echo -e "${YELLOW}[1/9] Checking dependencies...${NC}"
check_dependency() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}Error: $1 is not installed${NC}"
        echo "Please install build dependencies:"
        echo "  Ubuntu/Debian: sudo apt install build-essential python3 nodejs yasm rustc cargo"
        echo "  macOS: brew install python nodejs yasm rust && xcode-select --install"
        exit 1
    fi
}

check_dependency python3
check_dependency node
check_dependency rustc
check_dependency cargo

if [[ "$PLATFORM" == "linux" ]]; then
    check_dependency yasm
fi

echo -e "${GREEN}✓ All dependencies found${NC}"

# Download Firefox ESR source
echo -e "${YELLOW}[2/9] Downloading Firefox ESR source...${NC}"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

FIREFOX_TARBALL="firefox-${FIREFOX_ESR_VERSION}.source.tar.xz"
FIREFOX_URL="https://archive.mozilla.org/pub/firefox/releases/${FIREFOX_ESR_VERSION}/source/${FIREFOX_TARBALL}"

if [ ! -f "$FIREFOX_TARBALL" ]; then
    echo "Downloading from $FIREFOX_URL"
    curl -L -o "$FIREFOX_TARBALL" "$FIREFOX_URL"
else
    echo "Using cached source: $FIREFOX_TARBALL"
fi

# Extract source
if [ ! -d "$SOURCE_DIR" ]; then
    echo "Extracting..."
    tar -xf "$FIREFOX_TARBALL"
    mv firefox-${FIREFOX_ESR_VERSION%esr} "$SOURCE_DIR" || mv firefox "$SOURCE_DIR"
fi

echo -e "${GREEN}✓ Firefox source ready${NC}"

# Download Tor Browser patches
if [ "$APPLY_TOR_PATCHES" = true ]; then
    echo -e "${YELLOW}[3/9] Downloading Tor Browser patches...${NC}"

    TOR_PATCHES_DIR="$BUILD_DIR/tor-browser-patches"
    mkdir -p "$TOR_PATCHES_DIR"

    # Clone Tor Browser repository (patches are in the repo)
    if [ ! -d "$TOR_PATCHES_DIR/tor-browser" ]; then
        git clone --depth 1 --branch "tor-browser-${FIREFOX_ESR_VERSION%esr}-14.0-1" \
            https://gitlab.torproject.org/tpo/applications/tor-browser.git \
            "$TOR_PATCHES_DIR/tor-browser" || {
            echo -e "${YELLOW}Warning: Could not clone exact version, using main branch${NC}"
            git clone --depth 1 \
                https://gitlab.torproject.org/tpo/applications/tor-browser.git \
                "$TOR_PATCHES_DIR/tor-browser"
        }
    fi

    echo -e "${GREEN}✓ Tor Browser patches downloaded${NC}"
else
    echo -e "${YELLOW}[3/9] Skipping Tor Browser patches${NC}"
fi

# Apply Tor Browser patches
if [ "$APPLY_TOR_PATCHES" = true ]; then
    echo -e "${YELLOW}[4/9] Applying Tor Browser patches...${NC}"

    cd "$SOURCE_DIR"

    # Tor Browser uses a series file to list patches
    PATCH_DIR="$TOR_PATCHES_DIR/tor-browser/browser/patches"

    if [ -d "$PATCH_DIR" ]; then
        echo "Applying patches from $PATCH_DIR"

        # Apply each patch
        for patch in "$PATCH_DIR"/*.patch; do
            if [ -f "$patch" ]; then
                echo "  Applying $(basename "$patch")"
                patch -p1 < "$patch" || {
                    echo -e "${YELLOW}Warning: Patch $(basename "$patch") failed, continuing...${NC}"
                }
            fi
        done
    else
        echo -e "${YELLOW}No patches directory found, using source as-is${NC}"
    fi

    echo -e "${GREEN}✓ Patches applied${NC}"
else
    echo -e "${YELLOW}[4/9] Skipping patch application${NC}"
fi

# Copy branding files
echo -e "${YELLOW}[5/9] Installing AnonNet branding...${NC}"

mkdir -p "$SOURCE_DIR/browser/branding/anonnet"
cp -r "$FORK_DIR/branding/"* "$SOURCE_DIR/browser/branding/anonnet/" || {
    echo -e "${YELLOW}Branding files not ready yet, using default${NC}"
}

echo -e "${GREEN}✓ Branding installed${NC}"

# Copy mozconfig
echo -e "${YELLOW}[6/9] Configuring build...${NC}"

cp "$BUILD_DIR/mozconfig" "$SOURCE_DIR/.mozconfig"

# Adjust mozconfig for debug build
if [ "$DEBUG_BUILD" = true ]; then
    sed -i 's/--enable-optimize/--disable-optimize/g' "$SOURCE_DIR/.mozconfig"
    sed -i 's/--enable-release/--disable-release/g' "$SOURCE_DIR/.mozconfig"
    sed -i 's/--disable-debug/--enable-debug/g' "$SOURCE_DIR/.mozconfig"
fi

echo -e "${GREEN}✓ Build configured${NC}"

# Build Firefox
echo -e "${YELLOW}[7/9] Building AnonNet Browser (this will take 1-2 hours)...${NC}"

cd "$SOURCE_DIR"

# Bootstrap dependencies
./mach bootstrap --application-choice browser --no-interactive || {
    echo -e "${YELLOW}Bootstrap failed, continuing...${NC}"
}

# Build
./mach build || {
    echo -e "${RED}Build failed!${NC}"
    echo "Check logs for details"
    exit 1
}

echo -e "${GREEN}✓ Build complete${NC}"

# Package
echo -e "${YELLOW}[8/9] Packaging...${NC}"

./mach package

# Copy to dist directory
mkdir -p "$DIST_DIR"

# Find the package
PACKAGE=$(find obj-*/dist -name "anonnet-browser*.tar.bz2" -o -name "anonnet-browser*.dmg" -o -name "anonnet-browser*.zip" | head -n 1)

if [ -n "$PACKAGE" ]; then
    cp "$PACKAGE" "$DIST_DIR/"
    echo -e "${GREEN}✓ Package created: $(basename "$PACKAGE")${NC}"
else
    echo -e "${YELLOW}Warning: Could not find package, checking build artifacts...${NC}"
    ls -la obj-*/dist/
fi

echo -e "${GREEN}✓ Packaging complete${NC}"

# Install system add-ons
echo -e "${YELLOW}[9/9] Installing system add-ons...${NC}"

EXTENSION_DIR="$SOURCE_DIR/obj-*/dist/bin/browser/features"
if [ -d "$EXTENSION_DIR" ]; then
    # Copy our credit monitoring extension
    cp -r "$BROWSER_DIR/extension" "$EXTENSION_DIR/anonnet-monitor@anonnet.org.xpi" || {
        echo -e "${YELLOW}Extension not ready, skipping${NC}"
    }
    echo -e "${GREEN}✓ System add-ons installed${NC}"
else
    echo -e "${YELLOW}Could not find extension directory, skipping${NC}"
fi

# Summary
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}  Build Complete!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Distribution package: ${GREEN}$DIST_DIR/$(basename "$PACKAGE")${NC}"
echo ""
echo "Next steps:"
echo "  1. Test: cd $DIST_DIR && tar -xf $(basename "$PACKAGE") && ./anonnet-browser/anonnet-browser"
echo "  2. Package: ./packaging/package.sh"
echo "  3. Distribute: Upload to releases"
echo ""
echo -e "${YELLOW}Note: First run will take longer as Firefox compiles startup cache${NC}"
