use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A cryptographic keypair for node identity and signing
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new random keypair (without PoW)
    ///
    /// Note: For production use, prefer `generate_with_pow()` to receive initial credits
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut rng = OsRng;
        let mut secret_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Generate a new keypair with Proof of Work
    ///
    /// This performs computational work to prove node capability and earn initial credits.
    /// Higher difficulty = more credits but takes longer to generate.
    ///
    /// Returns: (KeyPair, ProofOfWork)
    ///
    /// Note: PoW is generated for the NodeID (hash of public key), not the public key itself
    pub fn generate_with_pow(difficulty: u8) -> (Self, super::ProofOfWork) {
        use rand::RngCore;
        let mut rng = OsRng;
        let mut secret_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        let keypair = Self {
            signing_key,
            verifying_key,
        };

        // Mine PoW for the NodeID (hash of public key)
        let node_id = super::NodeId::from_public_key(&keypair.public_key());
        let node_id_bytes = *node_id.as_bytes();
        let pow = super::ProofOfWork::mine(&node_id_bytes, difficulty);

        (keypair, pow)
    }

    /// Create a keypair from a secret key
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Result<Self, KeyPairError> {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Get the secret key bytes
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the public key bytes
    pub fn public_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            key: self.verifying_key,
        }
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }

    /// Verify a signature on a message
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        self.verifying_key
            .verify(message, &Signature::from_bytes(signature))
            .is_ok()
    }
}

impl fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPair")
            .field("public_key", &hex::encode(self.public_bytes()))
            .field("secret_key", &"<redacted>")
            .finish()
    }
}

/// A public key for verifying signatures
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey {
    #[serde(with = "public_key_serde")]
    key: VerifyingKey,
}

impl PublicKey {
    /// Create a public key from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, KeyPairError> {
        let key = VerifyingKey::from_bytes(bytes)
            .map_err(|_| KeyPairError::InvalidPublicKey)?;
        Ok(Self { key })
    }

    /// Get the public key bytes
    pub fn as_bytes(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    /// Verify a signature on a message
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        self.key
            .verify(message, &Signature::from_bytes(signature))
            .is_ok()
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", hex::encode(self.as_bytes()))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_bytes()))
    }
}

/// Errors related to keypair operations
#[derive(Debug, thiserror::Error)]
pub enum KeyPairError {
    #[error("Invalid secret key")]
    InvalidSecretKey,

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("Invalid signature")]
    InvalidSignature,
}

// Custom serde for VerifyingKey
mod public_key_serde {
    use ed25519_dalek::VerifyingKey;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(key: &VerifyingKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        key.to_bytes().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<VerifyingKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: [u8; 32] = Deserialize::deserialize(deserializer)?;
        VerifyingKey::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        assert_eq!(keypair.secret_bytes().len(), 32);
        assert_eq!(keypair.public_bytes().len(), 32);
    }

    #[test]
    fn test_keypair_from_bytes() {
        let keypair1 = KeyPair::generate();
        let secret = keypair1.secret_bytes();

        let keypair2 = KeyPair::from_secret_bytes(&secret).unwrap();
        assert_eq!(keypair1.public_bytes(), keypair2.public_bytes());
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"Hello, AnonNet!";

        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature));

        // Wrong message should fail
        assert!(!keypair.verify(b"Wrong message", &signature));
    }

    #[test]
    fn test_public_key_verify() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();
        let message = b"Test message";

        let signature = keypair.sign(message);
        assert!(public_key.verify(message, &signature));
    }

    #[test]
    fn test_public_key_serialization() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();

        let serialized = bincode::serialize(&public_key).unwrap();
        let deserialized: PublicKey = bincode::deserialize(&serialized).unwrap();

        assert_eq!(public_key, deserialized);
    }
}
