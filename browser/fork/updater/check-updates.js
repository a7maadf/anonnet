// AnonNet Browser Update Checker
// Checks for browser updates and notifies user

const MANIFEST_URL = "https://raw.githubusercontent.com/a7maadf/anonnet/main/browser/fork/updater/update-manifest.json";
const CURRENT_VERSION = "1.0.0";

// Compare semantic versions
function compareVersions(v1, v2) {
    const parts1 = v1.split('.').map(Number);
    const parts2 = v2.split('.').map(Number);

    for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
        const num1 = parts1[i] || 0;
        const num2 = parts2[i] || 0;

        if (num1 > num2) return 1;
        if (num1 < num2) return -1;
    }

    return 0;
}

// Get platform and architecture
function getPlatform() {
    const platform = navigator.platform.toLowerCase();
    if (platform.includes('win')) return 'windows';
    if (platform.includes('mac')) return 'darwin';
    if (platform.includes('linux')) return 'linux';
    return 'unknown';
}

function getArchitecture() {
    // navigator.userAgent analysis
    if (navigator.userAgent.includes('x86_64') || navigator.userAgent.includes('Win64')) {
        return 'x86_64';
    }
    if (navigator.userAgent.includes('arm64') || navigator.userAgent.includes('aarch64')) {
        return 'arm64';
    }
    return 'x86_64'; // Default assumption
}

// Check for updates
async function checkForUpdates() {
    try {
        const response = await fetch(MANIFEST_URL, {
            cache: 'no-cache'
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }

        const manifest = await response.json();
        const platform = getPlatform();
        const arch = getArchitecture();

        // Find latest version for this platform
        const builds = manifest.builds.filter(b =>
            b.platform === platform && b.architecture === arch
        );

        if (builds.length === 0) {
            console.log('No builds available for this platform');
            return null;
        }

        // Sort by version (descending)
        builds.sort((a, b) => compareVersions(b.version, a.version));
        const latestBuild = builds[0];

        // Compare with current version
        const comparison = compareVersions(latestBuild.version, CURRENT_VERSION);

        if (comparison > 0) {
            return {
                available: true,
                version: latestBuild.version,
                url: latestBuild.url,
                hash: latestBuild.hash,
                size: latestBuild.size,
                releaseDate: latestBuild.releaseDate,
                releaseNotes: latestBuild.releaseNotes,
                critical: latestBuild.critical
            };
        }

        return {
            available: false,
            currentVersion: CURRENT_VERSION
        };

    } catch (error) {
        console.error('Failed to check for updates:', error);
        return {
            error: error.message
        };
    }
}

// Show update notification
function showUpdateNotification(updateInfo) {
    if (!updateInfo.available) return;

    const message = updateInfo.critical
        ? `CRITICAL SECURITY UPDATE AVAILABLE!\n\nVersion ${updateInfo.version} is now available.\nThis is a security update and should be installed immediately.`
        : `AnonNet Browser ${updateInfo.version} is available.\n\nYou are currently using version ${CURRENT_VERSION}.`;

    // Create notification
    const notification = {
        type: 'basic',
        iconUrl: '../icons/icon-48.png',
        title: updateInfo.critical ? '🔴 Critical Update Available' : '📦 Update Available',
        message: message,
        buttons: [
            { title: 'Download Update' },
            { title: 'Release Notes' }
        ],
        requireInteraction: updateInfo.critical
    };

    if (typeof browser !== 'undefined' && browser.notifications) {
        browser.notifications.create('anonnet-update', notification);

        // Handle button clicks
        browser.notifications.onButtonClicked.addListener((notifId, buttonIndex) => {
            if (notifId === 'anonnet-update') {
                if (buttonIndex === 0) {
                    // Download update
                    browser.tabs.create({ url: updateInfo.url });
                } else if (buttonIndex === 1) {
                    // Release notes
                    browser.tabs.create({ url: updateInfo.releaseNotes });
                }
            }
        });
    } else {
        // Fallback for non-extension context
        console.log('Update available:', updateInfo);
    }
}

// Automatic update check on startup
async function init() {
    console.log('AnonNet Browser Update Checker initialized');
    console.log('Current version:', CURRENT_VERSION);

    // Check on startup
    const updateInfo = await checkForUpdates();

    if (updateInfo && !updateInfo.error) {
        if (updateInfo.available) {
            console.log('Update available:', updateInfo.version);
            showUpdateNotification(updateInfo);
        } else {
            console.log('Browser is up to date');
        }
    } else if (updateInfo && updateInfo.error) {
        console.error('Update check failed:', updateInfo.error);
    }

    // Check every 24 hours
    setInterval(async () => {
        const info = await checkForUpdates();
        if (info && info.available) {
            showUpdateNotification(info);
        }
    }, 24 * 60 * 60 * 1000);
}

// Export for use in extension or standalone
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { checkForUpdates, showUpdateNotification, init };
}

// Auto-init if running in browser extension context
if (typeof browser !== 'undefined') {
    init();
}
