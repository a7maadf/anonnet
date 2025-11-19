## 🌐 What is AnonNet?

AnonNet is a next-generation anonymous network that reimagines how privacy and performance can coexist on the internet. Unlike traditional anonymity networks that rely on volunteer nodes and often suffer from poor performance, AnonNet introduces a novel **credit-based economy** where users earn credits by relaying traffic and spend credits to use the network.

### The Problem We're Solving

Current anonymity networks like Tor face several critical challenges:

- **Poor Performance**: Slow speeds due to volunteer relay constraints
- **Centralization Risk**: Directory authorities create single points of failure
- **Unequal Contribution**: Most users consume bandwidth without contributing
- **Limited Scalability**: Difficult to scale beyond current size
- **Attack Surface**: Known vulnerabilities in routing and consensus mechanisms

### Our Solution

AnonNet addresses these issues through:

1. **Peer-to-Peer Architecture**: No central authorities or directory servers
2. **Economic Incentives**: Credit system encourages widespread relay participation
3. **Modern Protocols**: QUIC transport, multi-path routing for 2-3x faster speeds
4. **Computational Merit**: Powerful hardware earns more credits, improving network quality
5. **Distributed Consensus**: Lightweight Byzantine fault-tolerant consensus for credit tracking
6. **Strong Anonymity**: Multi-hop routing with layered encryption and traffic analysis resistance

---

## ✨ Key Features

### 🔐 Privacy & Anonymity

- **Multi-Hop Routing**: Traffic routes through 3+ nodes by default
- **Layered Encryption**: Onion/garlic routing with perfect forward secrecy
- **Traffic Analysis Resistance**: Timing obfuscation, cover traffic, message padding
- **No Metadata Leakage**: Circuit-level isolation, no DNS leaks
- **Identity Protection**: Cryptographic identities with optional proof-of-work
- **Unlinkability**: Different identities for different services

### ⚡ Performance

- **QUIC Transport**: Modern UDP-based protocol with built-in encryption
- **Multi-Path Routing**: Parallel circuits for better throughput
- **Adaptive Circuits**: Network learns optimal paths over time
- **Low Latency**: Optimized for interactive applications (web, chat, VoIP)
- **Bandwidth Shaping**: Smart bandwidth allocation based on credits
- **Connection Pooling**: Reuse circuits across multiple streams

### 🪙 Credit Economy

- **Earn by Relaying**: Automatically earn credits by forwarding traffic
- **Spend to Use**: Use credits to send your own anonymous traffic
- **Fair Pricing**: Dynamic pricing based on network load
- **Fraud Prevention**: Proof-of-relay system with random auditing
- **Reputation System**: Long-term participants earn higher reputation
- **No Cryptocurrency**: Internal credits, not a blockchain or token

### 🌍 Decentralization

- **No Central Servers**: Fully peer-to-peer architecture
- **DHT-Based Discovery**: Kademlia-style distributed hash table
- **Distributed Consensus**: PBFT-like consensus among validators
- **Resilience**: No single point of failure
- **Censorship Resistant**: No central authority to block or shut down
- **Community Governed**: Network parameters controlled by participants

### 🛠️ Developer Friendly

- **Protocol Agnostic**: Support for TCP, UDP, HTTP, WebRTC, and custom protocols
- **Simple SDK**: Easy-to-use libraries for Rust, Python, JavaScript
- **SOCKS5/HTTP Proxy**: Works with existing applications
- **.anon TLD**: Distributed naming system for anonymous services
- **API Access**: REST and gRPC APIs for integration
- **Extensible**: Plugin system for custom protocols and features

---

## 🏗️ Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                        Application Layer                     │
│  (Web Browser, Chat App, File Sharing, Custom Applications) │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                      Proxy Layer (SOCKS5/HTTP)              │
│          Translates normal traffic to anonymous routing     │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                      Protocol Layer                          │
│     Streams (TCP-like) • Datagrams (UDP-like) • HTTP        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Anonymous Routing Layer                    │
│  Circuit Creation • Path Selection • Multi-hop Forwarding   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    Credit & Consensus Layer                  │
│   Credit Tracking • Relay Proofs • Validator Consensus      │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                     P2P Network Layer                        │
│     Peer Discovery (DHT) • QUIC Transport • NAT Traversal   │
└─────────────────────────────────────────────────────────────┘
```

### How It Works

#### 1. **Network Participation**

Every user runs a daemon that:
- Connects to other peers via DHT-based discovery
- Maintains connections to 20-50 peers
- Optionally relays traffic to earn credits
- Creates circuits for anonymous communication

#### 2. **Circuit Creation**

When you want to send anonymous traffic:

```
You → Node A → Node B → Node C → Destination
```

- **Your Node**: Encrypts data 3 times (layers for A, B, C)
- **Node A**: Decrypts outer layer, forwards to B
- **Node B**: Decrypts next layer, forwards to C
- **Node C**: Decrypts final layer, forwards to destination
- **Return Path**: Separate circuit in reverse direction

Each relay only knows:
- Previous hop (where it came from)
- Next hop (where to forward it)
- **Not**: The source, destination, or full path

#### 3. **Credit System**

```
Earning Credits:
- Relay 1 GB of traffic → Earn ~1000 credits
- Credits earned = f(bandwidth, latency, uptime, reputation)
- Higher performance nodes earn more per GB

Spending Credits:
- Send 1 GB of traffic → Spend ~1000 credits
- Prices adjust based on network congestion
- Heavy users pay more during peak times

Balance:
- Start with initial credits (1000)
- Earn more by relaying
- Network automatically balances itself
```

#### 4. **Consensus Mechanism**

For credit tracking without a central server:

- **Validators**: Top-reputation nodes (21+ validators)
- **Blocks**: Every 30 seconds, validators create a new block
- **Transactions**: Credit transfers from user to relay nodes
- **Consensus**: PBFT-like voting among validators
- **Finality**: Confirmed transactions are irreversible

#### 5. **Relay Proof System**

To prevent fraud (claiming to relay without actually doing it):

```
Sender → Relay → Receiver
         ↓
    Generates Proof
         ↓
    Submits to Validators
         ↓
    Random Audits
         ↓
    Credit Award or Penalty
```

