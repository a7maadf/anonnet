use crate::identity::NodeId;
use crate::protocol::Signature64;
use anonnet_common::{Credits, Timestamp};
use serde::{Deserialize, Serialize};

/// A transaction in the credit system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    /// Unique transaction ID
    pub id: TransactionId,

    /// Transaction type and details
    pub tx_type: TransactionType,

    /// When the transaction was created
    pub timestamp: Timestamp,

    /// Signature from the sender
    pub signature: Signature64,

    /// Nonce to prevent replay attacks
    pub nonce: u64,
}

impl Transaction {
    pub fn new(tx_type: TransactionType, nonce: u64) -> Self {
        Self {
            id: TransactionId::generate(),
            tx_type,
            timestamp: Timestamp::now(),
            signature: Signature64([0u8; 64]),
            nonce,
        }
    }

    pub fn with_signature(mut self, signature: Signature64) -> Self {
        self.signature = signature;
        self
    }

    /// Get the sender of this transaction
    pub fn sender(&self) -> NodeId {
        match &self.tx_type {
            TransactionType::Transfer { from, .. } => *from,
            TransactionType::RelayReward { relay_node, .. } => *relay_node,
            TransactionType::Genesis { recipient, .. } => *recipient,
        }
    }

    /// Get the amount transferred in this transaction
    pub fn amount(&self) -> Credits {
        match &self.tx_type {
            TransactionType::Transfer { amount, .. } => *amount,
            TransactionType::RelayReward { amount, .. } => *amount,
            TransactionType::Genesis { amount, .. } => *amount,
        }
    }

    /// Calculate the hash of this transaction for signing
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // Hash transaction data (excluding signature)
        hasher.update(&self.id.0.to_le_bytes());
        hasher.update(&bincode::serialize(&self.tx_type).unwrap());
        hasher.update(&self.timestamp.as_secs().to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());

        *hasher.finalize().as_bytes()
    }
}

/// Unique transaction identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(pub u64);

impl TransactionId {
    pub fn generate() -> Self {
        use rand::Rng;
        Self(rand::thread_rng().gen())
    }

    pub fn from_u64(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tx({})", self.0)
    }
}

/// Types of transactions in the credit system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    /// Direct credit transfer between nodes
    Transfer {
        from: NodeId,
        to: NodeId,
        amount: Credits,
    },

    /// Reward for relaying traffic
    RelayReward {
        /// Node that performed the relay
        relay_node: NodeId,

        /// Circuit ID that was used
        circuit_id: u64,

        /// Bytes relayed
        bytes_relayed: u64,

        /// Credits earned
        amount: Credits,

        /// Proof of relay
        proof: RelayProof,
    },

    /// Genesis transaction (initial credit distribution based on PoW)
    Genesis {
        recipient: NodeId,
        amount: Credits,
        pow: crate::identity::ProofOfWork,
    },
}

/// Proof that a node relayed traffic
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayProof {
    /// Circuit ID
    pub circuit_id: u64,

    /// Number of cells relayed
    pub cells_relayed: u64,

    /// Bytes relayed
    pub bytes_relayed: u64,

    /// Timestamp of relay activity
    pub timestamp: Timestamp,

    /// Signatures from sender and receiver (optional)
    pub sender_signature: Option<Signature64>,
    pub receiver_signature: Option<Signature64>,

    /// Hash of relayed data (for verification)
    pub data_hash: [u8; 32],
}

impl RelayProof {
    pub fn new(circuit_id: u64, cells_relayed: u64, bytes_relayed: u64) -> Self {
        Self {
            circuit_id,
            cells_relayed,
            bytes_relayed,
            timestamp: Timestamp::now(),
            sender_signature: None,
            receiver_signature: None,
            data_hash: [0u8; 32],
        }
    }

    /// Verify the relay proof
    pub fn verify(&self) -> bool {
        // Basic validation
        if self.bytes_relayed == 0 {
            return false;
        }

        if self.cells_relayed == 0 {
            return false;
        }

        // Check timestamp is recent (within last hour)
        if self.timestamp.elapsed().as_secs() > 3600 {
            return false;
        }

        // In production, would verify signatures from sender/receiver
        true
    }

    /// Calculate credits earned for this relay
    pub fn calculate_credits(&self) -> Credits {
        use anonnet_common::credits;

        // Base calculation: credits_per_gb * (bytes / 1GB)
        let gb = self.bytes_relayed as f64 / (1024.0 * 1024.0 * 1024.0);
        let credits = (gb * credits::CREDITS_PER_GB as f64) as u64;

        Credits::new(credits.max(1)) // Minimum 1 credit
    }
}

/// Transaction validation errors
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: u64, available: u64 },

    #[error("Invalid amount: {0}")]
    InvalidAmount(u64),

    #[error("Replay attack detected (duplicate nonce)")]
    ReplayAttack,

    #[error("Invalid relay proof")]
    InvalidRelayProof,

    #[error("Transaction expired")]
    Expired,

    #[error("Invalid sender")]
    InvalidSender,

    #[error("Credit transfers between identities are not allowed")]
    TransferNotAllowed,

    #[error("Invalid Proof of Work")]
    InvalidPoW,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_transaction_id_generation() {
        let id1 = TransactionId::generate();
        let id2 = TransactionId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_transaction_creation() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let from = NodeId::from_public_key(&kp1.public_key());
        let to = NodeId::from_public_key(&kp2.public_key());

        let tx = Transaction::new(
            TransactionType::Transfer {
                from,
                to,
                amount: Credits::new(100),
            },
            1,
        );

        assert_eq!(tx.sender(), from);
        assert_eq!(tx.amount(), Credits::new(100));
    }

    #[test]
    fn test_relay_proof() {
        let proof = RelayProof::new(12345, 100, 50000);

        assert!(proof.verify());
        assert!(proof.calculate_credits().amount() > 0);
    }

    #[test]
    fn test_relay_proof_expired() {
        let mut proof = RelayProof::new(12345, 100, 50000);

        // Set timestamp to 2 hours ago
        proof.timestamp = Timestamp::from_secs(Timestamp::now().as_secs() - 7200);

        assert!(!proof.verify());
    }

    #[test]
    fn test_relay_proof_credits() {
        // 1 GB = 1000 credits
        let proof = RelayProof::new(1, 1000, 1024 * 1024 * 1024);
        let credits = proof.calculate_credits();

        assert_eq!(credits.amount(), 1000);
    }

    #[test]
    fn test_transaction_hash() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let from = NodeId::from_public_key(&kp1.public_key());
        let to = NodeId::from_public_key(&kp2.public_key());

        let tx1 = Transaction::new(
            TransactionType::Transfer {
                from,
                to,
                amount: Credits::new(100),
            },
            1,
        );

        let tx2 = Transaction::new(
            TransactionType::Transfer {
                from,
                to,
                amount: Credits::new(100),
            },
            1,
        );

        // Same data should produce different hashes due to different IDs
        assert_ne!(tx1.hash(), tx2.hash());
    }
}
