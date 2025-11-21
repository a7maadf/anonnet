#!/bin/bash
set -e

# Prepare AnonNet Monitor extension as a system add-on
# System add-ons are bundled with the browser and cannot be removed by users

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FORK_DIR="$(dirname "$SCRIPT_DIR")"
BROWSER_DIR="$(dirname "$FORK_DIR")"
EXTENSION_DIR="$BROWSER_DIR/extension"
OUTPUT_DIR="$SCRIPT_DIR/build"

echo "Preparing AnonNet Monitor system add-on..."

# Clean and create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/anonnet-monitor"

# Copy extension files
echo "Copying extension files..."
cp -r "$EXTENSION_DIR"/* "$OUTPUT_DIR/anonnet-monitor/"

# Modify manifest for system add-on
echo "Updating manifest for system add-on..."
cd "$OUTPUT_DIR/anonnet-monitor"

# Update manifest.json to mark as system add-on
node -e "
const fs = require('fs');
const manifest = JSON.parse(fs.readFileSync('manifest.json', 'utf8'));

// Mark as system add-on (cannot be disabled)
manifest.browser_specific_settings = manifest.browser_specific_settings || {};
manifest.browser_specific_settings.gecko = manifest.browser_specific_settings.gecko || {};
manifest.browser_specific_settings.gecko.id = 'anonnet-monitor@anonnet.org';
manifest.browser_specific_settings.gecko.update_url = null;

// Increase version
manifest.version = '1.0.0';

// Add additional permissions if needed
manifest.permissions = manifest.permissions || [];
if (!manifest.permissions.includes('privacy')) {
    manifest.permissions.push('privacy');
}

fs.writeFileSync('manifest.json', JSON.stringify(manifest, null, 2));
console.log('Manifest updated successfully');
" || {
    echo "Warning: Could not update manifest with Node.js, using sed"
    # Fallback: use sed to add system addon marker
    sed -i 's/"id": "anonnet@anonnet.org"/"id": "anonnet-monitor@anonnet.org"/g' manifest.json
}

# Create XPI file (Firefox extension format)
echo "Creating XPI package..."
zip -r -FS "../anonnet-monitor@anonnet.org.xpi" ./* -x "*.git*"

echo "System add-on created: $OUTPUT_DIR/anonnet-monitor@anonnet.org.xpi"
echo ""
echo "To install as system add-on, copy this file to:"
echo "  Firefox: <firefox-dir>/browser/features/"
echo "  Or during build: Copy to mozilla-source/obj-*/dist/bin/browser/features/"
