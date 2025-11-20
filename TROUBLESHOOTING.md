# AnonNet Troubleshooting Guide

Quick fixes for common issues when testing the infrastructure.

---

## Issue: "Address already in use" when starting network

**Symptoms:**
```
Error: Failed to bind to address: Address already in use (os error 48)
```

**Cause:** Previous daemon processes are still running and holding ports 9000-9004.

**Solution:**

### Quick Fix (Recommended)
```bash
# Run the force cleanup script
~/Documents/anonnet/scripts/force-cleanup.sh

# Then start again
~/anonnet-test/start-network.sh
```

### Manual Fix
```bash
# Find all anonnet processes
ps aux | grep anonnet-daemon

# Kill them all
killall -9 anonnet-daemon

# Or kill individual PIDs
kill -9 <PID>

# Verify ports are free
lsof -i :9000
lsof -i :9001
# etc...
```

---

## Issue: Health check shows 0 healthy nodes

**Symptoms:**
```bash
~/anonnet-test/health-check.sh
# Shows: Summary: 0 healthy, 0 unhealthy
```

**Possible Causes:**

### 1. Nodes didn't start successfully

Check the logs:
```bash
# Check bootstrap log
tail -20 ~/anonnet-test/bootstrap/bootstrap.log

# Check node logs
tail -20 ~/anonnet-test/node1/node1.log

# Look for errors like:
# - "Failed to bind to address" → Port conflict
# - "No such file" → Daemon not built
# - Panic/crash messages → Code issue
```

### 2. Data directories not created

Check if nodes created their data files:
```bash
# Should see api_port.txt, socks5_port.txt, http_port.txt
ls -la ~/anonnet-test/bootstrap/data/
ls -la ~/anonnet-test/node1/data/
```

If empty, the daemon crashed before creating files. Check logs.

### 3. Daemon binary not found

```bash
# Check if daemon exists
ls -lh ~/Documents/anonnet/target/release/anonnet-daemon

# If not found, build it:
cd ~/Documents/anonnet
cargo build --release
```

### 4. Wrong paths (macOS vs Linux)

The scripts were originally written for Linux (`/home/user`) but you're on macOS (`/Users/ahmad`).

**Solution:** Re-run the setup script to regenerate scripts with correct paths:
```bash
cd ~/Documents/anonnet
./scripts/setup-test-network.sh
```

---

## Issue: Daemon crashes immediately

**Symptoms:**
Process starts then immediately exits. PID file exists but process is gone.

**Debug steps:**

```bash
# Run daemon directly to see error
cd ~/anonnet-test/bootstrap
~/Documents/anonnet/target/release/anonnet-daemon node

# Common errors:

# 1. Config file error
# Error: Failed to parse config...
# Fix: Check anonnet.toml syntax

# 2. Permission denied
# Error: Permission denied (os error 13)
# Fix: chmod +x on daemon, or check data dir permissions

# 3. Missing dependencies
# Error: dyld: Library not loaded...
# Fix: cargo clean && cargo build --release
```

---

## Issue: Nodes start but don't find peers

**Symptoms:**
- Health check shows peers: 0 for all nodes
- Logs don't show "Connected to peer" messages

**Possible Causes:**

### 1. Bootstrap node not running
```bash
# Check if bootstrap is alive
ps aux | grep anonnet | grep bootstrap

# Or check its PID
kill -0 $(cat ~/anonnet-test/bootstrap/bootstrap.pid)
```

### 2. Wrong bootstrap address in configs
```bash
# Verify node configs point to correct bootstrap
grep bootstrap_nodes ~/anonnet-test/node1/anonnet.toml
# Should show: bootstrap_nodes = ["127.0.0.1:9000"]
```

### 3. DHT needs more time
```bash
# Wait 60 seconds instead of 30
sleep 30
~/anonnet-test/health-check.sh
```

### 4. Firewall blocking localhost connections
```bash
# Test if ports are reachable
nc -zv 127.0.0.1 9000
nc -zv 127.0.0.1 9001
```

---

## Issue: Can't access API endpoints

**Symptoms:**
```bash
curl http://127.0.0.1:<port>/health
# curl: (7) Failed to connect to 127.0.0.1 port XXXXX: Connection refused
```

**Solutions:**

### 1. Check API port file exists
```bash
cat ~/anonnet-test/bootstrap/data/api_port.txt
# If file doesn't exist, daemon didn't start the API server
```

### 2. Check if daemon is running
```bash
ps aux | grep anonnet-daemon
# Should see running processes
```

### 3. Check daemon logs for API startup
```bash
grep -i "api" ~/anonnet-test/bootstrap/bootstrap.log
# Should see: "API server listening on 127.0.0.1:XXXXX"
```

### 4. API server might not be implemented
If the daemon starts but doesn't create API port file, the API server might not be fully implemented in the daemon code.

Check the daemon source:
```bash
grep -r "ApiServer" ~/Documents/anonnet/crates/daemon/src/
```

---

## Diagnostic Commands

Run these to gather information for debugging:

```bash
# 1. Full diagnostic
~/Documents/anonnet/scripts/debug-network.sh

# 2. Check what's running
ps aux | grep anonnet

# 3. Check what ports are in use
lsof -i :9000-9004

# 4. View recent logs
tail -50 ~/anonnet-test/bootstrap/bootstrap.log
tail -50 ~/anonnet-test/node1/node1.log

# 5. Check directory structure
tree ~/anonnet-test/ -L 2

# 6. Test daemon manually
cd ~/anonnet-test/bootstrap
~/Documents/anonnet/target/release/anonnet-daemon node
```

---

## Clean Slate - Start Over

If everything is broken, reset completely:

```bash
# 1. Force kill everything
killall -9 anonnet-daemon 2>/dev/null || true
pkill -f "python3 -m http.server" 2>/dev/null || true

# 2. Remove all test data
rm -rf ~/anonnet-test/*/data
rm -rf ~/anonnet-test/*/*.log
rm -rf ~/anonnet-test/*/*.pid

# 3. Re-setup
cd ~/Documents/anonnet
./scripts/setup-test-network.sh

# 4. Try again
~/anonnet-test/start-network.sh
```

---

## macOS-Specific Issues

### Issue: lsof permission denied
```bash
# Use with sudo
sudo lsof -i :9000

# Or check process list
ps aux | grep anonnet
```

### Issue: "Operation not permitted" when killing process
```bash
# Make sure you own the process
ps aux | grep anonnet
# Check USER column matches your username

# Kill with signal
kill -TERM <PID>
sleep 2
kill -KILL <PID>
```

### Issue: Port still in use after killing processes
```bash
# Find what's using it
sudo lsof -i :9000

# Kill by port (macOS)
sudo kill -9 $(sudo lsof -t -i :9000)
```

---

## Getting Help

If none of these solutions work:

1. **Run full diagnostics:**
   ```bash
   ~/Documents/anonnet/scripts/debug-network.sh > ~/anonnet-diagnostics.txt
   ```

2. **Collect logs:**
   ```bash
   tar czf ~/anonnet-logs.tar.gz ~/anonnet-test/*/bootstrap.log ~/anonnet-test/*/node*.log
   ```

3. **Check daemon version:**
   ```bash
   ~/Documents/anonnet/target/release/anonnet-daemon version
   ```

4. **Report the issue** with:
   - Output from debug-network.sh
   - Content of bootstrap.log
   - Your OS version: `uname -a`
   - Rust version: `rustc --version`

---

## Quick Reference

**Problem → Solution:**

| Problem | Quick Fix |
|---------|-----------|
| Port in use | `~/Documents/anonnet/scripts/force-cleanup.sh` |
| Daemon not found | `cd ~/Documents/anonnet && cargo build --release` |
| 0 healthy nodes | Check `~/anonnet-test/bootstrap/bootstrap.log` |
| No peers | Wait 60s, check bootstrap is running |
| Can't stop network | `killall -9 anonnet-daemon` |
| Total reset | `rm -rf ~/anonnet-test/*/data && ./scripts/setup-test-network.sh` |

---

## Expected Behavior (Success)

When everything works correctly, you should see:

```bash
# Starting network
~/anonnet-test/start-network.sh
# Output:
# ✅ Bootstrap PID: 12345
# ✅ Node 1 PID: 12346
# ✅ Node 2 PID: 12347
# ✅ Node 3 PID: 12348
# ✅ Node 4 PID: 12349
# ⏳ Waiting 30 seconds...
# ✅ Network ready!

# Health check
~/anonnet-test/health-check.sh
# Output:
# bootstrap:   ✅ Healthy | Peers: 4   | Circuits: 0   | Credits: 1000
# node1:       ✅ Healthy | Peers: 3-4 | Circuits: 0   | Credits: 1000
# node2:       ✅ Healthy | Peers: 3-4 | Circuits: 0   | Credits: 1000
# node3:       ✅ Healthy | Peers: 3-4 | Circuits: 0   | Credits: 1000
# node4:       ✅ Healthy | Peers: 3-4 | Circuits: 0   | Credits: 1000
```

If you see this, your infrastructure is working! 🎉
