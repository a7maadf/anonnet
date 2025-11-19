/// .anon service address system
///
/// Service addresses are derived from the service's public key,
/// similar to Tor's .onion addresses. This ensures authenticity
/// and prevents impersonation.

use crate::identity::PublicKey;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A .anon service address (e.g., "abc123...xyz.anon")
///
/// Format: [base32-encoded-hash].anon
/// The hash is derived from the service's public key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceAddress([u8; 32]);

impl ServiceAddress {
    /// Create a service address from a public key
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(b"ANONNET-SERVICE-V1");
        hasher.update(&public_key.as_bytes());
        Self(*hasher.finalize().as_bytes())
    }

    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to .anon hostname
    pub fn to_hostname(&self) -> String {
        format!("{}.anon", self.to_base32())
    }

    /// Convert to base32 string (without .anon suffix)
    pub fn to_base32(&self) -> String {
        data_encoding::BASE32_NOPAD.encode(&self.0).to_lowercase()
    }

    /// Parse from hostname (with or without .anon suffix)
    pub fn from_hostname(hostname: &str) -> Result<Self, ServiceAddressError> {
        let hostname = hostname.trim().to_lowercase();

        // Remove .anon suffix if present
        let base32_part = if hostname.ends_with(".anon") {
            &hostname[..hostname.len() - 5]
        } else {
            &hostname
        };

        // Decode base32
        let bytes = data_encoding::BASE32_NOPAD
            .decode(base32_part.to_uppercase().as_bytes())
            .map_err(|_| ServiceAddressError::InvalidEncoding)?;

        if bytes.len() != 32 {
            return Err(ServiceAddressError::InvalidLength(bytes.len()));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Self(array))
    }

    /// Check if a hostname is a .anon address
    pub fn is_anon_address(hostname: &str) -> bool {
        hostname.trim().to_lowercase().ends_with(".anon")
    }

    /// Verify that this address matches the given public key
    pub fn verify_public_key(&self, public_key: &PublicKey) -> bool {
        let expected = Self::from_public_key(public_key);
        self == &expected
    }
}

impl fmt::Display for ServiceAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hostname())
    }
}

/// Service address errors
#[derive(Debug, thiserror::Error)]
pub enum ServiceAddressError {
    #[error("Invalid base32 encoding")]
    InvalidEncoding,

    #[error("Invalid address length: {0} (expected 32)")]
    InvalidLength(usize),

    #[error("Not a .anon address")]
    NotAnonAddress,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_service_address_creation() {
        let keypair = KeyPair::generate();
        let address = ServiceAddress::from_public_key(&keypair.public_key());

        // Should produce valid hostname
        let hostname = address.to_hostname();
        assert!(hostname.ends_with(".anon"));
        assert!(hostname.len() > 10);
    }

    #[test]
    fn test_service_address_roundtrip() {
        let keypair = KeyPair::generate();
        let address = ServiceAddress::from_public_key(&keypair.public_key());

        // Convert to hostname and back
        let hostname = address.to_hostname();
        let parsed = ServiceAddress::from_hostname(&hostname).unwrap();

        assert_eq!(address, parsed);
    }

    #[test]
    fn test_service_address_verification() {
        let keypair = KeyPair::generate();
        let address = ServiceAddress::from_public_key(&keypair.public_key());

        // Should verify with correct key
        assert!(address.verify_public_key(&keypair.public_key()));

        // Should fail with wrong key
        let other_keypair = KeyPair::generate();
        assert!(!address.verify_public_key(&other_keypair.public_key()));
    }

    #[test]
    fn test_is_anon_address() {
        assert!(ServiceAddress::is_anon_address("test.anon"));
        assert!(ServiceAddress::is_anon_address("abc123.anon"));
        assert!(ServiceAddress::is_anon_address("TEST.ANON"));

        assert!(!ServiceAddress::is_anon_address("example.com"));
        assert!(!ServiceAddress::is_anon_address("test.onion"));
        assert!(!ServiceAddress::is_anon_address("clearnet.org"));
    }

    #[test]
    fn test_parse_with_and_without_suffix() {
        let keypair = KeyPair::generate();
        let address = ServiceAddress::from_public_key(&keypair.public_key());

        let base32 = address.to_base32();
        let hostname = address.to_hostname();

        // Should parse both formats
        let parsed1 = ServiceAddress::from_hostname(&base32).unwrap();
        let parsed2 = ServiceAddress::from_hostname(&hostname).unwrap();

        assert_eq!(address, parsed1);
        assert_eq!(address, parsed2);
    }
}
