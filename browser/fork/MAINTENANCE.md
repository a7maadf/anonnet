# AnonNet Browser Maintenance Guide

This guide covers ongoing maintenance tasks for the AnonNet Browser fork, including updates, security patches, and ESR migrations.

## Table of Contents

1. [Maintenance Schedule](#maintenance-schedule)
2. [Monthly Security Updates](#monthly-security-updates)
3. [Annual ESR Migrations](#annual-esr-migrations)
4. [Tor Browser Patch Updates](#tor-browser-patch-updates)
5. [Emergency Security Patches](#emergency-security-patches)
6. [Testing Procedures](#testing-procedures)
7. [Release Process](#release-process)
8. [Troubleshooting](#troubleshooting)

---

## Maintenance Schedule

### Monthly Tasks (1st of each month)
- [ ] Check for Firefox ESR security updates
- [ ] Check for Tor Browser patch updates
- [ ] Review dependency vulnerabilities
- [ ] Test with latest anonnet daemon
- [ ] Update documentation if needed

### Quarterly Tasks
- [ ] Full security audit
- [ ] Performance profiling
- [ ] User feedback review
- [ ] Update third-party dependencies

### Annual Tasks (When new ESR is released)
- [ ] Plan ESR migration (typically January/February)
- [ ] Test new ESR extensively
- [ ] Re-apply all customizations
- [ ] Update build system
- [ ] Major version release

---

## Monthly Security Updates

Firefox ESR releases security updates every 4-6 weeks. Here's how to apply them:

### 1. Check for Updates

```bash
# Subscribe to Mozilla security advisories
# https://www.mozilla.org/en-US/security/advisories/

# Check current version
cat browser/fork/build/mozconfig | grep FIREFOX_ESR_VERSION

# Check latest ESR version
curl -s https://product-details.mozilla.org/1.0/firefox_versions.json | jq
```

### 2. Update Build Configuration

```bash
cd browser/fork/build

# Edit mozconfig or build.sh to update version
vim build.sh

# Change: FIREFOX_ESR_VERSION="128.6.0esr"
# To:     FIREFOX_ESR_VERSION="128.7.0esr"  # New version
```

### 3. Rebuild Browser

```bash
# Clean previous build
./build/clean.sh  # (create this if needed)

# Build with new version
./build/build.sh --firefox-version 128.7.0esr

# This will:
# 1. Download new Firefox ESR source
# 2. Apply Tor Browser patches
# 3. Apply AnonNet customizations
# 4. Build browser
# 5. Package for distribution
```

### 4. Test Thoroughly

```bash
# Run automated tests
./build/test.sh

# Manual testing checklist:
# - Browser starts successfully
# - Proxy is hardcoded and cannot be changed
# - Extension loads and shows credit balance
# - Clearnet sites are blocked
# - .anon sites load correctly
# - No new fingerprinting vectors
# - Check about:config for locked prefs
```

### 5. Package and Release

```bash
# Create packages for all platforms
./packaging/package.sh

# Sign packages
gpg --detach-sign --armor dist/anonnet-browser-1.0.1-linux-x86_64.tar.gz

# Update version in update manifest
vim updater/update-manifest.json

# Create GitHub release
gh release create browser-v1.0.1 \
  dist/anonnet-browser-* \
  --title "AnonNet Browser 1.0.1" \
  --notes "Security update for Firefox ESR 128.7.0"
```

### 6. Announce Update

```bash
# Update README.md version badges
# Post announcement on GitHub Discussions
# Notify users via Twitter/Mastodon
# Update website (if any)
```

---

## Annual ESR Migrations

Firefox releases a new ESR version annually (e.g., ESR 128 → ESR 140). This is a major update requiring extensive testing.

### Timeline

**Week 1-2: Preparation**
- Download new ESR beta
- Review Mozilla's ESR migration guide
- Identify breaking changes in build system
- Check Tor Browser's migration status

**Week 3-4: Patching**
- Apply Tor Browser patches to new ESR
- Resolve patch conflicts (there will be many)
- Document which patches failed and why
- Create workarounds for failed patches

**Week 5-6: Testing**
- Extensive security testing
- Fingerprinting analysis
- Performance benchmarking
- User acceptance testing

**Week 7-8: Release**
- Package for all platforms
- Create comprehensive release notes
- Phased rollout (beta → stable)
- Monitor for issues

### Detailed Steps

#### 1. Download New ESR

```bash
cd browser/fork/build

# Update build script
vim build.sh

# Change: FIREFOX_ESR_VERSION="128.6.0esr"
# To:     FIREFOX_ESR_VERSION="140.0.0esr"

# Download source
./build.sh --firefox-version 140.0.0esr --no-tor-patches
```

#### 2. Update Tor Browser Patches

```bash
cd build/tor-browser-patches

# Clone latest Tor Browser for new ESR
git clone --depth 1 --branch "tor-browser-140.0-14.5-1" \
  https://gitlab.torproject.org/tpo/applications/tor-browser.git \
  tor-browser-140

# Review changes
diff -r tor-browser-128/ tor-browser-140/
```

#### 3. Apply Patches (Expect Conflicts)

```bash
cd ../firefox-source

# Try applying patches
for patch in ../tor-browser-patches/tor-browser-140/browser/patches/*.patch; do
    echo "Applying $(basename $patch)..."
    patch -p1 < "$patch" || {
        echo "FAILED: $patch"
        echo "$patch" >> ../failed-patches.txt
    }
done

# Review failures
cat ../failed-patches.txt
```

#### 4. Resolve Conflicts

For each failed patch:

1. **Understand the patch purpose**
   ```bash
   head -20 path/to/failed.patch
   ```

2. **Find the affected file**
   ```bash
   grep "diff --git" path/to/failed.patch
   ```

3. **Check if code moved**
   ```bash
   rg "function_name_from_patch" .
   ```

4. **Manual application**
   - If code moved: Apply to new location
   - If code removed: Determine if still needed
   - If conflicting: Merge carefully

5. **Document changes**
   ```bash
   echo "Patch X: Applied manually to file.js:123 (was file.js:100 in ESR 128)" \
     >> ../manual-patches.txt
   ```

#### 5. AnonNet-Specific Updates

```bash
# Update branding version
vim browser/branding/anonnet/configure.sh
# Change: MOZ_APP_VERSION=1.0.0
# To:     MOZ_APP_VERSION=2.0.0  # Major version bump for ESR change

# Update autoconfig if needed
vim autoconfig/anonnet.cfg
# Check if new prefs need locking

# Update extension
vim ../extension/manifest.json
# Update minimum Firefox version
```

#### 6. Build and Test

```bash
# Clean build
rm -rf obj-*

# Full build
./mach build

# Run all tests
./build/test.sh --full

# Manual testing (critical)
# Test EVERYTHING - ESR migrations often break things
```

#### 7. Security Audit

```bash
# Fingerprinting test
# Visit: https://coveryourtracks.eff.org/
# Should show "Strong protection"

# DNS leak test
# Visit: https://browserleaks.com/dns
# Should show no leaks

# WebRTC test
# Visit: https://browserleaks.com/webrtc
# Should be blocked

# Proxy test
# about:config → network.proxy.socks_port
# Should be locked at 9050
```

#### 8. Performance Benchmarking

```bash
# Compare startup time
time ./anonnet-browser --version

# Memory usage
ps aux | grep anonnet-browser

# Page load time (on .anon sites)
# Use browser dev tools
```

#### 9. Release

```bash
# Update version everywhere
find . -name "*.json" -o -name "*.sh" -o -name "*.cfg" | \
  xargs grep -l "1.0.0" | \
  xargs sed -i 's/1.0.0/2.0.0/g'

# Package
./packaging/package.sh

# Create detailed release notes
cat > RELEASE-NOTES-2.0.0.md << EOF
# AnonNet Browser 2.0.0 - ESR 140 Migration

## Major Changes
- Migrated to Firefox ESR 140 (from ESR 128)
- Updated all Tor Browser security patches
- [List other changes]

## Security Improvements
- [List security improvements]

## Breaking Changes
- Requires AnonNet daemon v0.2.0+
- [Other breaking changes]

## Known Issues
- [Any known issues]

## Upgrade Instructions
[Step-by-step upgrade guide]
EOF

# Release
gh release create browser-v2.0.0 \
  dist/* \
  --title "AnonNet Browser 2.0.0 - ESR 140" \
  --notes-file RELEASE-NOTES-2.0.0.md
```

---

## Tor Browser Patch Updates

Tor Browser updates their patches every 2-4 weeks. Not all updates need immediate action.

### When to Update Tor Patches

**Update immediately if:**
- Security vulnerability fix
- Privacy regression fix
- Fingerprinting mitigation improvement

**Can wait until next release if:**
- Feature additions
- UI/UX changes
- Documentation updates

### How to Update

```bash
cd browser/fork/build/tor-browser-patches

# Check for updates
git fetch origin
git log HEAD..origin/main --oneline

# Review changes
git diff HEAD..origin/main browser/patches/

# If relevant, pull and rebuild
git pull
cd ../../
./build.sh
```

---

## Emergency Security Patches

When a critical zero-day is announced:

### 1. Assess Severity

```bash
# Check Mozilla Security Advisories
# https://www.mozilla.org/en-US/security/advisories/

# Questions:
# - Does it affect Firefox ESR?
# - Is it actively exploited?
# - Does it bypass our hardening?
# - What's the CVSS score?
```

### 2. Fast-Track Build

```bash
# Goal: Release within 24 hours

# Skip Tor Browser patches if not updated yet
./build.sh --firefox-version 128.X.Xesr --no-tor-patches

# Quick test (not full suite)
./build/test.sh --quick

# Emergency release
./packaging/package.sh
```

### 3. Emergency Release Process

```bash
# Mark as critical in manifest
cat > updater/update-manifest.json << EOF
{
  "version": "1.0.2",
  "critical": true,  # <-- Emergency flag
  "securityAdvisory": "https://mozilla.org/...",
  ...
}
EOF

# Release
gh release create browser-v1.0.2-EMERGENCY \
  --prerelease \
  --title "🔴 CRITICAL SECURITY UPDATE" \
  --notes "Emergency release for CVE-XXXX-XXXX"

# Notify all users
# - Update manifest (triggers auto-check)
# - Post GitHub announcement
# - Social media alert
# - Email notification (if list exists)
```

---

## Testing Procedures

### Automated Tests

```bash
#!/bin/bash
# browser/fork/build/test.sh

# Proxy configuration test
test_proxy() {
    # Launch browser in headless mode
    ./dist/anonnet-browser --headless &
    PID=$!
    sleep 5

    # Check that proxy is set correctly
    # (would need to access about:config via automation)

    kill $PID
}

# Extension test
test_extension() {
    # Check extension is loaded
    # Check it cannot be disabled
}

# Clearnet blocking test
test_clearnet_blocking() {
    # Try to access example.com
    # Should be blocked
}

# .anon access test
test_anon_access() {
    # Access a .anon service
    # Should work
}

# Run all tests
test_proxy
test_extension
test_clearnet_blocking
test_anon_access
```

### Manual Test Checklist

Before each release:

**Basic Functionality**
- [ ] Browser launches without errors
- [ ] Extension loads automatically
- [ ] Credit balance displays correctly
- [ ] Network status updates

**Security**
- [ ] Proxy is locked (cannot change in settings)
- [ ] Clearnet sites are blocked
- [ ] .anon sites work
- [ ] WebRTC is disabled
- [ ] DNS goes through SOCKS

**Privacy**
- [ ] about:config shows all locked prefs
- [ ] Fingerprinting resistance works
- [ ] Canvas is randomized
- [ ] User agent is spoofed

**Performance**
- [ ] Startup time < 5 seconds
- [ ] Memory usage reasonable
- [ ] Pages load smoothly

---

## Release Process

### Version Numbering

```
MAJOR.MINOR.PATCH

Examples:
1.0.0 - Initial release (ESR 128)
1.0.1 - Security patch
1.1.0 - New feature (credit visualization)
2.0.0 - ESR migration (ESR 140)
```

### Release Checklist

**Pre-Release**
- [ ] All tests passing
- [ ] Security audit completed
- [ ] Documentation updated
- [ ] Changelog written
- [ ] Version bumped everywhere

**Build**
- [ ] Build for Linux (x86_64, arm64)
- [ ] Build for macOS (x86_64, arm64)
- [ ] Build for Windows (x86_64)
- [ ] Verify all packages

**Sign**
- [ ] Generate checksums (SHA256)
- [ ] GPG sign all packages
- [ ] Update signature verification docs

**Publish**
- [ ] Create GitHub release
- [ ] Upload all packages
- [ ] Update update-manifest.json
- [ ] Update website

**Announce**
- [ ] GitHub Discussions
- [ ] Project README
- [ ] Social media
- [ ] User mailing list (if exists)

---

## Troubleshooting

### Build Fails

**"Patch does not apply"**
```bash
# Conflict in patch, need manual resolution
git apply --reject path/to/patch.patch
# Fix .rej files manually
```

**"Missing dependency"**
```bash
# Install build dependencies
sudo apt install [missing-package]
```

**"Out of memory"**
```bash
# Reduce parallel jobs
export MOZ_PARALLEL_BUILD=2
./mach build
```

### Runtime Issues

**"Proxy not working"**
```bash
# Check daemon is running
ps aux | grep anonnet-daemon

# Check port file exists
cat ~/.anonnet/data/socks5_port.txt

# Check autoconfig loaded
# about:support → check "Enterprise Policies"
```

**"Extension not loading"**
```bash
# Check system add-on directory
ls dist/anonnet-browser/browser/features/

# Check extension is valid
cd browser/extension
zip -T ../anonnet-monitor.xpi
```

**"High memory usage"**
```bash
# Disable some features
# Edit autoconfig/anonnet.cfg
# Comment out resource-intensive settings
```

---

## Maintenance Tools

### Useful Scripts

**check-versions.sh**
```bash
#!/bin/bash
# Check versions of all components

echo "Firefox ESR: $(grep FIREFOX_ESR_VERSION build/build.sh)"
echo "Tor Browser: $(grep TOR_BROWSER_VERSION build/build.sh)"
echo "AnonNet Browser: $(grep MOZ_APP_VERSION branding/configure.sh)"
echo "Extension: $(jq -r .version ../extension/manifest.json)"
```

**security-audit.sh**
```bash
#!/bin/bash
# Run security checks

# Check for known vulnerabilities
npm audit
cargo audit

# Check dependency versions
# Verify patch levels
# Run automated security tests
```

**build-all-platforms.sh**
```bash
#!/bin/bash
# Build for all platforms (requires cross-compilation setup)

./build.sh --platform linux --architecture x86_64
./build.sh --platform linux --architecture arm64
./build.sh --platform darwin --architecture x86_64
./build.sh --platform darwin --architecture arm64
./build.sh --platform windows --architecture x86_64
```

---

## Resources

### Essential Links

- **Firefox ESR Releases**: https://www.mozilla.org/firefox/enterprise/
- **Mozilla Security Advisories**: https://www.mozilla.org/security/advisories/
- **Tor Browser GitLab**: https://gitlab.torproject.org/tpo/applications/tor-browser
- **Tor Browser Design Doc**: https://2019.www.torproject.org/projects/torbrowser/design/
- **Firefox Build Docs**: https://firefox-source-docs.mozilla.org/

### Communities

- **Mozilla Dev**: https://chat.mozilla.org/
- **Tor Project**: https://forum.torproject.net/
- **AnonNet**: https://github.com/a7maadf/anonnet/discussions

### Monitoring

**Set up alerts for:**
- Mozilla security advisories (RSS/email)
- Tor Browser releases (GitHub watch)
- CVE databases (NVD, Mitre)

```bash
# Subscribe to Mozilla Security Announce
# https://lists.mozilla.org/listinfo/dev-security-announce

# Watch Tor Browser repo
gh repo watch thetorproject/torbrowser --notifications
```

---

## Maintenance Log Template

```markdown
# Maintenance Log

## 2025-01-15 - Security Update 1.0.1

**Changes:**
- Updated Firefox ESR 128.6.0 → 128.7.0
- Applied CVE-2025-XXXX patch

**Testing:**
- All automated tests passed
- Manual security audit completed
- No regressions found

**Release:**
- Released at: 2025-01-15 14:00 UTC
- Platforms: Linux, macOS, Windows
- Download count: [track]

**Issues:**
- None reported

---

## 2026-02-01 - ESR Migration 2.0.0

...
```

---

**Remember:** Maintenance is ongoing. Security is paramount. When in doubt, delay release and test more.

**Questions?** Open a GitHub Discussion or contact the maintainers.

---

*Last Updated: 2025-11-21*
*Maintained by: AnonNet Project*
