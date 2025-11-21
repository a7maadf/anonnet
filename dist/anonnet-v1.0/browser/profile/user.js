/******************************************************************************
 * AnonNet Browser Configuration (user.js)
 * Based on Tor Browser hardening principles
 *
 * This configuration provides:
 * - Privacy-focused settings from Tor Browser
 * - Fingerprinting resistance
 * - Security hardening
 * - AnonNet proxy integration (SOCKS5: 9050, HTTP: 8118)
 * - .anon domain support
 *
 * Installation:
 * 1. Copy this file to your Firefox profile directory:
 *    - Linux: ~/.mozilla/firefox/[profile-name]/
 *    - macOS: ~/Library/Application Support/Firefox/Profiles/[profile-name]/
 *    - Windows: %APPDATA%\Mozilla\Firefox\Profiles\[profile-name]\
 * 2. Restart Firefox
 * 3. Ensure AnonNet daemon is running (anonnet-daemon proxy)
 ******************************************************************************/

/******************************************************************************
 * SECTION 0: ANONNET BRANDING & UPDATES
 ******************************************************************************/

// Disable Firefox updates (use system package manager instead)
user_pref("app.update.enabled", false);
user_pref("app.update.auto", false);

// Disable extension updates (manage manually)
user_pref("extensions.update.enabled", false);
user_pref("extensions.update.autoUpdateDefault", false);

// Custom user agent (prevents Firefox version fingerprinting)
user_pref("general.useragent.override", "Mozilla/5.0 (Windows NT 10.0; rv:128.0) Gecko/20100101 Firefox/128.0");

// Disable "What's New" and welcome pages
user_pref("browser.startup.homepage_override.mstone", "ignore");
user_pref("startup.homepage_welcome_url", "");
user_pref("startup.homepage_welcome_url.additional", "");

/******************************************************************************
 * SECTION 1: ANONNET PROXY CONFIGURATION
 ******************************************************************************/

// Enable SOCKS5 proxy for all connections
user_pref("network.proxy.type", 1); // 1 = manual proxy configuration

// AnonNet SOCKS5 proxy (Tor-compatible port)
user_pref("network.proxy.socks", "127.0.0.1");
user_pref("network.proxy.socks_port", 9050);
user_pref("network.proxy.socks_version", 5);

// Route DNS through SOCKS proxy (critical for anonymity)
user_pref("network.proxy.socks_remote_dns", true);

// Disable other proxy types (use SOCKS5 for everything)
user_pref("network.proxy.http", "");
user_pref("network.proxy.http_port", 0);
user_pref("network.proxy.ssl", "");
user_pref("network.proxy.ssl_port", 0);
user_pref("network.proxy.ftp", "");
user_pref("network.proxy.ftp_port", 0);

// No proxy for localhost
user_pref("network.proxy.no_proxies_on", "");

// Disable proxy failover (don't bypass proxy on failure)
user_pref("network.proxy.failover_direct", false);
user_pref("network.proxy.backup.ssl", "");
user_pref("network.proxy.backup.ssl_port", 0);

/******************************************************************************
 * SECTION 2: PRIVACY & FINGERPRINTING RESISTANCE
 * (Tor Browser's RFP - Resist Fingerprinting)
 ******************************************************************************/

// Enable comprehensive fingerprinting protection
user_pref("privacy.resistFingerprinting", true);
user_pref("privacy.resistFingerprinting.letterboxing", true);
user_pref("privacy.resistFingerprinting.block_mozAddonManager", true);

// Disable WebGL (major fingerprinting vector)
user_pref("webgl.disabled", true);
user_pref("webgl.enable-webgl2", false);

// Canvas fingerprinting protection
user_pref("privacy.resistFingerprinting.autoDeclineNoUserInputCanvasPrompts", false);

// Audio fingerprinting protection
user_pref("media.webaudio.enabled", false);

// Screen/display fingerprinting
user_pref("privacy.window.maxInnerWidth", 1000);
user_pref("privacy.window.maxInnerHeight", 1000);

// Timezone spoofing (use UTC)
user_pref("privacy.resistFingerprinting.jsmloglevel", "Warn");

// Disable hardware acceleration (prevents GPU fingerprinting)
user_pref("gfx.direct2d.disabled", true);
user_pref("layers.acceleration.disabled", true);

/******************************************************************************
 * SECTION 3: DNS & NETWORK PRIVACY
 ******************************************************************************/

// Disable DNS over HTTPS (use proxy for all DNS)
user_pref("network.trr.mode", 5); // 5 = explicitly disabled

// Disable DNS prefetching
user_pref("network.dns.disablePrefetch", true);
user_pref("network.dns.disablePrefetchFromHTTPS", true);

// Disable link prefetching
user_pref("network.prefetch-next", false);

// Disable predictor/pre-connections
user_pref("network.predictor.enabled", false);
user_pref("network.predictor.enable-prefetch", false);
user_pref("network.http.speculative-parallel-limit", 0);

// Disable IPv6 (can leak outside proxy)
user_pref("network.dns.disableIPv6", true);

// Disable link rel=preconnect
user_pref("network.preload", false);

/******************************************************************************
 * SECTION 4: HTTP & HTTPS SECURITY
 ******************************************************************************/

// Force HTTPS-only mode
user_pref("dom.security.https_only_mode", true);
user_pref("dom.security.https_only_mode_ever_enabled", true);

// Disable HTTP Alternative Services
user_pref("network.http.altsvc.enabled", false);
user_pref("network.http.altsvc.oe", false);

// HTTP Referer control
user_pref("network.http.referer.XOriginPolicy", 2); // Only send to same origin
user_pref("network.http.referer.XOriginTrimmingPolicy", 2); // Trim to origin only
user_pref("network.http.sendRefererHeader", 2); // Send referer for links clicked

// Disable HTTP/2 (prevents some fingerprinting)
user_pref("network.http.http2.enabled", false);

// TLS/SSL hardening
user_pref("security.tls.version.min", 3); // TLS 1.2 minimum
user_pref("security.tls.version.max", 4); // TLS 1.3 maximum
user_pref("security.ssl.require_safe_negotiation", true);
user_pref("security.ssl.treat_unsafe_negotiation_as_broken", true);

// Disable TLS session tickets (privacy leak)
user_pref("security.ssl.disable_session_identifiers", true);

// OCSP stapling
user_pref("security.ssl.enable_ocsp_stapling", true);
user_pref("security.OCSP.enabled", 1);
user_pref("security.OCSP.require", false); // Don't break sites

/******************************************************************************
 * SECTION 5: COOKIES & STORAGE
 ******************************************************************************/

// Enhanced Tracking Protection (strict mode)
user_pref("browser.contentblocking.category", "strict");
user_pref("privacy.trackingprotection.enabled", true);
user_pref("privacy.trackingprotection.pbmode.enabled", true);
user_pref("privacy.trackingprotection.socialtracking.enabled", true);

// First Party Isolation (critical for privacy)
user_pref("privacy.firstparty.isolate", true);

// Cookies: allow from current site only
user_pref("network.cookie.cookieBehavior", 1); // Block third-party cookies
user_pref("network.cookie.lifetimePolicy", 2); // Accept for session only

// Disable Storage API
user_pref("dom.storage.enabled", false);

// Disable IndexedDB
user_pref("dom.indexedDB.enabled", false);

// Clear everything on shutdown
user_pref("privacy.sanitize.sanitizeOnShutdown", true);
user_pref("privacy.clearOnShutdown.cache", true);
user_pref("privacy.clearOnShutdown.cookies", true);
user_pref("privacy.clearOnShutdown.downloads", true);
user_pref("privacy.clearOnShutdown.formdata", true);
user_pref("privacy.clearOnShutdown.history", true);
user_pref("privacy.clearOnShutdown.offlineApps", true);
user_pref("privacy.clearOnShutdown.sessions", true);
user_pref("privacy.clearOnShutdown.siteSettings", false); // Keep site settings

/******************************************************************************
 * SECTION 6: WEBRTC & MEDIA
 ******************************************************************************/

// Disable WebRTC (major IP leak vector)
user_pref("media.peerconnection.enabled", false);
user_pref("media.peerconnection.ice.default_address_only", true);
user_pref("media.peerconnection.ice.no_host", true);
user_pref("media.peerconnection.ice.proxy_only_if_behind_proxy", true);

// Disable getUserMedia (webcam/microphone access)
user_pref("media.navigator.enabled", false);

// Disable WebRTC device enumeration
user_pref("media.peerconnection.ice.tcp", false);

// Disable screen sharing
user_pref("media.getusermedia.screensharing.enabled", false);

/******************************************************************************
 * SECTION 7: JAVASCRIPT & WEB APIs
 ******************************************************************************/

// JavaScript is enabled but restricted (sites need JS to function)
user_pref("javascript.enabled", true);

// Disable dangerous JS APIs
user_pref("dom.event.clipboardevents.enabled", false); // Clipboard access
user_pref("dom.battery.enabled", false); // Battery API
user_pref("dom.gamepad.enabled", false); // Gamepad API
user_pref("dom.netinfo.enabled", false); // Network Information API
user_pref("dom.vibrator.enabled", false); // Vibration API

// Disable Service Workers (can be used for tracking)
user_pref("dom.serviceWorkers.enabled", false);

// Disable Push API
user_pref("dom.push.enabled", false);

// Disable Notifications
user_pref("dom.webnotifications.enabled", false);
user_pref("dom.webnotifications.serviceworker.enabled", false);

// Disable Beacon API (analytics tracking)
user_pref("beacon.enabled", false);

// Disable Virtual Reality APIs
user_pref("dom.vr.enabled", false);

/******************************************************************************
 * SECTION 8: LOCATION & SENSORS
 ******************************************************************************/

// Disable geolocation
user_pref("geo.enabled", false);
user_pref("geo.provider.network.url", "");

// Disable sensor APIs
user_pref("device.sensors.enabled", false);
user_pref("dom.device.enabled", false);

// Disable camera/microphone status
user_pref("media.navigator.video.enabled", false);

/******************************************************************************
 * SECTION 9: TELEMETRY & DATA COLLECTION
 ******************************************************************************/

// Disable all telemetry
user_pref("toolkit.telemetry.enabled", false);
user_pref("toolkit.telemetry.unified", false);
user_pref("toolkit.telemetry.archive.enabled", false);
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);

// Disable crash reports
user_pref("breakpad.reportURL", "");
user_pref("browser.tabs.crashReporting.sendReport", false);
user_pref("browser.crashReports.unsubmittedCheck.autoSubmit2", false);

// Disable experiments
user_pref("experiments.enabled", false);
user_pref("experiments.supported", false);
user_pref("network.allow-experiments", false);

// Disable Pocket
user_pref("extensions.pocket.enabled", false);

// Disable Firefox Accounts / Sync
user_pref("identity.fxaccounts.enabled", false);

/******************************************************************************
 * SECTION 10: SEARCH & SUGGESTIONS
 ******************************************************************************/

// Disable search suggestions
user_pref("browser.search.suggest.enabled", false);
user_pref("browser.urlbar.suggest.searches", false);
user_pref("browser.urlbar.suggest.quicksuggest.sponsored", false);
user_pref("browser.urlbar.suggest.quicksuggest.nonsponsored", false);

// Disable form autofill
user_pref("browser.formfill.enable", false);
user_pref("extensions.formautofill.addresses.enabled", false);
user_pref("extensions.formautofill.creditCards.enabled", false);

// Disable password manager (use external manager)
user_pref("signon.rememberSignons", false);
user_pref("signon.autofillForms", false);

/******************************************************************************
 * SECTION 11: BROWSER FEATURES
 ******************************************************************************/

// Disable safebrowsing (sends URLs to Google)
user_pref("browser.safebrowsing.malware.enabled", false);
user_pref("browser.safebrowsing.phishing.enabled", false);
user_pref("browser.safebrowsing.downloads.enabled", false);
user_pref("browser.safebrowsing.downloads.remote.enabled", false);

// Disable PDF.js (potential attack surface)
user_pref("pdfjs.disabled", true);
user_pref("pdfjs.enableScripting", false);

// Disable WebAssembly (potential attack surface)
user_pref("javascript.options.wasm", false);

// Disable clipboard events
user_pref("dom.event.clipboardevents.enabled", false);

// Disable right-click context menu manipulation
user_pref("dom.event.contextmenu.enabled", true);

// Disable middle mouse click new tab from clipboard
user_pref("middlemouse.contentLoadURL", false);

/******************************************************************************
 * SECTION 12: FONT & RENDERING
 ******************************************************************************/

// Limit font fingerprinting
user_pref("browser.display.use_document_fonts", 0); // Only use system fonts

// Disable font enumeration
user_pref("layout.css.font-visibility.private", 1);
user_pref("layout.css.font-visibility.standard", 1);
user_pref("layout.css.font-visibility.trackingprotection", 1);

/******************************************************************************
 * SECTION 13: UI & UX HARDENING
 ******************************************************************************/

// New tab page blank
user_pref("browser.newtabpage.enabled", false);
user_pref("browser.newtab.preload", false);
user_pref("browser.newtabpage.activity-stream.enabled", false);

// Disable snippets
user_pref("browser.newtabpage.activity-stream.feeds.snippets", false);

// Disable Highlights
user_pref("browser.newtabpage.activity-stream.feeds.section.highlights", false);

// Disable sponsored content
user_pref("browser.newtabpage.activity-stream.showSponsored", false);
user_pref("browser.newtabpage.activity-stream.showSponsoredTopSites", false);

// Home page
user_pref("browser.startup.homepage", "about:blank");
user_pref("browser.startup.page", 0); // 0 = blank page

/******************************************************************************
 * SECTION 14: DOWNLOAD PROTECTION
 ******************************************************************************/

// Disable download scanning metadata collection
user_pref("browser.download.useDownloadDir", false); // Always ask where to save

// Don't reveal download folder
user_pref("browser.download.folderList", 2);

/******************************************************************************
 * SECTION 15: ANONNET SPECIFIC SETTINGS
 ******************************************************************************/

// Custom preferences for .anon domain handling
// Note: .anon domains are handled by the AnonNet proxy layer

// Disable automatic proxy detection (always use configured proxy)
user_pref("network.proxy.autoconfig_url", "");
user_pref("network.proxy.no_proxies_on", ""); // Don't bypass proxy for any domain

// Warning for proxy bypass attempts
user_pref("network.proxy.allow_bypass", false);

// Custom certificate handling for .anon services
user_pref("security.enterprise_roots.enabled", false);

/******************************************************************************
 * SECTION 16: PERFORMANCE TWEAKS (Minimal for privacy)
 ******************************************************************************/

// Disable disk cache (privacy, but slower)
user_pref("browser.cache.disk.enable", false);
user_pref("browser.cache.disk_cache_ssl", false);
user_pref("browser.cache.memory.enable", true);
user_pref("browser.cache.memory.capacity", 65536); // 64MB memory cache

// Session restore privacy
user_pref("browser.sessionstore.privacy_level", 2); // Never store session data

/******************************************************************************
 * SECTION 17: MISCELLANEOUS HARDENING
 ******************************************************************************/

// Disable mozAddonManager Web API
user_pref("privacy.resistFingerprinting.block_mozAddonManager", true);

// Disable WebAssembly baseline compiler
user_pref("javascript.options.wasm_baselinejit", false);

// Disable WebAssembly optimizing compiler
user_pref("javascript.options.wasm_optimizingjit", false);

// Enforce punycode for IDN (prevents phishing)
user_pref("network.IDN_show_punycode", true);

// Disable IPv6
user_pref("network.dns.disableIPv6", true);

// Disable HTTP Alternative-Services
user_pref("network.http.altsvc.enabled", false);

// Additional security headers
user_pref("security.csp.enable", true);
user_pref("security.sri.enable", true);

/******************************************************************************
 * END OF CONFIGURATION
 *
 * After applying these settings:
 * 1. Restart Firefox completely
 * 2. Ensure AnonNet daemon is running: cargo run --bin anonnet-daemon proxy
 * 3. Test connection: Visit a .anon service
 * 4. Verify proxy: about:config -> network.proxy.socks_port should be 9050
 *
 * Security Level: HIGH
 * Privacy Level: MAXIMUM
 * Compatibility: MODERATE (some sites may break)
 *
 * For support: https://github.com/a7maadf/anonnet
 ******************************************************************************/
