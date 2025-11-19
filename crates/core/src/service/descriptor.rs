/// Service descriptor system
///
/// Descriptors contain information about .anon services including
/// introduction points and public keys for establishing connections.

use crate::identity::{NodeId, PublicKey};
use crate::service::ServiceAddress;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Ed25519 signature wrapper (64 bytes) with serde support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(#[serde(with = "serde_bytes")] pub [u8; 64]);

/// A service descriptor published by .anon services
///
/// Contains information needed to connect to the service:
/// - Service public key (for verification)
/// - Introduction points (rendezvous circuit endpoints)
/// - Metadata (version, expiry, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    /// Version of the descriptor format
    pub version: u8,

    /// Service address (derived from public key)
    pub address: ServiceAddress,

    /// Service's public key
    pub public_key: PublicKey,

    /// Introduction points where clients can establish rendezvous
    pub introduction_points: Vec<IntroductionPoint>,

    /// When this descriptor was created
    pub created_at: SystemTime,

    /// How long this descriptor is valid
    pub ttl: Duration,

    /// Signature over the descriptor (proves ownership)
    pub signature: Signature,
}

/// An introduction point for a service
///
/// Clients contact these nodes to establish a rendezvous
/// circuit with the service without revealing the service's location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntroductionPoint {
    /// Node ID of the introduction point
    pub node_id: NodeId,

    /// Public key for encrypted communication
    pub public_key: PublicKey,

    /// Connection information
    pub connection_info: ConnectionInfo,

    /// Proof that this node agreed to be an introduction point
    pub auth_signature: Signature,
}

/// Connection information for an introduction point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// IP addresses (can be multiple for multi-homing)
    pub addresses: Vec<String>,

    /// Port number
    pub port: u16,

    /// Protocol version
    pub protocol_version: u8,
}

impl ServiceDescriptor {
    /// Create a new service descriptor
    pub fn new(
        public_key: PublicKey,
        introduction_points: Vec<IntroductionPoint>,
        ttl: Duration,
    ) -> Self {
        let address = ServiceAddress::from_public_key(&public_key);

        Self {
            version: 1,
            address,
            public_key,
            introduction_points,
            created_at: SystemTime::now(),
            ttl,
            signature: Signature([0u8; 64]), // Will be set by sign()
        }
    }

    /// Sign the descriptor with the service's private key
    pub fn sign(&mut self, keypair: &crate::identity::KeyPair) {
        let data = self.signing_data();
        self.signature = Signature(keypair.sign(&data));
    }

    /// Verify the descriptor signature
    pub fn verify(&self) -> bool {
        let data = self.signing_data();
        self.public_key.verify(&data, &self.signature.0)
    }

    /// Check if the descriptor is expired
    pub fn is_expired(&self) -> bool {
        match SystemTime::now().duration_since(self.created_at) {
            Ok(elapsed) => elapsed > self.ttl,
            Err(_) => true, // Clock went backwards, consider expired
        }
    }

    /// Get the data to be signed
    fn signing_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[self.version]);
        data.extend_from_slice(self.address.as_bytes());
        data.extend_from_slice(&self.public_key.as_bytes());

        // Add introduction points
        for intro in &self.introduction_points {
            data.extend_from_slice(intro.node_id.as_bytes());
        }

        // Add timestamp
        if let Ok(duration) = self.created_at.duration_since(SystemTime::UNIX_EPOCH) {
            data.extend_from_slice(&duration.as_secs().to_le_bytes());
        }

        data.extend_from_slice(&self.ttl.as_secs().to_le_bytes());
        data
    }

    /// Verify that the address matches the public key
    pub fn verify_address(&self) -> bool {
        self.address.verify_public_key(&self.public_key)
    }
}

/// Errors that can occur with service descriptors
#[derive(Debug, thiserror::Error)]
pub enum DescriptorError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Descriptor expired")]
    Expired,

    #[error("Invalid address (doesn't match public key)")]
    InvalidAddress,

    #[error("No introduction points")]
    NoIntroductionPoints,

    #[error("Too many introduction points: {0} (max 10)")]
    TooManyIntroductionPoints(usize),

    #[error("Invalid TTL: {0:?}")]
    InvalidTTL(Duration),
}

impl ServiceDescriptor {
    /// Validate the descriptor
    pub fn validate(&self) -> Result<(), DescriptorError> {
        // Check signature
        if !self.verify() {
            return Err(DescriptorError::InvalidSignature);
        }

        // Check address matches public key
        if !self.verify_address() {
            return Err(DescriptorError::InvalidAddress);
        }

        // Check expiry
        if self.is_expired() {
            return Err(DescriptorError::Expired);
        }

        // Check introduction points
        if self.introduction_points.is_empty() {
            return Err(DescriptorError::NoIntroductionPoints);
        }

        if self.introduction_points.len() > 10 {
            return Err(DescriptorError::TooManyIntroductionPoints(
                self.introduction_points.len(),
            ));
        }

        // Check TTL is reasonable (1 hour to 24 hours)
        if self.ttl < Duration::from_secs(3600) || self.ttl > Duration::from_secs(86400) {
            return Err(DescriptorError::InvalidTTL(self.ttl));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_descriptor_creation() {
        let keypair = KeyPair::generate();
        let intro_point = create_test_intro_point();

        let mut descriptor = ServiceDescriptor::new(
            keypair.public_key(),
            vec![intro_point],
            Duration::from_secs(3600),
        );

        // Should not verify without signature
        assert!(!descriptor.verify());

        // Sign it
        descriptor.sign(&keypair);

        // Should verify now
        assert!(descriptor.verify());
    }

    #[test]
    fn test_descriptor_validation() {
        let keypair = KeyPair::generate();
        let intro_point = create_test_intro_point();

        let mut descriptor = ServiceDescriptor::new(
            keypair.public_key(),
            vec![intro_point],
            Duration::from_secs(3600),
        );

        descriptor.sign(&keypair);

        // Should pass validation
        assert!(descriptor.validate().is_ok());
    }

    #[test]
    fn test_descriptor_expiry() {
        let keypair = KeyPair::generate();
        let intro_point = create_test_intro_point();

        let mut descriptor = ServiceDescriptor::new(
            keypair.public_key(),
            vec![intro_point],
            Duration::from_secs(0), // Instant expiry
        );

        descriptor.sign(&keypair);

        // Should be expired
        assert!(descriptor.is_expired());
    }

    #[test]
    fn test_address_verification() {
        let keypair = KeyPair::generate();
        let intro_point = create_test_intro_point();

        let descriptor = ServiceDescriptor::new(
            keypair.public_key(),
            vec![intro_point],
            Duration::from_secs(3600),
        );

        // Address should match public key
        assert!(descriptor.verify_address());
    }

    fn create_test_intro_point() -> IntroductionPoint {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        IntroductionPoint {
            node_id,
            public_key: keypair.public_key(),
            connection_info: ConnectionInfo {
                addresses: vec!["127.0.0.1".to_string()],
                port: 9001,
                protocol_version: 1,
            },
            auth_signature: Signature([0u8; 64]),
        }
    }
}

