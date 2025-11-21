// AnonNet Extension Background Script
// Enforces .anon-only browsing by blocking clearnet URLs

console.log('AnonNet extension background script loaded');

// Check if a hostname is a .anon address
function isAnonAddress(hostname) {
    if (!hostname) return false;

    // Remove port if present
    const host = hostname.split(':')[0].toLowerCase();

    // Check if it ends with .anon
    return host.endsWith('.anon');
}

// Check if URL should be allowed
function isAllowedUrl(url) {
    // Allow extension pages
    if (url.startsWith('moz-extension://') || url.startsWith('chrome-extension://')) {
        return true;
    }

    // Allow about: pages
    if (url.startsWith('about:')) {
        return true;
    }

    // Allow file: URLs
    if (url.startsWith('file:')) {
        return true;
    }

    // Allow data: URLs
    if (url.startsWith('data:')) {
        return true;
    }

    try {
        const urlObj = new URL(url);
        const hostname = urlObj.hostname;

        // Allow localhost for the extension to communicate with daemon API
        // (daemon auto-selects a free port, so we allow any port on localhost)
        if (hostname === 'localhost' || hostname === '127.0.0.1') {
            const port = parseInt(urlObj.port) || 80;
            // Allow high ports (where API typically runs) and common API ports
            if (port >= 8000 || port === 80) {
                return true;
            }
        }

        // Check if it's a .anon address
        return isAnonAddress(hostname);
    } catch (e) {
        // If URL parsing fails, block it
        console.error('Failed to parse URL:', url, e);
        return false;
    }
}

// Get blocked page HTML
function getBlockedPageHtml(blockedUrl) {
    return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Clearnet Access Blocked - AnonNet</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #fff;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            padding: 20px;
        }
        .container {
            max-width: 600px;
            background: rgba(255, 255, 255, 0.95);
            border-radius: 16px;
            padding: 40px;
            color: #333;
            box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
        }
        .icon {
            font-size: 64px;
            text-align: center;
            margin-bottom: 20px;
        }
        h1 {
            font-size: 32px;
            margin: 0 0 16px 0;
            text-align: center;
            color: #d32f2f;
        }
        p {
            font-size: 16px;
            line-height: 1.6;
            margin: 0 0 20px 0;
            color: #555;
        }
        .blocked-url {
            background: #f5f5f5;
            padding: 16px;
            border-radius: 8px;
            font-family: 'Courier New', monospace;
            word-break: break-all;
            margin: 20px 0;
            border-left: 4px solid #d32f2f;
        }
        .reason {
            background: #fff3cd;
            border-left: 4px solid #ffc107;
            padding: 16px;
            border-radius: 8px;
            margin: 20px 0;
        }
        .reason-title {
            font-weight: 600;
            margin-bottom: 8px;
            color: #856404;
        }
        .reason-text {
            color: #856404;
            margin: 0;
        }
        ul {
            margin: 16px 0;
            padding-left: 24px;
        }
        li {
            margin: 8px 0;
            color: #555;
        }
        .footer {
            margin-top: 24px;
            padding-top: 24px;
            border-top: 1px solid #ddd;
            text-align: center;
            font-size: 14px;
            color: #777;
        }
        .logo {
            text-align: center;
            margin-bottom: 20px;
        }
        .logo svg {
            width: 64px;
            height: 64px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="logo">
            <svg width="64" height="64" viewBox="0 0 32 32" fill="none">
                <circle cx="16" cy="16" r="14" stroke="#d32f2f" stroke-width="2" fill="none"/>
                <circle cx="16" cy="16" r="10" stroke="#d32f2f" stroke-width="2" fill="none"/>
                <circle cx="16" cy="16" r="6" stroke="#d32f2f" stroke-width="2" fill="none"/>
                <circle cx="16" cy="16" r="3" fill="#d32f2f"/>
                <line x1="8" y1="8" x2="24" y2="24" stroke="#d32f2f" stroke-width="3"/>
            </svg>
        </div>

        <h1>🚫 Clearnet Access Blocked</h1>

        <p>
            AnonNet has blocked this request to a clearnet (regular internet) address.
            For your safety and privacy, AnonNet <strong>only allows access to .anon services</strong>.
        </p>

        <div class="blocked-url">
            <strong>Blocked URL:</strong><br>
            ${escapeHtml(blockedUrl)}
        </div>

        <div class="reason">
            <div class="reason-title">Why is this blocked?</div>
            <p class="reason-text">
                AnonNet is designed exclusively for anonymous .anon services.
                Accessing clearnet sites would bypass the anonymous network and
                could expose your identity.
            </p>
        </div>

        <p>
            <strong>What you can do:</strong>
        </p>
        <ul>
            <li>Use a .anon service instead (URLs ending in .anon)</li>
            <li>Use a regular browser for clearnet sites</li>
            <li>Check if the service you want has a .anon mirror</li>
        </ul>

        <div class="footer">
            <strong>AnonNet Browser Extension</strong><br>
            Privacy-first anonymous networking
        </div>
    </div>
</body>
</html>
    `.trim();
}

// HTML escape function
function escapeHtml(unsafe) {
    return unsafe
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

// Listen for web requests
browser.webRequest.onBeforeRequest.addListener(
    function(details) {
        const url = details.url;

        // Skip if it's an allowed URL
        if (isAllowedUrl(url)) {
            return {};
        }

        console.log('Blocking clearnet request:', url);

        // Block the request and show blocked page
        const blockedPageHtml = getBlockedPageHtml(url);
        const dataUrl = 'data:text/html;charset=utf-8,' + encodeURIComponent(blockedPageHtml);

        return { redirectUrl: dataUrl };
    },
    { urls: ["<all_urls>"] },
    ["blocking"]
);

// Show notification when extension is installed
browser.runtime.onInstalled.addListener((details) => {
    if (details.reason === 'install') {
        console.log('AnonNet extension installed');

        // Set badge to show extension is active
        browser.browserAction.setBadgeText({ text: '🔒' });
        browser.browserAction.setBadgeBackgroundColor({ color: '#4caf50' });
    }
});

// Try to discover API port
const POSSIBLE_PORTS = [19150, 19151, 19152, 19153, 19154, 19155, 9150, 9151, 8150];
let discoveredPort = null;

async function discoverApiPort() {
    for (const port of POSSIBLE_PORTS) {
        try {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 500);
            const response = await fetch(`http://127.0.0.1:${port}/health`, {
                signal: controller.signal
            });
            clearTimeout(timeoutId);
            if (response.ok) {
                discoveredPort = port;
                return port;
            }
        } catch (e) {
            continue;
        }
    }
    return null;
}

// Update badge based on daemon connection
async function updateBadge() {
    try {
        // Try discovered port first, or discover it
        if (!discoveredPort) {
            discoveredPort = await discoverApiPort();
        }

        if (discoveredPort) {
            const response = await fetch(`http://127.0.0.1:${discoveredPort}/health`);
            if (response.ok) {
                browser.browserAction.setBadgeText({ text: '✓' });
                browser.browserAction.setBadgeBackgroundColor({ color: '#4caf50' });
                return;
            }
        }

        // Failed to connect
        browser.browserAction.setBadgeText({ text: '✗' });
        browser.browserAction.setBadgeBackgroundColor({ color: '#f44336' });
        discoveredPort = null; // Reset for retry
    } catch (error) {
        browser.browserAction.setBadgeText({ text: '✗' });
        browser.browserAction.setBadgeBackgroundColor({ color: '#f44336' });
        discoveredPort = null; // Reset for retry
    }
}

// Update badge every 10 seconds
setInterval(updateBadge, 10000);
updateBadge(); // Initial update

/******************************************************************************
 * CREDIT MONITORING & WARNINGS
 ******************************************************************************/

let lastCreditBalance = null;
let hasShownLowCreditWarning = false;
let hasShownCriticalCreditWarning = false;

// Credit warning thresholds (can be configured in about:config)
const LOW_CREDIT_THRESHOLD = 100;
const CRITICAL_CREDIT_THRESHOLD = 50;
const ZERO_CREDIT_THRESHOLD = 10;

async function checkCreditBalance() {
    try {
        if (!discoveredPort) {
            discoveredPort = await discoverApiPort();
        }

        if (!discoveredPort) return;

        const response = await fetch(`http://127.0.0.1:${discoveredPort}/api/credits/stats`);
        if (!response.ok) return;

        const data = await response.json();
        const balance = data.balance;

        // Update last known balance
        if (lastCreditBalance === null) {
            lastCreditBalance = balance;
            return; // Skip warnings on first check
        }

        // Critical: Nearly out of credits
        if (balance <= ZERO_CREDIT_THRESHOLD && !hasShownCriticalCreditWarning) {
            showCriticalCreditWarning(balance);
            hasShownCriticalCreditWarning = true;
        }
        // Warning: Low credits
        else if (balance <= CRITICAL_CREDIT_THRESHOLD && !hasShownCriticalCreditWarning) {
            showCriticalCreditWarning(balance);
            hasShownCriticalCreditWarning = true;
            hasShownLowCreditWarning = true; // Skip low warning
        }
        // Info: Credits running low
        else if (balance <= LOW_CREDIT_THRESHOLD && !hasShownLowCreditWarning) {
            showLowCreditWarning(balance);
            hasShownLowCreditWarning = true;
        }

        // Reset warnings if credits increase significantly
        if (balance > LOW_CREDIT_THRESHOLD) {
            hasShownLowCreditWarning = false;
            hasShownCriticalCreditWarning = false;
        }

        lastCreditBalance = balance;

    } catch (error) {
        console.error('Error checking credit balance:', error);
    }
}

function showLowCreditWarning(balance) {
    browser.notifications.create('low-credits', {
        type: 'basic',
        iconUrl: browser.runtime.getURL('icons/icon-48.png'),
        title: 'AnonNet: Low Credits',
        message: `You have ${balance} credits remaining. Consider running as a relay to earn more credits.`,
        priority: 1
    });
}

function showCriticalCreditWarning(balance) {
    browser.notifications.create('critical-credits', {
        type: 'basic',
        iconUrl: browser.runtime.getURL('icons/icon-48.png'),
        title: 'AnonNet: Critical - Nearly Out of Credits!',
        message: `WARNING: Only ${balance} credits left! Your connection may be interrupted. Run as a relay or mine credits immediately.`,
        priority: 2,
        requireInteraction: true // Keep notification visible until user dismisses
    });

    // Also update badge to show warning
    browser.browserAction.setBadgeText({ text: '⚠️' });
    browser.browserAction.setBadgeBackgroundColor({ color: '#ff9800' });
}

// Check credits every 30 seconds
setInterval(checkCreditBalance, 30000);
checkCreditBalance(); // Initial check

// Handle notification clicks (open extension popup)
browser.notifications.onClicked.addListener((notificationId) => {
    if (notificationId === 'low-credits' || notificationId === 'critical-credits') {
        browser.browserAction.openPopup();
    }
});
