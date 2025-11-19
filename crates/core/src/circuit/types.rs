use super::crypto::LayerCrypto;
use crate::identity::{NodeId, PublicKey};
use anonnet_common::{routing, Timestamp};
use serde::{Deserialize, Serialize};

/// Unique identifier for a circuit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CircuitId(pub u64);

impl CircuitId {
    pub fn generate() -> Self {
        use rand::Rng;
        Self(rand::thread_rng().gen())
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CircuitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circuit({})", self.0)
    }
}

/// State of a circuit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is being built (extending)
    Building,

    /// Circuit is ready for use
    Ready,

    /// Circuit is being torn down
    Closing,

    /// Circuit has failed
    Failed,

    /// Circuit has been closed
    Closed,
}

/// A single node (hop) in a circuit
#[derive(Debug, Clone)]
pub struct CircuitNode {
    /// Node ID
    pub node_id: NodeId,

    /// Node's public key
    pub public_key: PublicKey,

    /// Forward encryption layer (client -> server)
    pub forward_crypto: LayerCrypto,

    /// Backward encryption layer (server -> client)
    pub backward_crypto: LayerCrypto,

    /// When this node was added to the circuit
    pub added_at: Timestamp,
}

impl CircuitNode {
    /// Create a new circuit node with proper bidirectional encryption
    pub fn new(
        node_id: NodeId,
        public_key: PublicKey,
        forward_crypto: LayerCrypto,
        backward_crypto: LayerCrypto,
    ) -> Self {
        Self {
            node_id,
            public_key,
            forward_crypto,
            backward_crypto,
            added_at: Timestamp::now(),
        }
    }

    /// Legacy constructor for backwards compatibility (DEPRECATED - use new())
    ///
    /// This creates LayerCrypto from raw keys but loses the proper nonce counters.
    /// Only use for testing/migration.
    #[deprecated(note = "Use new() with proper LayerCrypto instances")]
    pub fn from_raw_keys(
        node_id: NodeId,
        public_key: PublicKey,
        _encryption_key: [u8; 32],
        _decryption_key: [u8; 32],
    ) -> Self {
        // For backwards compat, create dummy LayerCrypto
        // This is NOT secure - proper code should use real DH and LayerCrypto
        use super::crypto::{EphemeralKeyPair, OnionCrypto};
        let dummy_secret = EphemeralKeyPair::generate();
        let dummy_public = EphemeralKeyPair::generate();
        let shared = dummy_secret.diffie_hellman(dummy_public.public_key());
        let (forward, backward) = OnionCrypto::derive_bidirectional_keys(&shared);

        Self {
            node_id,
            public_key,
            forward_crypto: forward,
            backward_crypto: backward,
            added_at: Timestamp::now(),
        }
    }
}

/// Complete circuit path from origin to destination
#[derive(Debug, Clone)]
pub struct Circuit {
    /// Unique circuit ID
    pub id: CircuitId,

    /// Current state
    pub state: CircuitState,

    /// Nodes in the circuit (ordered from entry to exit)
    pub nodes: Vec<CircuitNode>,

    /// When the circuit was created
    pub created_at: Timestamp,

    /// When the circuit was last used
    pub last_used: Timestamp,

    /// Total bytes sent through this circuit
    pub bytes_sent: u64,

    /// Total bytes received through this circuit
    pub bytes_received: u64,

    /// Purpose of this circuit
    pub purpose: CircuitPurpose,
}

impl Circuit {
    pub fn new(id: CircuitId, purpose: CircuitPurpose) -> Self {
        Self {
            id,
            state: CircuitState::Building,
            nodes: Vec::new(),
            created_at: Timestamp::now(),
            last_used: Timestamp::now(),
            bytes_sent: 0,
            bytes_received: 0,
            purpose,
        }
    }

    /// Get the circuit length (number of hops)
    pub fn length(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the circuit is complete
    pub fn is_complete(&self) -> bool {
        self.length() >= routing::MIN_CIRCUIT_LENGTH
    }

    /// Check if the circuit is ready to use
    pub fn is_ready(&self) -> bool {
        self.state == CircuitState::Ready && self.is_complete()
    }

    /// Check if the circuit has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > routing::CIRCUIT_LIFETIME_SECS
    }

    /// Get the entry node (first hop)
    pub fn entry_node(&self) -> Option<&CircuitNode> {
        self.nodes.first()
    }

    /// Get the exit node (last hop)
    pub fn exit_node(&self) -> Option<&CircuitNode> {
        self.nodes.last()
    }

    /// Add a node to the circuit
    pub fn add_node(&mut self, node: CircuitNode) {
        self.nodes.push(node);

        // If we've reached the minimum length, mark as ready
        if self.is_complete() && self.state == CircuitState::Building {
            self.state = CircuitState::Ready;
        }
    }

    /// Mark the circuit as used
    pub fn mark_used(&mut self) {
        self.last_used = Timestamp::now();
    }

    /// Record bytes sent
    pub fn add_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.mark_used();
    }

    /// Record bytes received
    pub fn add_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.mark_used();
    }

    /// Get mutable access to forward encryption layers for encrypting cells
    pub fn forward_layers_mut(&mut self) -> Vec<&mut LayerCrypto> {
        self.nodes.iter_mut().map(|node| &mut node.forward_crypto).collect()
    }

    /// Get mutable access to backward encryption layers for decrypting cells
    pub fn backward_layers_mut(&mut self) -> Vec<&mut LayerCrypto> {
        self.nodes.iter_mut().map(|node| &mut node.backward_crypto).collect()
    }

    /// Mark the circuit as failed
    pub fn mark_failed(&mut self) {
        self.state = CircuitState::Failed;
    }

    /// Mark the circuit as closing
    pub fn mark_closing(&mut self) {
        self.state = CircuitState::Closing;
    }

    /// Mark the circuit as closed
    pub fn mark_closed(&mut self) {
        self.state = CircuitState::Closed;
    }
}

/// Purpose of a circuit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircuitPurpose {
    /// General anonymous communication
    General,

    /// Testing/measurement
    Testing,

    /// Directory lookups
    Directory,

    /// Rendezvous point for hidden services
    Rendezvous,

    /// Introduction point for hidden services
    Introduction,
}

/// Relay cell types (Tor-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayCellType {
    /// Begin a new stream
    Begin,

    /// Data for a stream
    Data,

    /// End a stream
    End,

    /// Acknowledge data
    Sendme,

    /// Extend the circuit
    Extend,

    /// Circuit extension succeeded
    Extended,

    /// Truncate the circuit
    Truncate,

    /// Circuit was truncated
    Truncated,

    /// Drop the cell
    Drop,
}

/// A relay cell (encrypted payload)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayCell {
    /// Cell type
    pub cell_type: RelayCellType,

    /// Stream ID (for multiplexing)
    pub stream_id: u16,

    /// Sequence number
    pub sequence: u32,

    /// Payload data
    pub payload: Vec<u8>,

    /// Digest/checksum
    pub digest: [u8; 4],
}

impl RelayCell {
    pub fn new(cell_type: RelayCellType, stream_id: u16, payload: Vec<u8>) -> Self {
        Self {
            cell_type,
            stream_id,
            sequence: 0,
            payload,
            digest: [0; 4],
        }
    }

    /// Calculate the digest for this cell
    pub fn calculate_digest(&self) -> [u8; 4] {
        use blake3;

        let mut hasher = blake3::Hasher::new();
        hasher.update(&[self.cell_type as u8]);
        hasher.update(&self.stream_id.to_be_bytes());
        hasher.update(&self.sequence.to_be_bytes());
        hasher.update(&self.payload);

        let hash = hasher.finalize();
        let mut digest = [0u8; 4];
        digest.copy_from_slice(&hash.as_bytes()[..4]);
        digest
    }

    /// Set the digest
    pub fn set_digest(&mut self) {
        self.digest = self.calculate_digest();
    }

    /// Verify the digest
    pub fn verify_digest(&self) -> bool {
        self.digest == self.calculate_digest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_circuit_id_generate() {
        let id1 = CircuitId::generate();
        let id2 = CircuitId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_circuit_creation() {
        let id = CircuitId::generate();
        let circuit = Circuit::new(id, CircuitPurpose::General);

        assert_eq!(circuit.id, id);
        assert_eq!(circuit.state, CircuitState::Building);
        assert_eq!(circuit.length(), 0);
        assert!(!circuit.is_complete());
    }

    #[test]
    fn test_circuit_add_nodes() {
        let id = CircuitId::generate();
        let mut circuit = Circuit::new(id, CircuitPurpose::General);

        // Add minimum required nodes
        #[allow(deprecated)]
        for _ in 0..routing::MIN_CIRCUIT_LENGTH {
            let keypair = KeyPair::generate();
            let node_id = NodeId::from_public_key(&keypair.public_key());
            let node = CircuitNode::from_raw_keys(
                node_id,
                keypair.public_key(),
                [0u8; 32],
                [0u8; 32],
            );
            circuit.add_node(node);
        }

        assert!(circuit.is_complete());
        assert_eq!(circuit.state, CircuitState::Ready);
    }

    #[test]
    fn test_circuit_entry_exit() {
        let id = CircuitId::generate();
        let mut circuit = Circuit::new(id, CircuitPurpose::General);

        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let kp3 = KeyPair::generate();

        let node1_id = NodeId::from_public_key(&kp1.public_key());
        let node2_id = NodeId::from_public_key(&kp2.public_key());
        let node3_id = NodeId::from_public_key(&kp3.public_key());

        #[allow(deprecated)]
        {
            circuit.add_node(CircuitNode::from_raw_keys(node1_id, kp1.public_key(), [0u8; 32], [0u8; 32]));
            circuit.add_node(CircuitNode::from_raw_keys(node2_id, kp2.public_key(), [0u8; 32], [0u8; 32]));
            circuit.add_node(CircuitNode::from_raw_keys(node3_id, kp3.public_key(), [0u8; 32], [0u8; 32]));
        }

        assert_eq!(circuit.entry_node().unwrap().node_id, node1_id);
        assert_eq!(circuit.exit_node().unwrap().node_id, node3_id);
    }

    #[test]
    fn test_relay_cell_digest() {
        let mut cell = RelayCell::new(RelayCellType::Data, 1, vec![1, 2, 3, 4, 5]);
        cell.set_digest();

        assert!(cell.verify_digest());

        // Tamper with payload
        cell.payload.push(6);
        assert!(!cell.verify_digest());
    }
}
