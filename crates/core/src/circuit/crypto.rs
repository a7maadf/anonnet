use super::types::{Circuit, CircuitNode};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};

/// Onion crypto manager for layered encryption/decryption
pub struct OnionCrypto;

impl OnionCrypto {
    /// Encrypt data with onion layers for a circuit
    ///
    /// Each layer is encrypted with the corresponding node's key,
    /// starting from the exit node and working backwards to the entry node.
    pub fn encrypt_onion(circuit: &Circuit, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut data = plaintext.to_vec();

        // Encrypt in reverse order (exit -> entry)
        // This way the entry node peels off the first layer
        for node in circuit.nodes.iter().rev() {
            data = Self::encrypt_layer(&node.encryption_key, &data)?;
        }

        Ok(data)
    }

    /// Decrypt one layer of onion encryption
    ///
    /// This is used by relay nodes to peel off their layer
    pub fn decrypt_layer(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = ChaCha20Poly1305::new(key.into());

        // Use a zero nonce - safe because each hop uses a unique key
        // In production, use proper nonce handling with counter or random nonces
        let nonce = Nonce::from_slice(&[0u8; 12]);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Encrypt one layer of onion encryption
    fn encrypt_layer(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = ChaCha20Poly1305::new(key.into());

        // Use a zero nonce - safe because each hop uses a unique key
        // In production, use proper nonce handling with counter or random nonces
        let nonce = Nonce::from_slice(&[0u8; 12]);

        cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)
    }

    /// Derive a nonce from data (for deterministic encryption)
    ///
    /// WARNING: In production, use proper random nonces or a counter!
    /// This is simplified for the prototype.
    fn derive_nonce(data: &[u8]) -> [u8; 12] {
        let hash = blake3::hash(data);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&hash.as_bytes()[..12]);
        nonce
    }

    /// Decrypt onion layers until we reach our layer
    ///
    /// Returns the decrypted payload and the number of layers peeled
    pub fn peel_onion_layers(
        keys: &[[u8; 32]],
        ciphertext: &[u8],
    ) -> Result<(Vec<u8>, usize), CryptoError> {
        let mut data = ciphertext.to_vec();
        let mut layers_peeled = 0;

        for key in keys {
            match Self::decrypt_layer(key, &data) {
                Ok(decrypted) => {
                    data = decrypted;
                    layers_peeled += 1;
                }
                Err(_) => {
                    // We've peeled all our layers
                    break;
                }
            }
        }

        if layers_peeled == 0 {
            return Err(CryptoError::NoLayersDecrypted);
        }

        Ok((data, layers_peeled))
    }

    /// Generate a random encryption key
    pub fn generate_key() -> [u8; 32] {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Derive shared secret using Diffie-Hellman (simplified)
    ///
    /// In production, use x25519 key exchange
    pub fn derive_shared_secret(our_secret: &[u8; 32], their_public: &[u8; 32]) -> [u8; 32] {
        // Simplified: just XOR and hash
        // Real implementation would use proper DH
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(our_secret);
        combined[32..].copy_from_slice(their_public);

        let hash = blake3::hash(&combined);
        *hash.as_bytes()
    }

    /// Create encryption and decryption keys from a shared secret
    pub fn derive_keys(shared_secret: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
        // Derive two keys using different prefixes
        let mut enc_input = [0u8; 33];
        enc_input[0] = 0x01; // Prefix for encryption key
        enc_input[1..].copy_from_slice(shared_secret);

        let mut dec_input = [0u8; 33];
        dec_input[0] = 0x02; // Prefix for decryption key
        dec_input[1..].copy_from_slice(shared_secret);

        let enc_key = *blake3::hash(&enc_input).as_bytes();
        let dec_key = *blake3::hash(&dec_input).as_bytes();

        (enc_key, dec_key)
    }
}

/// Cryptographic errors
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("No layers could be decrypted")]
    NoLayersDecrypted,

    #[error("Invalid key length")]
    InvalidKeyLength,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::types::{CircuitId, CircuitPurpose};
    use crate::identity::KeyPair;
    use crate::identity::NodeId;

    #[test]
    fn test_key_generation() {
        let key1 = OnionCrypto::generate_key();
        let key2 = OnionCrypto::generate_key();

        assert_ne!(key1, key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_single_layer_encryption() {
        let key = OnionCrypto::generate_key();
        let plaintext = b"Hello, AnonNet!";

        let ciphertext = OnionCrypto::encrypt_layer(&key, plaintext).unwrap();
        let decrypted = OnionCrypto::decrypt_layer(&key, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_onion_encryption_decryption() {
        // Create a test circuit with 3 hops
        let id = CircuitId::generate();
        let mut circuit = Circuit::new(id, CircuitPurpose::General);

        for _ in 0..3 {
            let keypair = KeyPair::generate();
            let node_id = NodeId::from_public_key(&keypair.public_key());
            // Use same key for encryption and decryption in this simplified version
            let key = OnionCrypto::generate_key();

            let node = CircuitNode::new(node_id, keypair.public_key(), key, key);
            circuit.add_node(node);
        }

        let plaintext = b"Secret message through AnonNet";

        // Encrypt with all layers
        let onion = OnionCrypto::encrypt_onion(&circuit, plaintext).unwrap();

        // Decrypt layer by layer (simulating relay nodes)
        let mut data = onion;

        for node in &circuit.nodes {
            data = OnionCrypto::decrypt_layer(&node.encryption_key, &data).unwrap();
        }

        assert_eq!(plaintext, data.as_slice());
    }

    #[test]
    fn test_derive_keys() {
        let shared_secret = OnionCrypto::generate_key();
        let (enc_key, dec_key) = OnionCrypto::derive_keys(&shared_secret);

        // Keys should be different
        assert_ne!(enc_key, dec_key);
        assert_ne!(enc_key, shared_secret);
        assert_ne!(dec_key, shared_secret);
    }

    #[test]
    fn test_peel_layers() {
        let key1 = OnionCrypto::generate_key();
        let key2 = OnionCrypto::generate_key();
        let key3 = OnionCrypto::generate_key();

        let plaintext = b"Multi-layer message";

        // Encrypt 3 layers
        let layer1 = OnionCrypto::encrypt_layer(&key3, plaintext).unwrap();
        let layer2 = OnionCrypto::encrypt_layer(&key2, &layer1).unwrap();
        let layer3 = OnionCrypto::encrypt_layer(&key1, &layer2).unwrap();

        // Peel all 3 layers
        let (decrypted, count) = OnionCrypto::peel_onion_layers(&[key1, key2, key3], &layer3).unwrap();

        assert_eq!(count, 3);
        assert_eq!(decrypted, plaintext);
    }
}
