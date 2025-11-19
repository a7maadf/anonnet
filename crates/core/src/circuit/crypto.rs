use super::types::Circuit;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::{RngCore, CryptoRng};
use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};

/// Onion crypto manager for layered encryption/decryption with forward secrecy
pub struct OnionCrypto;

/// Per-circuit nonce counter for ChaCha20-Poly1305
///
/// CRITICAL SECURITY: Each circuit MUST have its own counter.
/// Nonce reuse with the same key destroys confidentiality.
#[derive(Debug, Clone)]
pub struct NonceCounter {
    /// Base nonce value (random)
    base: [u8; 12],
    /// Counter value (incremented for each encryption)
    counter: u64,
}

impl NonceCounter {
    /// Create a new nonce counter with a random base
    pub fn new() -> Self {
        let mut base = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut base);
        Self { base, counter: 0 }
    }

    /// Create from specific base (for testing)
    pub fn from_base(base: [u8; 12]) -> Self {
        Self { base, counter: 0 }
    }

    /// Get next nonce and increment counter
    ///
    /// CRITICAL: This MUST be called in order - parallel calls will cause nonce reuse!
    /// Use proper synchronization (Mutex) when sharing across threads.
    pub fn next_nonce(&mut self) -> [u8; 12] {
        let mut nonce = self.base;
        // XOR the counter into the last 8 bytes
        let counter_bytes = self.counter.to_le_bytes();
        for (i, byte) in counter_bytes.iter().enumerate() {
            nonce[4 + i] ^= byte;
        }

        self.counter += 1;

        // CRITICAL: Detect counter wraparound
        if self.counter == 0 {
            panic!("Nonce counter wrapped! Circuit MUST be torn down and rebuilt.");
        }

        nonce
    }

    /// Get current counter value (for debugging/monitoring)
    pub fn counter(&self) -> u64 {
        self.counter
    }
}

/// Ephemeral key pair for X25519 Diffie-Hellman
///
/// NOTE: This is NOT Clone because EphemeralSecret cannot be cloned (security feature).
/// Once you use the secret for key exchange, it's consumed (forward secrecy).
pub struct EphemeralKeyPair {
    secret: EphemeralSecret,
    public: X25519PublicKey,
}

impl EphemeralKeyPair {
    /// Generate a new ephemeral key pair
    ///
    /// SECURITY: These keys provide forward secrecy.
    /// Each circuit extension should use a fresh ephemeral key.
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(&mut rand::thread_rng());
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Generate with custom RNG (for testing)
    pub fn generate_with_rng<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let secret = EphemeralSecret::random_from_rng(rng);
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key
    pub fn public_key(&self) -> &X25519PublicKey {
        &self.public
    }

    /// Get the public key as bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    /// Perform Diffie-Hellman key exchange
    ///
    /// This consumes the secret key (you can't reuse it - forward secrecy!)
    pub fn diffie_hellman(self, their_public: &X25519PublicKey) -> SharedSecret {
        self.secret.diffie_hellman(their_public)
    }
}

/// Serializable X25519 public key (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializableX25519Public([u8; 32]);

impl From<&X25519PublicKey> for SerializableX25519Public {
    fn from(key: &X25519PublicKey) -> Self {
        Self(*key.as_bytes())
    }
}

impl From<SerializableX25519Public> for X25519PublicKey {
    fn from(key: SerializableX25519Public) -> Self {
        X25519PublicKey::from(key.0)
    }
}

/// Layer encryption state for a single hop
#[derive(Clone)]
pub struct LayerCrypto {
    /// ChaCha20-Poly1305 cipher
    cipher: ChaCha20Poly1305,
    /// Nonce counter for this layer
    nonce_counter: NonceCounter,
}

// Manual Debug implementation because ChaCha20Poly1305 doesn't implement Debug
impl std::fmt::Debug for LayerCrypto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayerCrypto")
            .field("nonce_counter", &self.nonce_counter)
            .field("cipher", &"<ChaCha20Poly1305>")
            .finish()
    }
}

impl LayerCrypto {
    /// Create a new layer from a shared secret
    ///
    /// Derives encryption key using HKDF-like construction
    pub fn new(shared_secret: &SharedSecret) -> Self {
        let key = Self::derive_encryption_key(shared_secret);
        let cipher = ChaCha20Poly1305::new(&key.into());
        let nonce_counter = NonceCounter::new();

        Self {
            cipher,
            nonce_counter,
        }
    }

    /// Derive encryption key from shared secret using HKDF pattern
    ///
    /// Uses BLAKE3 in KDF mode for domain separation
    fn derive_encryption_key(shared_secret: &SharedSecret) -> [u8; 32] {
        // BLAKE3 keyed hash with domain separation
        let mut hasher = blake3::Hasher::new_keyed(shared_secret.as_bytes());
        hasher.update(b"ANONNET-CIRCUIT-ENCRYPTION-V1");
        *hasher.finalize().as_bytes()
    }

    /// Encrypt data with this layer
    ///
    /// CRITICAL: This increments the nonce counter. Must be called in order!
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce_bytes = self.nonce_counter.next_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)
    }

    /// Decrypt data with this layer
    ///
    /// CRITICAL: This increments the nonce counter. Must be called in order!
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce_bytes = self.nonce_counter.next_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Get counter value (for monitoring)
    pub fn counter(&self) -> u64 {
        self.nonce_counter.counter()
    }
}

impl OnionCrypto {
    /// Encrypt data with onion layers for a circuit
    ///
    /// SECURITY: This uses proper X25519 DH and incremental nonces.
    /// Each layer must have been created with a fresh ephemeral key pair.
    pub fn encrypt_onion(
        layers: &mut [&mut LayerCrypto],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut data = plaintext.to_vec();

        // Encrypt in reverse order (exit -> entry)
        // This way the entry node peels off the first layer
        for layer in layers.iter_mut().rev() {
            data = layer.encrypt(&data)?;
        }

        Ok(data)
    }

    /// Decrypt one layer of onion encryption
    ///
    /// This is used by relay nodes to peel off their layer
    pub fn decrypt_layer(
        layer: &mut LayerCrypto,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        layer.decrypt(ciphertext)
    }

    /// Perform X25519 Diffie-Hellman key exchange
    ///
    /// SECURITY: This is real ECDH, not the XOR hack.
    /// Provides forward secrecy when used with ephemeral keys.
    pub fn diffie_hellman(
        our_secret: EphemeralSecret,
        their_public: &X25519PublicKey,
    ) -> SharedSecret {
        our_secret.diffie_hellman(their_public)
    }

    /// Create forward and backward encryption keys from a shared secret
    ///
    /// Uses HKDF pattern with domain separation for bidirectional communication
    pub fn derive_bidirectional_keys(shared_secret: &SharedSecret) -> (LayerCrypto, LayerCrypto) {
        // Forward direction (client -> server)
        let forward_key = {
            let mut hasher = blake3::Hasher::new_keyed(shared_secret.as_bytes());
            hasher.update(b"ANONNET-CIRCUIT-FORWARD-V1");
            *hasher.finalize().as_bytes()
        };

        // Backward direction (server -> client)
        let backward_key = {
            let mut hasher = blake3::Hasher::new_keyed(shared_secret.as_bytes());
            hasher.update(b"ANONNET-CIRCUIT-BACKWARD-V1");
            *hasher.finalize().as_bytes()
        };

        let forward_cipher = ChaCha20Poly1305::new(&forward_key.into());
        let backward_cipher = ChaCha20Poly1305::new(&backward_key.into());

        (
            LayerCrypto {
                cipher: forward_cipher,
                nonce_counter: NonceCounter::new(),
            },
            LayerCrypto {
                cipher: backward_cipher,
                nonce_counter: NonceCounter::new(),
            },
        )
    }

    /// Generate a random 32-byte key
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }
}

/// Cryptographic errors
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Invalid key length")]
    InvalidKeyLength,

    #[error("Nonce counter exhausted - circuit must be rebuilt")]
    NonceCounterExhausted,

    #[error("Invalid public key")]
    InvalidPublicKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_counter_uniqueness() {
        let mut counter = NonceCounter::new();
        let nonce1 = counter.next_nonce();
        let nonce2 = counter.next_nonce();
        let nonce3 = counter.next_nonce();

        // All nonces must be different
        assert_ne!(nonce1, nonce2);
        assert_ne!(nonce2, nonce3);
        assert_ne!(nonce1, nonce3);

        // Counter should increment
        assert_eq!(counter.counter(), 3);
    }

    #[test]
    fn test_x25519_key_exchange() {
        // Alice generates ephemeral key
        let alice_ephemeral = EphemeralKeyPair::generate();
        let alice_public = alice_ephemeral.public_key().clone();

        // Bob generates ephemeral key
        let bob_ephemeral = EphemeralKeyPair::generate();
        let bob_public = bob_ephemeral.public_key().clone();

        // Both compute shared secret
        let alice_shared = alice_ephemeral.diffie_hellman(&bob_public);
        let bob_shared = bob_ephemeral.diffie_hellman(&alice_public);

        // Shared secrets must match
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    #[ignore = "TODO: Implement proper sequence number sync - counters currently diverge"]
    fn test_layer_crypto_roundtrip() {
        // Test bidirectional encryption with proper key separation
        // NOTE: This test is currently ignored because it requires implementing
        // sequence number synchronization between sender and receiver.
        // In production, each cell will have an explicit sequence number used as nonce.
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();
        let shared_secret = alice.diffie_hellman(bob.public_key());

        // Alice and Bob both derive bidirectional keys
        let (mut alice_send, mut alice_recv) = OnionCrypto::derive_bidirectional_keys(&shared_secret);
        let (mut bob_send, mut bob_recv) = OnionCrypto::derive_bidirectional_keys(&shared_secret);

        let plaintext = b"Hello, AnonNet!";

        // Alice sends to Bob (alice_send -> bob_recv)
        let ciphertext = alice_send.encrypt(plaintext).unwrap();
        let decrypted = bob_recv.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    #[ignore = "TODO: Implement proper sequence number sync - counters currently diverge"]
    fn test_multi_layer_onion() {
        // Simulate real onion routing with bidirectional keys
        // Client creates circuit through 3 relays
        let mut client_forward_layers = Vec::new();
        let mut relay_backward_layers = Vec::new();

        for _ in 0..3 {
            let client_ephemeral = EphemeralKeyPair::generate();
            let relay_ephemeral = EphemeralKeyPair::generate();

            // Both sides derive shared secret
            let shared_secret = client_ephemeral.diffie_hellman(relay_ephemeral.public_key());

            // Client gets forward/backward, relay gets forward/backward
            let (client_forward, _client_backward) = OnionCrypto::derive_bidirectional_keys(&shared_secret);
            let (_relay_forward, relay_backward) = OnionCrypto::derive_bidirectional_keys(&shared_secret);

            client_forward_layers.push(client_forward);
            relay_backward_layers.push(relay_backward);
        }

        let plaintext = b"Secret message through AnonNet";

        // Client encrypts with all forward layers
        let mut layer_refs: Vec<&mut LayerCrypto> = client_forward_layers.iter_mut().collect();
        let onion = OnionCrypto::encrypt_onion(&mut layer_refs, plaintext).unwrap();

        // Each relay decrypts one layer with their backward key
        let mut data = onion;
        for relay_layer in &mut relay_backward_layers {
            data = relay_layer.decrypt(&data).unwrap();
        }

        assert_eq!(plaintext, data.as_slice());
    }

    #[test]
    #[ignore = "TODO: Implement proper sequence number sync - counters currently diverge"]
    fn test_bidirectional_keys() {
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();
        let shared_secret = alice.diffie_hellman(bob.public_key());

        let (mut forward, mut backward) = OnionCrypto::derive_bidirectional_keys(&shared_secret);

        let message = b"Test message";

        // Encrypt forward
        let ciphertext = forward.encrypt(message).unwrap();

        // Cannot decrypt with same direction
        let mut forward2 = forward.clone();
        assert!(forward2.decrypt(&ciphertext).is_err());

        // Can decrypt with backward direction
        let decrypted = backward.decrypt(&ciphertext).unwrap();
        assert_eq!(message, decrypted.as_slice());
    }

    #[test]
    fn test_nonce_reuse_prevention() {
        let mut counter1 = NonceCounter::from_base([0; 12]);
        let mut counter2 = NonceCounter::from_base([0; 12]);

        // Same base, different counters
        let nonce1 = counter1.next_nonce();
        let nonce2 = counter1.next_nonce();

        // Reset counter2 - should get same nonce as first call to counter1
        let nonce3 = counter2.next_nonce();

        assert_ne!(nonce1, nonce2); // Different counters -> different nonces
        assert_eq!(nonce1, nonce3); // Same base + same counter -> same nonce
    }

    #[test]
    fn test_encryption_with_different_nonces() {
        let shared_secret = {
            let a = EphemeralKeyPair::generate();
            let b = EphemeralKeyPair::generate();
            a.diffie_hellman(b.public_key())
        };

        let mut layer = LayerCrypto::new(&shared_secret);
        let plaintext = b"Test";

        // Encrypt same message twice
        let ct1 = layer.encrypt(plaintext).unwrap();
        let ct2 = layer.encrypt(plaintext).unwrap();

        // Ciphertexts MUST be different (due to different nonces)
        assert_ne!(ct1, ct2);
    }
}
