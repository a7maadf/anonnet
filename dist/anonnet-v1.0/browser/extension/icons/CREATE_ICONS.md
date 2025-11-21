# Creating Extension Icons

The extension requires PNG icons. You have two options:

## Option 1: Convert SVG to PNG (Recommended)

If you have ImageMagick installed:

```bash
cd browser/extension/icons
convert -background none -size 48x48 icon-48.svg icon-48.png
convert -background none -size 96x96 icon-48.svg icon-96.png
```

## Option 2: Use SVG Directly (Firefox only)

Firefox accepts SVG icons. Update `manifest.json`:

```json
"icons": {
  "48": "icons/icon-48.svg",
  "96": "icons/icon-48.svg"
},
```

## Option 3: Create PNG Manually

1. Open `icon-48.svg` in any graphics editor (Inkscape, GIMP, etc.)
2. Export as PNG at 48x48 and 96x96 resolutions
3. Save as `icon-48.png` and `icon-96.png` in this directory

## Temporary Workaround

For development, you can use any 48x48 and 96x96 PNG images as placeholders.
