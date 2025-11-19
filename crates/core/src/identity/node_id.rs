use super::PublicKey;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A unique identifier for a node in the network
///
/// Derived from the node's public key using BLAKE3 hash
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId([u8; 32]);

impl NodeId {
    /// Create a NodeId from a public key
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        let hash = blake3::hash(&public_key.as_bytes());
        Self(*hash.as_bytes())
    }

    /// Create a NodeId from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes of the NodeId
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to a hexadecimal string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from a hexadecimal string
    pub fn from_hex(s: &str) -> Result<Self, NodeIdError> {
        let bytes = hex::decode(s).map_err(|_| NodeIdError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(NodeIdError::InvalidLength);
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Self(array))
    }

    /// Calculate XOR distance to another NodeId (for DHT)
    pub fn distance(&self, other: &NodeId) -> Distance {
        let mut result = [0u8; 32];
        for i in 0..32 {
            result[i] = self.0[i] ^ other.0[i];
        }
        Distance(result)
    }

    /// Get a shortened display version (first 8 bytes as hex)
    pub fn short_hex(&self) -> String {
        hex::encode(&self.0[..8])
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({}...)", &self.short_hex())
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_hex())
    }
}

/// Distance between two NodeIds in the DHT keyspace
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Distance([u8; 32]);

impl Distance {
    /// Get the raw bytes of the distance
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Calculate the number of leading zero bits
    pub fn leading_zeros(&self) -> u32 {
        let mut count = 0;
        for byte in self.0.iter() {
            let zeros = byte.leading_zeros();
            count += zeros;
            if zeros < 8 {
                break;
            }
        }
        count
    }
}

impl fmt::Debug for Distance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Distance({}...)", hex::encode(&self.0[..4]))
    }
}

/// Errors related to NodeId operations
#[derive(Debug, thiserror::Error)]
pub enum NodeIdError {
    #[error("Invalid hexadecimal string")]
    InvalidHex,

    #[error("Invalid length (expected 32 bytes)")]
    InvalidLength,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_node_id_from_public_key() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();
        let node_id = NodeId::from_public_key(&public_key);

        assert_eq!(node_id.as_bytes().len(), 32);
    }

    #[test]
    fn test_node_id_deterministic() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();

        let node_id1 = NodeId::from_public_key(&public_key);
        let node_id2 = NodeId::from_public_key(&public_key);

        assert_eq!(node_id1, node_id2);
    }

    #[test]
    fn test_node_id_hex() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();
        let node_id = NodeId::from_public_key(&public_key);

        let hex = node_id.to_hex();
        let restored = NodeId::from_hex(&hex).unwrap();

        assert_eq!(node_id, restored);
    }

    #[test]
    fn test_node_id_distance() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let id1 = NodeId::from_public_key(&kp1.public_key());
        let id2 = NodeId::from_public_key(&kp2.public_key());

        let dist1 = id1.distance(&id2);
        let dist2 = id2.distance(&id1);

        // XOR distance is symmetric
        assert_eq!(dist1, dist2);

        // Distance to self is zero
        let dist_self = id1.distance(&id1);
        assert_eq!(dist_self.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_distance_ordering() {
        let id1 = NodeId::from_bytes([0u8; 32]);
        let id2 = NodeId::from_bytes([1u8; 32]);
        let id3 = NodeId::from_bytes([255u8; 32]);

        let dist_12 = id1.distance(&id2);
        let dist_13 = id1.distance(&id3);

        assert!(dist_12 < dist_13);
    }

    #[test]
    fn test_serialization() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        let serialized = bincode::serialize(&node_id).unwrap();
        let deserialized: NodeId = bincode::deserialize(&serialized).unwrap();

        assert_eq!(node_id, deserialized);
    }
}
