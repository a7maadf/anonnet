#!/bin/bash
set -e

# AnonNet Browser Packaging Script
# Creates distribution packages for Linux, macOS, and Windows

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FORK_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$FORK_DIR/build"
DIST_DIR="$FORK_DIR/dist"

# Version info
VERSION="1.0.0"
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  AnonNet Browser Packaging${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Version: ${GREEN}$VERSION${NC}"
echo -e "Platform: ${GREEN}$PLATFORM${NC}"
echo -e "Architecture: ${GREEN}$ARCH${NC}"
echo ""

# Check if build exists
FIREFOX_BUILD=$(find "$BUILD_DIR/firefox-source/obj-"* -name "firefox" -o -name "anonnet-browser" 2>/dev/null | head -n 1)

if [ -z "$FIREFOX_BUILD" ]; then
    echo -e "${RED}Error: No Firefox build found${NC}"
    echo "Run ./build/build.sh first"
    exit 1
fi

BUILD_OBJ_DIR=$(dirname "$FIREFOX_BUILD")
echo -e "Found build: ${GREEN}$BUILD_OBJ_DIR${NC}"

# Create package directory
PACKAGE_NAME="anonnet-browser-$VERSION-$PLATFORM-$ARCH"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"

echo -e "${YELLOW}Creating package directory...${NC}"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

# Copy browser files
echo -e "${YELLOW}Copying browser files...${NC}"
cp -r "$BUILD_OBJ_DIR"/* "$PACKAGE_DIR/" || {
    # Try alternative location
    cp -r "$BUILD_DIR/firefox-source/obj-"*/dist/bin/* "$PACKAGE_DIR/"
}

# Install autoconfig files
echo -e "${YELLOW}Installing autoconfig...${NC}"
mkdir -p "$PACKAGE_DIR/defaults/pref"
cp "$FORK_DIR/autoconfig/autoconfig.js" "$PACKAGE_DIR/defaults/pref/"
cp "$FORK_DIR/autoconfig/anonnet.cfg" "$PACKAGE_DIR/"

# Install system add-on
echo -e "${YELLOW}Installing system add-on...${NC}"
if [ ! -f "$FORK_DIR/system-addons/build/anonnet-monitor@anonnet.org.xpi" ]; then
    echo "Building system add-on first..."
    cd "$FORK_DIR/system-addons"
    bash prepare-addon.sh
fi

mkdir -p "$PACKAGE_DIR/browser/features"
cp "$FORK_DIR/system-addons/build/anonnet-monitor@anonnet.org.xpi" \
   "$PACKAGE_DIR/browser/features/"

# Create launcher script
echo -e "${YELLOW}Creating launcher script...${NC}"

if [ "$PLATFORM" = "linux" ]; then
    cat > "$PACKAGE_DIR/anonnet-browser" << 'EOF'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export MOZ_APP_LAUNCHER="AnonNet Browser"
exec "$SCRIPT_DIR/firefox" "$@"
EOF
    chmod +x "$PACKAGE_DIR/anonnet-browser"

elif [ "$PLATFORM" = "darwin" ]; then
    # macOS app bundle structure
    echo "Creating macOS app bundle..."
    # TODO: Create proper .app structure
fi

# Create README
echo -e "${YELLOW}Creating README...${NC}"
cat > "$PACKAGE_DIR/README.txt" << EOF
AnonNet Browser v$VERSION

A privacy-focused browser built on Firefox ESR with Tor Browser hardening,
designed exclusively for anonymous browsing on the AnonNet network.

QUICK START:
1. Ensure AnonNet daemon is running:
   cargo run --release --bin anonnet-daemon proxy

2. Launch browser:
   Linux:   ./anonnet-browser
   macOS:   open AnonNet.app
   Windows: anonnet-browser.exe

FEATURES:
✓ Tor Browser privacy hardening
✓ Hardcoded AnonNet proxy (cannot be disabled)
✓ Built-in credit monitoring
✓ Clearnet access blocked
✓ Circuit visualization
✓ Automatic security updates

IMPORTANT:
- Only .anon domains are accessible
- All settings are locked for security
- Monitor your credit balance in the extension
- Do not modify browser files

For more information:
https://github.com/a7maadf/anonnet

LICENSE: MPL 2.0 (Firefox), MIT/Apache-2.0 (AnonNet)
EOF

# Create desktop entry (Linux)
if [ "$PLATFORM" = "linux" ]; then
    echo -e "${YELLOW}Creating desktop entry...${NC}"
    cat > "$PACKAGE_DIR/anonnet-browser.desktop" << EOF
[Desktop Entry]
Version=1.0
Name=AnonNet Browser
Comment=Privacy-focused browser for AnonNet
Exec=$PACKAGE_DIR/anonnet-browser %u
Icon=$PACKAGE_DIR/browser/chrome/icons/default/default128.png
Terminal=false
Type=Application
Categories=Network;WebBrowser;
MimeType=text/html;text/xml;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;
StartupNotify=true
EOF
fi

# Package based on platform
echo -e "${YELLOW}Creating distribution package...${NC}"

cd "$DIST_DIR"

if [ "$PLATFORM" = "linux" ]; then
    # Create tar.gz
    tar -czf "$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"
    echo -e "${GREEN}✓ Created: $PACKAGE_NAME.tar.gz${NC}"

    # Optionally create .deb (requires dpkg-deb)
    if command -v dpkg-deb &> /dev/null; then
        echo "Creating .deb package..."
        DEB_DIR="$DIST_DIR/deb"
        mkdir -p "$DEB_DIR/DEBIAN"
        mkdir -p "$DEB_DIR/opt/anonnet-browser"
        mkdir -p "$DEB_DIR/usr/share/applications"

        cp -r "$PACKAGE_DIR"/* "$DEB_DIR/opt/anonnet-browser/"
        cp "$PACKAGE_DIR/anonnet-browser.desktop" "$DEB_DIR/usr/share/applications/"

        cat > "$DEB_DIR/DEBIAN/control" << EOF
Package: anonnet-browser
Version: $VERSION
Section: web
Priority: optional
Architecture: $ARCH
Maintainer: AnonNet Project <contact@anonnet.org>
Description: Privacy-focused browser for AnonNet
 A Firefox ESR-based browser with Tor Browser hardening,
 designed exclusively for the AnonNet anonymous network.
EOF

        dpkg-deb --build "$DEB_DIR" "$DIST_DIR/anonnet-browser_${VERSION}_${ARCH}.deb"
        echo -e "${GREEN}✓ Created: anonnet-browser_${VERSION}_${ARCH}.deb${NC}"
        rm -rf "$DEB_DIR"
    fi

elif [ "$PLATFORM" = "darwin" ]; then
    # Create .dmg
    hdiutil create -volname "AnonNet Browser" -srcfolder "$PACKAGE_DIR" \
        -ov -format UDZO "$PACKAGE_NAME.dmg"
    echo -e "${GREEN}✓ Created: $PACKAGE_NAME.dmg${NC}"

elif [ "$PLATFORM" = "mingw"* ] || [ "$PLATFORM" = "msys"* ]; then
    # Create .zip for Windows
    zip -r "$PACKAGE_NAME.zip" "$PACKAGE_DIR"
    echo -e "${GREEN}✓ Created: $PACKAGE_NAME.zip${NC}"
fi

# Generate checksums
echo -e "${YELLOW}Generating checksums...${NC}"
for file in "$DIST_DIR"/*.{tar.gz,deb,dmg,zip} 2>/dev/null; do
    if [ -f "$file" ]; then
        sha256sum "$file" > "$file.sha256"
        echo -e "${GREEN}✓ Checksum: $(basename "$file").sha256${NC}"
    fi
done

# Summary
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}  Packaging Complete!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Packages created in: $DIST_DIR"
ls -lh "$DIST_DIR"/*.{tar.gz,deb,dmg,zip} 2>/dev/null || true
echo ""
echo "Next steps:"
echo "  1. Test the package"
echo "  2. Sign with GPG: gpg --detach-sign --armor <package>"
echo "  3. Upload to GitHub releases"
echo "  4. Update update manifest"
