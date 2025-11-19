use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

pub const NODE_ID_LEN: usize = 32;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid node id length: expected {expected}, got {actual}")]
    InvalidNodeId { expected: usize, actual: usize },
    #[error("invalid node id hex: {0}")]
    InvalidNodeIdHex(String),
    #[error("insufficient credits: available {available}, required {required}")]
    InsufficientCredits { available: u64, required: u64 },
    #[error("credit overflow")]
    CreditOverflow,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId([u8; NODE_ID_LEN]);

impl NodeId {
    pub fn from_bytes(bytes: [u8; NODE_ID_LEN]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, DomainError> {
        if bytes.len() != NODE_ID_LEN {
            return Err(DomainError::InvalidNodeId {
                expected: NODE_ID_LEN,
                actual: bytes.len(),
            });
        }

        let mut array = [0u8; NODE_ID_LEN];
        array.copy_from_slice(bytes);
        Ok(Self(array))
    }

    pub fn as_bytes(&self) -> &[u8; NODE_ID_LEN] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, DomainError> {
        let decoded =
            hex::decode(hex_str).map_err(|err| DomainError::InvalidNodeIdHex(err.to_string()))?;
        Self::from_slice(&decoded)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", hex::encode(self.0))
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl From<[u8; NODE_ID_LEN]> for NodeId {
    fn from(value: [u8; NODE_ID_LEN]) -> Self {
        Self::from_bytes(value)
    }
}

impl TryFrom<&[u8]> for NodeId {
    type Error = DomainError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::from_slice(value)
    }
}

impl FromStr for NodeId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditBalance {
    available: u64,
}

impl CreditBalance {
    pub fn new(amount: u64) -> Self {
        Self { available: amount }
    }

    pub fn available(&self) -> u64 {
        self.available
    }

    pub fn credit(&mut self, amount: u64) -> Result<(), DomainError> {
        self.available = self
            .available
            .checked_add(amount)
            .ok_or(DomainError::CreditOverflow)?;
        Ok(())
    }

    pub fn debit(&mut self, amount: u64) -> Result<(), DomainError> {
        if self.available < amount {
            return Err(DomainError::InsufficientCredits {
                available: self.available,
                required: amount,
            });
        }
        self.available -= amount;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    Control,
    Data,
    Credit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub from: NodeId,
    pub to: NodeId,
    pub nonce: u64,
    pub kind: MessageKind,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}

impl MessageEnvelope {
    pub fn new(
        from: NodeId,
        to: NodeId,
        nonce: u64,
        kind: MessageKind,
        payload: Vec<u8>,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            from,
            to,
            nonce,
            kind,
            payload,
            signature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_rejects_wrong_length() {
        let err = NodeId::try_from(&[1u8; 16][..]).unwrap_err();
        assert!(matches!(err, DomainError::InvalidNodeId { .. }));
    }

    #[test]
    fn node_id_parses_hex_roundtrip() {
        let hex_id = "ab".repeat(NODE_ID_LEN);
        let parsed = NodeId::from_hex(&hex_id).expect("should parse valid hex");
        assert_eq!(parsed.to_string(), hex_id);
    }

    #[test]
    fn node_id_rejects_bad_hex() {
        let err = NodeId::from_hex("not-hex").unwrap_err();
        assert!(matches!(err, DomainError::InvalidNodeIdHex(_)));
    }

    #[test]
    fn credit_balance_updates() {
        let mut balance = CreditBalance::new(100);
        balance.credit(50).unwrap();
        assert_eq!(balance.available(), 150);
        balance.debit(25).unwrap();
        assert_eq!(balance.available(), 125);
    }

    #[test]
    fn message_envelope_serializes() {
        let node = NodeId::from([1u8; NODE_ID_LEN]);
        let envelope =
            MessageEnvelope::new(node, node, 1, MessageKind::Data, vec![0, 1, 2], vec![9, 9]);
        let encoded = bincode::serialize(&envelope).unwrap();
        let decoded: MessageEnvelope = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, envelope);
    }
}
