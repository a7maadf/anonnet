# AnonNet Architecture

Technical deep-dive into how AnonNet works under the hood.

## Table of Contents

1. [System Overview](#system-overview)
2. [Network Layer](#network-layer)
3. [Circuit Construction](#circuit-construction)
4. [Hidden Services](#hidden-services)
5. [Credit System](#credit-system)
6. [Consensus Mechanism](#consensus-mechanism)
7. [Security Model](#security-model)
8. [Performance](#performance)

---

## System Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Applications                          │
│   (Browser, CLI, Custom Apps)                               │
└───────────────┬─────────────────────────────────────────────┘
                │
┌───────────────▼─────────────────────────────────────────────┐
│                   Proxy Layer (SOCKS5/HTTP)                  │
│   - Protocol translation                                     │
│   - .anon address resolution                                 │
│   - Clearnet blocking                                        │
└───────────────┬─────────────────────────────────────────────┘
                │
┌───────────────▼─────────────────────────────────────────────┐
│                      AnonNet Core                            │
│  ┌──────────────┬──────────────┬──────────────┬──────────┐  │
│  │   Circuits   │     DHT      │  Services    │  Credits │  │
│  │   Builder    │   Routing    │  Directory   │  Ledger  │  │
│  └──────────────┴──────────────┴──────────────┴──────────┘  │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │           Consensus & Blockchain Layer               │   │
│  │   (Proof-of-Work, Distributed Ledger)                │   │
│  └──────────────────────────────────────────────────────┘   │
└───────────────┬─────────────────────────────────────────────┘
                │
┌───────────────▼─────────────────────────────────────────────┐
│                   Transport Layer (QUIC)                     │
│   - UDP-based, encrypted connections                         │
│   - Multiplexing, flow control                               │
│   - Connection migration                                     │
└───────────────┬─────────────────────────────────────────────┘
                │
┌───────────────▼─────────────────────────────────────────────┐
│                  Internet (UDP/IP)                           │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

1. **Node**: Main entry point, manages all components
2. **DHT**: Kademlia-based distributed hash table for peer/service discovery
3. **Circuit Manager**: Builds and maintains multi-hop circuits
4. **Service Directory**: Publishes and discovers hidden services
5. **Credit Ledger**: Blockchain-based payment tracking
6. **Consensus**: Distributed agreement on network state
7. **Connection Manager**: QUIC-based encrypted connections
8. **Proxy Servers**: SOCKS5/HTTP interfaces for applications

---

## Network Layer

### Peer Discovery (DHT)

AnonNet uses a **Kademlia DHT** for decentralized peer discovery.

**Key Operations:**

```rust
// Find nodes closest to a target ID
pub async fn find_node(&self, target: NodeId) -> Vec<NodeInfo> {
    // 1. Query closest known nodes
    // 2. Recursively ask for closer nodes
    // 3. Return K closest nodes
}

// Store a value in the DHT
pub async fn store(&self, key: Key, value: Vec<u8>) -> Result<()> {
    // 1. Find K closest nodes to key
    // 2. Send STORE messages
    // 3. Replicate on K nodes
}

// Retrieve a value from the DHT
pub async fn find_value(&self, key: Key) -> Result<Vec<u8>> {
    // 1. Query closest nodes
    // 2. If found, return value
    // 3. If not, query next closest
}
```

**DHT Parameters:**
- K (bucket size): 20 nodes
- α (concurrency): 3 parallel queries
- Refresh interval: 1 hour
- Replication factor: 3 nodes

**Routing Table Structure:**

```
Distance (XOR metric)     Bucket
═══════════════════════════════
2^0  - 2^1                [node1, node2, ...]
2^1  - 2^2                [node3, node4, ...]
...
2^255 - 2^256             [node_n, ...]
```

---

### Transport (QUIC)

AnonNet uses **QUIC** instead of TCP for several advantages:

**Why QUIC?**
1. **Built-in encryption**: TLS 1.3 integrated
2. **Multiplexing**: Multiple streams without head-of-line blocking
3. **Connection migration**: Survives IP changes (mobile networks)
4. **0-RTT**: Fast reconnection
5. **UDP-based**: Better for NAT traversal

**Connection Establishment:**

```
Client                                  Server
  |                                        |
  |---- Initial Packet (ClientHello) ---->|
  |                                        |
  |<--- Handshake (ServerHello, Cert) ----|
  |                                        |
  |---- Handshake (Finished) ------------->|
  |                                        |
  |<====== Encrypted Application Data ====>|
```

**Streams:**

```rust
// Open a new stream for a message
let (mut send, mut recv) = connection.open_bi().await?;

// Send message
send.write_all(&message_bytes).await?;

// Read response
let mut response = Vec::new();
recv.read_to_end(&mut response).await?;
```

---

## Circuit Construction

### Multi-Hop Circuits

AnonNet builds **3-hop circuits** for anonymity (configurable):

```
You ← → Entry ← → Middle ← → Exit ← → Destination
        Node      Node       Node       (.anon service)
```

**Why 3 hops?**
- Entry node: Knows your IP, doesn't know destination
- Middle node: Knows neither
- Exit node: Knows destination, doesn't know your IP

**Circuit Building Process:**

```
1. SELECT NODES
   ├─ Choose entry node (guard node)
   ├─ Choose middle node (high-bandwidth)
   └─ Choose exit node (reliable)

2. EXTEND TO ENTRY
   ├─ Create DH ephemeral keypair
   ├─ Send CREATE_CIRCUIT to entry
   ├─ Entry responds with its DH public key
   └─ Derive shared secret

3. EXTEND TO MIDDLE (through entry)
   ├─ Create new DH keypair
   ├─ Send EXTEND to entry (encrypted)
   ├─ Entry forwards to middle
   ├─ Middle responds (encrypted)
   └─ Derive shared secret with middle

4. EXTEND TO EXIT (through entry + middle)
   ├─ Create new DH keypair
   ├─ Send EXTEND (doubly encrypted)
   ├─ Exit responds (doubly encrypted)
   └─ Derive shared secret with exit

5. CIRCUIT READY
   └─ Now have 3 layer onion encryption
```

**Onion Encryption:**

```rust
// Encrypt data for circuit
fn encrypt_for_circuit(data: &[u8], circuit: &Circuit) -> Vec<u8> {
    let mut encrypted = data.to_vec();

    // Layer 3: Exit node encryption
    encrypted = exit_crypto.encrypt(&encrypted);

    // Layer 2: Middle node encryption
    encrypted = middle_crypto.encrypt(&encrypted);

    // Layer 1: Entry node encryption
    encrypted = entry_crypto.encrypt(&encrypted);

    encrypted
}

// Each node peels one layer
fn relay_cell(encrypted: &[u8], node_crypto: &Crypto) -> Vec<u8> {
    node_crypto.decrypt(encrypted)
}
```

---

### Path Selection

**Node Selection Criteria:**

```rust
pub struct PathSelectionConstraints {
    // Must be different /16 subnets (network diversity)
    subnet_diversity: bool,

    // Minimum bandwidth requirement (bytes/sec)
    min_bandwidth: u64,

    // Minimum uptime (seconds)
    min_uptime: u64,

    // Exclude nodes in same country (if known)
    geographic_diversity: bool,

    // Exclude low-reputation nodes
    min_reputation: f64,
}
```

**Guard Nodes:**

Entry nodes are selected from a small **guard list** to prevent profiling attacks:

- Choose 3 guard nodes
- Use them exclusively for entry
- Rotate every 30 days
- Fallback if guards offline

---

## Hidden Services

### Service Registration

```rust
pub struct ServiceDescriptor {
    // Service identity
    public_key: PublicKey,          // Ed25519
    service_address: ServiceAddress, // Hash of public key

    // Introduction points
    introduction_points: Vec<IntroductionPoint>,

    // Metadata
    protocol: Protocol,              // HTTP, HTTPS, TCP, etc.
    version: u32,
    timestamp: SystemTime,

    // Cryptographic signature
    signature: Signature,
}

pub struct IntroductionPoint {
    node_id: NodeId,
    address: SocketAddr,
    public_key: PublicKey,
}
```

**Publishing Flow:**

```
Service                          DHT
  |                               |
  |-- 1. Generate descriptor ---->|
  |                               |
  |<-- 2. Find K closest nodes ---|
  |                               |
  |-- 3. Store on K nodes ------->|
  |                               |
  |-- 4. Re-publish every 6h ---->|
```

---

### Service Connection

**Client → Service Connection:**

```
CLIENT SIDE:
1. Lookup service descriptor in DHT
   └─ GET(hash("abc123def456.anon"))

2. Extract introduction points from descriptor
   └─ [intro1, intro2, intro3]

3. Build circuit to intro point
   └─ You ← → Entry ← → Middle ← → IntroPoint

4. Choose rendezvous point
   └─ Random node from DHT

5. Send INTRODUCE cell to intro point
   └─ Contains: rendezvous point, rendezvous cookie

SERVICE SIDE:
6. Intro point forwards INTRODUCE to service

7. Service builds circuit to rendezvous point
   └─ Service ← → Entry ← → Middle ← → Rendezvous

8. Service sends RENDEZVOUS cell
   └─ Contains: rendezvous cookie

RENDEZVOUS POINT:
9. Matches client and service circuits

10. Joins circuits together
    └─ Client ← → ... ← → Rendezvous ← → ... ← → Service

11. End-to-end encrypted tunnel established
```

**Mutual Anonymity:**
- Client doesn't know service's IP
- Service doesn't know client's IP
- Rendezvous point knows neither
- Introduction points only know service

---

## Credit System

### Transaction Model

```rust
pub struct Transaction {
    from: NodeId,
    to: NodeId,
    amount: Credits,
    reason: TransactionReason,
    timestamp: SystemTime,
    signature: Signature,
}

pub enum TransactionReason {
    CircuitRelay { bytes: u64 },
    ServiceAccess { service: ServiceAddress },
    DHTStorage { key: Key },
    InitialAllocation,
}
```

### Earning Credits

**1. Relaying Traffic**

```rust
// When relaying a cell
async fn relay_cell(&mut self, cell: RelayCell) -> Result<()> {
    // Decrypt and forward
    let decrypted = self.crypto.decrypt(&cell.payload)?;
    self.forward_to_next_hop(decrypted).await?;

    // Record relay for credit
    self.credit_tracker.record_relay(
        from: cell.circuit_id,
        bytes: cell.payload.len() as u64,
    );

    // Credits earned = bytes * relay_rate
    // Example: 1024 bytes * 0.001 = 1.024 credits
}
```

**2. Hosting Services**

```rust
// When accepting a service connection
async fn accept_service_connection(&mut self, intro: IntroduceCell) -> Result<()> {
    // Charge for connection
    let fee = self.service_config.connection_fee;
    self.credit_tracker.charge(intro.client_id, fee);

    // Charge per byte transferred
    // Handled by relay mechanism above
}
```

**3. DHT Participation**

```rust
// When storing a value in DHT
async fn handle_store(&mut self, key: Key, value: Vec<u8>) -> Result<()> {
    self.dht.store(key, value)?;

    // Earn credits for storage
    let storage_fee = (value.len() as u64) * STORAGE_RATE;
    self.credit_tracker.earn(storage_fee);
}
```

---

### Spending Credits

**1. Building Circuits**

```rust
// When creating a circuit
async fn build_circuit(&mut self) -> Result<CircuitId> {
    let nodes = self.select_path()?;

    // Pay each node in circuit
    for node in &nodes {
        let fee = CIRCUIT_ESTABLISHMENT_FEE;
        self.credit_tracker.pay(node.id, fee)?;
    }

    // Continue building...
}
```

**2. Using Bandwidth**

```rust
// When sending data through circuit
async fn send_through_circuit(&mut self, data: &[u8]) -> Result<()> {
    let bytes = data.len() as u64;
    let total_cost = bytes * RELAY_RATE * circuit.hop_count;

    // Pay each relay node
    for node in circuit.nodes {
        let node_cost = bytes * RELAY_RATE;
        self.credit_tracker.pay(node.id, node_cost)?;
    }

    // Send data...
}
```

---

### Consensus & Blockchain

AnonNet uses a **lightweight blockchain** for credit consensus:

```rust
pub struct Block {
    height: u64,
    prev_hash: Hash,
    timestamp: SystemTime,
    transactions: Vec<Transaction>,
    miner: NodeId,
    nonce: u64,
    hash: Hash,
}
```

**Proof-of-Work:**

```rust
// Mine a block
pub fn mine_block(transactions: Vec<Transaction>) -> Block {
    let mut nonce = 0u64;
    let target = DIFFICULTY_TARGET;

    loop {
        let block = Block {
            transactions: transactions.clone(),
            nonce,
            // ... other fields
        };

        let hash = block.calculate_hash();

        if hash < target {
            return block; // Found valid block!
        }

        nonce += 1;
    }
}
```

**Difficulty:** 12 leading zero bits (configurable)

**Block time:** ~10 minutes

**Finality:** 6 confirmations (~1 hour)

---

## Security Model

### Threat Model

**What AnonNet protects against:**

1. **Traffic Analysis**
   - ✅ ISP can't see what you're accessing
   - ✅ Websites can't see your IP
   - ✅ Relay nodes can't correlate traffic

2. **Censorship**
   - ✅ No central authority to block services
   - ✅ DHT distributes service discovery
   - ✅ Circuit routing avoids specific nodes

3. **Surveillance**
   - ✅ End-to-end encryption
   - ✅ Onion routing prevents linkability
   - ✅ Hidden services protect both parties

**What AnonNet does NOT protect against:**

1. **Global Passive Adversary**
   - ❌ Entity monitoring ALL network traffic
   - ❌ Can correlate timing/volume patterns
   - **Mitigation**: Use guards, padding, timing obfuscation

2. **Malicious Majority**
   - ❌ If >50% of nodes are malicious
   - ❌ Can correlate circuits
   - **Mitigation**: Path diversity, reputation system

3. **Application-Level Leaks**
   - ❌ JavaScript can fingerprint browser
   - ❌ Cookies can identify users
   - **Mitigation**: Use hardened browser, disable JS

4. **Endpoint Security**
   - ❌ Malware on your device
   - ❌ Compromised service operators
   - **Mitigation**: Use trusted devices, verify services

---

### Cryptographic Primitives

**Identity & Signing:**
- **Ed25519**: Digital signatures (node IDs, transactions)

**Key Exchange:**
- **X25519**: Diffie-Hellman for circuit keys

**Symmetric Encryption:**
- **ChaCha20-Poly1305**: Fast, authenticated encryption

**Hashing:**
- **SHA-256**: Block hashes, node IDs
- **SHA-3**: Service addresses

**Transport:**
- **TLS 1.3**: QUIC handshake

---

### Anonymity Guarantees

**Circuit-Level:**
- Entry node: Knows your IP, doesn't know destination
- Middle node: Knows neither
- Exit node: Knows destination, doesn't know your IP

**Service-Level:**
- Introduction points: Know service, don't know clients
- Rendezvous points: Connect circuits, know neither party
- DHT: Service descriptors public, but not service location

**Unlinkability:**
- Different circuits for different services
- Circuit rotation (new circuit every 10 minutes)
- Stream isolation (different apps use different circuits)

---

## Performance

### Latency

**Circuit Building:**
- 3 round trips (one per hop)
- ~300-500ms total (depends on node locations)

**Data Transfer:**
- 3 additional hops vs. direct
- ~2-3x latency overhead
- Typical: 100-300ms for web pages

**Optimization:**
- Circuit pooling (reuse circuits)
- Predictive circuit building
- Faster relay node selection

---

### Throughput

**Bottlenecks:**
1. Slowest node in circuit (limits bandwidth)
2. Encryption/decryption overhead (~10%)
3. Relay queuing (if nodes overloaded)

**Typical Speeds:**
- Text browsing: 500 KB/s - 2 MB/s
- Video streaming: 1-5 MB/s (if good circuit)
- Large downloads: Limited by weakest link

**Optimization:**
- Choose high-bandwidth relays
- Multiple parallel circuits
- Adaptive circuit building

---

### Scalability

**DHT:**
- O(log N) lookup time (N = number of nodes)
- 1M nodes: ~20 hops to find anything
- Scales horizontally

**Consensus:**
- Blockchain grows linearly with transactions
- Pruning old blocks reduces storage
- Light clients don't need full chain

**Circuit Pool:**
- Pre-build circuits for instant use
- Limit pool size to conserve credits
- Typical: 3-5 pre-built circuits

---

## Implementation Details

### Code Structure

```
crates/
├── core/           # Core AnonNet logic
│   ├── circuit/    # Circuit building, relay
│   ├── consensus/  # Blockchain, PoW
│   ├── crypto/     # Cryptographic primitives
│   ├── dht/        # Kademlia DHT
│   ├── network/    # QUIC, connections
│   ├── service/    # Hidden services
│   └── node.rs     # Main Node orchestrator
│
├── daemon/         # Daemon application
│   ├── api/        # REST API server
│   ├── proxy/      # SOCKS5/HTTP proxies
│   └── main.rs     # Entry point
│
├── common/         # Shared types
│   ├── config.rs   # Configuration
│   └── types.rs    # Common data structures
│
└── protocol/       # Wire protocol
    └── messages.rs # Protocol messages
```

### Key Files

**`crates/core/src/node.rs`** (450 lines)
- Main Node struct
- Coordinates all components
- Public API for applications

**`crates/core/src/circuit/builder.rs`** (580 lines)
- Circuit construction algorithm
- Path selection
- Onion encryption

**`crates/core/src/dht/mod.rs`** (750 lines)
- Kademlia DHT implementation
- Peer discovery
- Value storage/retrieval

**`crates/core/src/service/directory.rs`** (420 lines)
- Service descriptor management
- Publishing and lookup
- Introduction point selection

**`crates/daemon/src/proxy/socks5.rs`** (340 lines)
- SOCKS5 proxy server
- .anon address resolution
- Clearnet blocking

---

## Next Steps

- **[Browser Usage Guide](BROWSER_USAGE.md)**: Learn how to use the browser
- **[Hosting Guide](HOSTING_GUIDE.md)**: Host your own .anon services
- **[API Reference](API_REFERENCE.md)**: Full API documentation
- **[Developer Guide](DEVELOPER_GUIDE.md)**: Build on AnonNet
