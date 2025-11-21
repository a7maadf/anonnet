# AnonNet Browser Extension

A WebExtension that integrates with the AnonNet daemon to display credit balance, network statistics, and enforce .anon-only browsing.

## Features

### 🪙 Credit System Integration
- **Real-time credit balance** displayed in popup
- **Earning/spending rates** (credits per hour)
- **Total earned/spent** statistics
- Auto-refreshes every 5 seconds

### 🌐 Network Monitoring
- **Peer count** (total and active)
- **Circuit statistics** (total and active)
- **Bandwidth usage** in real-time
- **Connection status** indicator
- **Node ID** display

### 🔒 .anon-Only Enforcement
- **Blocks all clearnet URLs** at browser level
- Shows informative blocked page
- Additional security layer (daemon also blocks)
- Allows only .anon services

### 🎨 Beautiful UI
- Modern gradient design
- Real-time status updates
- Smooth animations
- Responsive layout

## Installation

### Prerequisites

1. **AnonNet daemon running:**
   ```bash
   cargo run --release --bin anonnet-daemon proxy
   ```

2. **Firefox 115+** or **Firefox ESR 115+**

### Install Extension

#### Method 1: Temporary Installation (Development)

1. **Create icon files:**
   ```bash
   cd browser/extension/icons
   # See CREATE_ICONS.md for instructions
   ```

2. **Open Firefox:**
   - Type `about:debugging` in the address bar
   - Click "This Firefox"
   - Click "Load Temporary Add-on"
   - Navigate to `browser/extension/` folder
   - Select `manifest.json`

3. **Verify installation:**
   - Extension icon should appear in toolbar
   - Click icon to see popup with credit balance

#### Method 2: Permanent Installation (Packaged)

1. **Create icons** (see above)

2. **Package extension:**
   ```bash
   cd browser/extension
   zip -r ../anonnet-extension.xpi *
   ```

3. **Install in Firefox:**
   - Type `about:addons` in address bar
   - Click gear icon → "Install Add-on From File"
   - Select `anonnet-extension.xpi`

#### Method 3: Automatic (via launcher script)

```bash
# The launcher script will auto-install the extension
./browser/scripts/launch-anonnet-browser.sh
```

## Usage

### View Stats

1. Click the AnonNet icon in toolbar
2. Popup shows:
   - Current credit balance
   - Earned/spent credits
   - Earning/spending rates
   - Network stats (peers, circuits, bandwidth)
   - Node ID

### Refresh Data

- Auto-refreshes every 5 seconds
- Manual refresh: Click "Refresh" button

### Browse .anon Sites

1. Start AnonNet daemon
2. Enter .anon address in browser
3. Extension allows connection
4. Daemon routes through anonymous circuits

### Clearnet Blocking

- Any attempt to visit clearnet sites shows blocked page
- Explains why it's blocked
- Suggests alternatives

## API Endpoints Used

The extension communicates with the daemon's REST API:

- `GET http://127.0.0.1:<API_PORT>/api/credits/balance` - Credit balance
- `GET http://127.0.0.1:<API_PORT>/api/credits/stats` - Earning/spending stats
- `GET http://127.0.0.1:<API_PORT>/api/network/status` - Network status
- `GET http://127.0.0.1:<API_PORT>/api/circuits/active` - Active circuits
- `GET http://127.0.0.1:<API_PORT>/health` - Health check

**Note:** The daemon auto-selects free ports to avoid conflicts. The extension automatically discovers the API port by probing common ports (19150-19155, 9150-9151, 8150). Port numbers are also saved to `./data/api_port.txt`.

## File Structure

```
extension/
├── manifest.json           # Extension metadata and permissions
├── icons/
│   ├── icon-48.svg        # Extension icon (SVG source)
│   ├── icon-48.png        # 48x48 icon
│   ├── icon-96.png        # 96x96 icon
│   └── CREATE_ICONS.md    # Icon creation guide
├── popup/
│   ├── popup.html         # Popup UI structure
│   ├── popup.css          # Popup styling
│   └── popup.js           # Popup logic and API calls
├── background/
│   └── background.js      # Background script (.anon enforcement)
└── README.md              # This file
```

## Permissions

The extension requires these permissions:

- **`storage`** - Store user preferences
- **`webRequest`** - Intercept web requests
- **`webRequestBlocking`** - Block clearnet requests
- **`http://127.0.0.1:9051/*`** - Access daemon API
- **`<all_urls>`** - Monitor all requests (for .anon filtering)

## Troubleshooting

### Extension shows "Error connecting to daemon"

**Problem:** Can't reach API at http://127.0.0.1:9051

**Solutions:**
1. Ensure daemon is running:
   ```bash
   cargo run --release --bin anonnet-daemon proxy
   ```

2. Check API is listening:
   ```bash
   curl http://127.0.0.1:9051/health
   # Should return "OK"
   ```

3. Check browser console:
   - Right-click extension icon → Inspect
   - Look for errors in console

### Credits show "---"

**Problem:** API returning errors

**Solutions:**
1. Check daemon logs for errors
2. Verify node has initialized:
   ```bash
   curl http://127.0.0.1:9051/api/network/status
   ```

3. Ensure credit ledger is initialized

### Clearnet sites not blocked

**Problem:** Extension not intercepting requests

**Solutions:**
1. Check extension is enabled in `about:addons`
2. Reload extension
3. Check background script console for errors:
   - `about:debugging` → Extension → Inspect

### Icons not showing

**Problem:** PNG icons not created

**Solution:**
- See `icons/CREATE_ICONS.md` for instructions
- Or use SVG icons (Firefox only)

## Development

### Testing Locally

1. **Start daemon:**
   ```bash
   cargo run --release --bin anonnet-daemon proxy
   ```

2. **Load extension in Firefox:**
   - `about:debugging` → Load Temporary Add-on

3. **View console:**
   - Background script: `about:debugging` → Inspect
   - Popup: Right-click icon → Inspect

### Making Changes

1. Edit files in `extension/` folder
2. Click "Reload" in `about:debugging`
3. Test changes

### API Testing

```bash
# Test credit balance
curl http://127.0.0.1:9051/api/credits/balance

# Test credit stats
curl http://127.0.0.1:9051/api/credits/stats

# Test network status
curl http://127.0.0.1:9051/api/network/status

# Test health
curl http://127.0.0.1:9051/health
```

## Security Considerations

### What the Extension Does

✅ Enforces .anon-only browsing
✅ Blocks clearnet at browser level
✅ Displays credit/network stats
✅ Communicates only with local daemon

### What the Extension Does NOT Do

❌ Send data to external servers
❌ Track your browsing
❌ Collect personal information
❌ Modify .anon traffic

### Privacy

- All communication is with local daemon (127.0.0.1)
- No external API calls
- No analytics or telemetry
- Open source and auditable

## Credits

- Integrates with AnonNet daemon
- Based on Tor Browser hardening principles
- Uses modern WebExtension APIs

## License

Same as AnonNet: Dual MIT/Apache-2.0

## Support

- Issues: https://github.com/a7maadf/anonnet/issues
- Docs: See main README.md
