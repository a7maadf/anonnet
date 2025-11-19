use crate::identity::{NodeId, PublicKey};
use anonnet_common::{Credits, NetworkAddress, Timestamp};
use serde::{Deserialize, Serialize};

/// Wrapper type for 64-byte signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature64(pub [u8; 64]);

impl Serialize for Signature64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Signature64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Signature64Visitor;

        impl<'de> serde::de::Visitor<'de> for Signature64Visitor {
            type Value = Signature64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a 64-byte signature")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 64];
                for i in 0..64 {
                    arr[i] = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Signature64(arr))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 64 {
                    return Err(E::invalid_length(v.len(), &self));
                }
                let mut arr = [0u8; 64];
                arr.copy_from_slice(v);
                Ok(Signature64(arr))
            }
        }

        deserializer.deserialize_seq(Signature64Visitor)
    }
}

/// Protocol message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message type and payload
    pub payload: MessagePayload,

    /// Message ID for tracking
    pub message_id: MessageId,

    /// Timestamp when message was created
    pub timestamp: Timestamp,

    /// Optional signature for authenticated messages
    pub signature: Option<Signature64>,
}

impl Message {
    pub fn new(payload: MessagePayload) -> Self {
        Self {
            payload,
            message_id: MessageId::generate(),
            timestamp: Timestamp::now(),
            signature: None,
        }
    }

    pub fn with_signature(mut self, signature: [u8; 64]) -> Self {
        self.signature = Some(Signature64(signature));
        self
    }

    pub fn message_type(&self) -> &str {
        self.payload.message_type()
    }
}

/// Unique message identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId([u8; 16]);

impl MessageId {
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// All possible message types in the protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessagePayload {
    // Connection management
    Handshake(HandshakeMessage),
    HandshakeResponse(HandshakeResponse),

    // Health checks
    Ping(PingMessage),
    Pong(PongMessage),

    // Peer discovery
    FindNode(FindNodeMessage),
    NodesFound(NodesFoundMessage),

    // DHT storage
    Store(StoreMessage),
    StoreResponse(StoreResponseMessage),
    FindValue(FindValueMessage),
    ValueFound(ValueFoundMessage),

    // Circuit management
    CreateCircuit(CreateCircuitMessage),
    CircuitCreated(CircuitCreatedMessage),
    CircuitFailed(CircuitFailedMessage),
    DestroyCircuit(DestroyCircuitMessage),

    // Data relay
    RelayData(RelayDataMessage),
    RelayAck(RelayAckMessage),

    // Credit system
    CreditTransfer(CreditTransferMessage),
    CreditBalance(CreditBalanceMessage),

    // General response
    Error(ErrorMessage),
}

impl MessagePayload {
    pub fn message_type(&self) -> &str {
        match self {
            Self::Handshake(_) => "handshake",
            Self::HandshakeResponse(_) => "handshake_response",
            Self::Ping(_) => "ping",
            Self::Pong(_) => "pong",
            Self::FindNode(_) => "find_node",
            Self::NodesFound(_) => "nodes_found",
            Self::Store(_) => "store",
            Self::StoreResponse(_) => "store_response",
            Self::FindValue(_) => "find_value",
            Self::ValueFound(_) => "value_found",
            Self::CreateCircuit(_) => "create_circuit",
            Self::CircuitCreated(_) => "circuit_created",
            Self::CircuitFailed(_) => "circuit_failed",
            Self::DestroyCircuit(_) => "destroy_circuit",
            Self::RelayData(_) => "relay_data",
            Self::RelayAck(_) => "relay_ack",
            Self::CreditTransfer(_) => "credit_transfer",
            Self::CreditBalance(_) => "credit_balance",
            Self::Error(_) => "error",
        }
    }
}

// ============================================================================
// Connection Messages
// ============================================================================

/// Initial handshake when connecting to a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Node ID of the sender
    pub node_id: NodeId,

    /// Public key of the sender
    pub public_key: PublicKey,

    /// Protocol version
    pub protocol_version: u32,

    /// Listen addresses
    pub addresses: Vec<NetworkAddress>,

    /// Whether this node accepts relay traffic
    pub accepts_relay: bool,

    /// Challenge nonce for authentication
    pub nonce: [u8; 32],
}

/// Response to handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// Node ID of the responder
    pub node_id: NodeId,

    /// Public key of the responder
    pub public_key: PublicKey,

    /// Protocol version
    pub protocol_version: u32,

    /// Listen addresses
    pub addresses: Vec<NetworkAddress>,

    /// Whether this node accepts relay traffic
    pub accepts_relay: bool,

    /// Signed challenge from handshake
    pub challenge_signature: Signature64,

    /// Success or error
    pub success: bool,

    /// Optional error message
    pub error: Option<String>,
}

// ============================================================================
// Health Check Messages
// ============================================================================

/// Ping message to check if peer is alive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    /// Random nonce
    pub nonce: u64,
}

/// Pong response to ping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    /// Echo back the nonce from ping
    pub nonce: u64,
}

// ============================================================================
// Peer Discovery Messages
// ============================================================================

/// Request to find nodes close to a target ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindNodeMessage {
    /// Target node ID to find peers close to
    pub target: NodeId,

    /// Number of nodes to return
    pub count: usize,
}

/// Response with found nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesFoundMessage {
    /// Nodes found close to the target
    pub nodes: Vec<PeerInfo>,
}

/// Information about a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub public_key: PublicKey,
    pub addresses: Vec<NetworkAddress>,
    pub last_seen: Timestamp,
}

// ============================================================================
// DHT Storage Messages
// ============================================================================

/// Store a value in the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMessage {
    /// Key to store under (32-byte hash)
    pub key: [u8; 32],

    /// Value to store
    pub value: Vec<u8>,

    /// Publisher's node ID
    pub publisher: NodeId,

    /// TTL in seconds
    pub ttl: u64,

    /// Optional signature
    pub signature: Option<Signature64>,
}

/// Response to a store request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResponseMessage {
    /// Whether the store succeeded
    pub success: bool,

    /// Optional error message
    pub error: Option<String>,
}

/// Find a value in the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindValueMessage {
    /// Key to find
    pub key: [u8; 32],
}

/// Response with found values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueFoundMessage {
    /// Whether value was found
    pub found: bool,

    /// Found values (may be multiple from different publishers)
    pub values: Vec<StoredValueMessage>,

    /// If not found, closest nodes that might have it
    pub closest_nodes: Vec<PeerInfo>,
}

/// Stored value in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredValueMessage {
    /// The stored data
    pub data: Vec<u8>,

    /// Publisher's node ID
    pub publisher: NodeId,

    /// When it was stored (Unix timestamp)
    pub stored_at: u64,

    /// TTL in seconds
    pub ttl: u64,

    /// Optional signature
    pub signature: Option<Signature64>,
}

// ============================================================================
// Circuit Messages
// ============================================================================

/// Request to create a circuit through this node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCircuitMessage {
    /// Circuit ID
    pub circuit_id: CircuitId,

    /// Next hop (if this is not the exit node)
    pub next_hop: Option<NodeId>,

    /// Encrypted payload for next hop
    pub encrypted_payload: Option<Vec<u8>>,
}

/// Response when circuit is successfully created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitCreatedMessage {
    /// Circuit ID
    pub circuit_id: CircuitId,

    /// Success status
    pub success: bool,
}

/// Response when circuit creation fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitFailedMessage {
    /// Circuit ID
    pub circuit_id: CircuitId,

    /// Reason for failure
    pub reason: String,
}

/// Request to tear down a circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyCircuitMessage {
    /// Circuit ID to destroy
    pub circuit_id: CircuitId,
}

/// Unique circuit identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CircuitId(pub u64);

impl CircuitId {
    pub fn generate() -> Self {
        use rand::Rng;
        Self(rand::thread_rng().gen())
    }
}

// ============================================================================
// Relay Messages
// ============================================================================

/// Relay data through a circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayDataMessage {
    /// Circuit ID
    pub circuit_id: CircuitId,

    /// Encrypted payload
    pub payload: Vec<u8>,

    /// Sequence number
    pub sequence: u64,
}

/// Acknowledgment of relayed data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayAckMessage {
    /// Circuit ID
    pub circuit_id: CircuitId,

    /// Sequence number acknowledged
    pub sequence: u64,

    /// Bytes relayed
    pub bytes_relayed: u64,
}

// ============================================================================
// Credit Messages
// ============================================================================

/// Transfer credits to another node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferMessage {
    /// Recipient node ID
    pub to: NodeId,

    /// Amount to transfer
    pub amount: Credits,

    /// Nonce to prevent replay
    pub nonce: u64,

    /// Signature of (to, amount, nonce)
    pub signature: Signature64,
}

/// Query or report credit balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBalanceMessage {
    /// Node ID
    pub node_id: NodeId,

    /// Current balance
    pub balance: Credits,
}

// ============================================================================
// Error Messages
// ============================================================================

/// Generic error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Error code
    pub code: ErrorCode,

    /// Human-readable error message
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidMessage,
    AuthenticationFailed,
    InsufficientCredits,
    CircuitCreationFailed,
    NodeNotFound,
    ProtocolVersionMismatch,
    InternalError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_id_generation() {
        let id1 = MessageId::generate();
        let id2 = MessageId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_message_serialization() {
        let ping = Message::new(MessagePayload::Ping(PingMessage { nonce: 12345 }));

        // Test with JSON serialization (bincode has issues with Option<custom types>)
        let serialized = serde_json::to_string(&ping).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        assert_eq!(ping.message_id, deserialized.message_id);
    }

    #[test]
    fn test_circuit_id_generation() {
        let id1 = CircuitId::generate();
        let id2 = CircuitId::generate();
        assert_ne!(id1, id2);
    }
}
