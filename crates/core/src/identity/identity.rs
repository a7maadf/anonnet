use super::{KeyPair, NodeId, PublicKey};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Complete identity for a node in the network
///
/// Contains the keypair, public key, and derived NodeId
#[derive(Clone)]
pub struct Identity {
    keypair: KeyPair,
    node_id: NodeId,
}

impl Identity {
    /// Generate a new random identity
    pub fn generate() -> Self {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        Self { keypair, node_id }
    }

    /// Create an identity from a secret key
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Result<Self, super::KeyPairError> {
        let keypair = KeyPair::from_secret_bytes(bytes)?;
        let node_id = NodeId::from_public_key(&keypair.public_key());

        Ok(Self { keypair, node_id })
    }

    /// Create an identity from an existing keypair
    pub fn from_keypair(keypair: KeyPair) -> Self {
        let node_id = NodeId::from_public_key(&keypair.public_key());
        Self { keypair, node_id }
    }

    /// Get the keypair
    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        self.keypair.public_key()
    }

    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.keypair.sign(message)
    }

    /// Verify a signature on a message using this identity's public key
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        self.keypair.verify(message, signature)
    }

    /// Get the secret key bytes (use carefully!)
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.keypair.secret_bytes()
    }

    /// Export to a saveable format
    pub fn to_exportable(&self) -> ExportableIdentity {
        ExportableIdentity {
            secret_key: self.keypair.secret_bytes(),
        }
    }

    /// Import from a saved format
    pub fn from_exportable(exportable: &ExportableIdentity) -> Result<Self, super::KeyPairError> {
        Self::from_secret_bytes(&exportable.secret_key)
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identity")
            .field("node_id", &self.node_id)
            .field("public_key", &self.public_key())
            .finish()
    }
}

/// Exportable/serializable format for saving identity to disk
#[derive(Serialize, Deserialize)]
pub struct ExportableIdentity {
    secret_key: [u8; 32],
}

impl ExportableIdentity {
    /// Save to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate();
        assert_eq!(identity.secret_bytes().len(), 32);
    }

    #[test]
    fn test_identity_sign_verify() {
        let identity = Identity::generate();
        let message = b"AnonNet test message";

        let signature = identity.sign(message);
        assert!(identity.verify(message, &signature));
    }

    #[test]
    fn test_identity_export_import() {
        let identity1 = Identity::generate();
        let exportable = identity1.to_exportable();

        let identity2 = Identity::from_exportable(&exportable).unwrap();

        assert_eq!(identity1.node_id(), identity2.node_id());
        assert_eq!(identity1.public_key(), identity2.public_key());
    }

    #[test]
    fn test_identity_json_serialization() {
        let identity = Identity::generate();
        let exportable = identity.to_exportable();

        let json = exportable.to_json().unwrap();
        let restored = ExportableIdentity::from_json(&json).unwrap();

        let identity2 = Identity::from_exportable(&restored).unwrap();
        assert_eq!(identity.node_id(), identity2.node_id());
    }
}
