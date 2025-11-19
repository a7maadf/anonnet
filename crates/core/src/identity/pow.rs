/// Proof of Work for identity generation
///
/// This prevents Sybil attacks and DDoS by requiring computational effort
/// to create new identities. Credits are awarded based on PoW difficulty.
use serde::{Deserialize, Serialize};

/// Proof of Work evidence for an identity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofOfWork {
    /// Nonce found during PoW mining
    pub nonce: u64,

    /// Difficulty (number of leading zero bits required)
    pub difficulty: u8,

    /// Timestamp when PoW was computed
    pub timestamp: u64,
}

impl ProofOfWork {
    /// Create a new PoW by mining
    ///
    /// Finds a nonce such that hash(public_key || nonce) has `difficulty` leading zero bits
    pub fn mine(public_key: &[u8; 32], difficulty: u8) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut nonce = 0u64;

        loop {
            if Self::verify_nonce(public_key, nonce, difficulty, timestamp) {
                return Self {
                    nonce,
                    difficulty,
                    timestamp,
                };
            }

            nonce += 1;

            // Prevent infinite loops in tests
            if nonce > u64::MAX - 1000 {
                panic!("PoW mining failed - difficulty too high");
            }
        }
    }

    /// Verify that a nonce produces the required difficulty
    fn verify_nonce(public_key: &[u8; 32], nonce: u64, difficulty: u8, timestamp: u64) -> bool {
        let hash = Self::hash_with_nonce(public_key, nonce, timestamp);
        Self::count_leading_zero_bits(&hash) >= difficulty
    }

    /// Hash public key with nonce
    fn hash_with_nonce(public_key: &[u8; 32], nonce: u64, timestamp: u64) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(public_key);
        hasher.update(&nonce.to_le_bytes());
        hasher.update(&timestamp.to_le_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Count leading zero bits in a hash
    fn count_leading_zero_bits(hash: &[u8; 32]) -> u8 {
        let mut count = 0u16; // Use u16 to prevent overflow, then cap to u8

        for &byte in hash {
            if byte == 0 {
                count += 8;
            } else {
                count += byte.leading_zeros() as u16;
                break;
            }
        }

        // Cap at 255 (max u8 value)
        count.min(255) as u8
    }

    /// Verify this PoW is valid for the given public key
    pub fn verify(&self, public_key: &[u8; 32]) -> bool {
        Self::verify_nonce(public_key, self.nonce, self.difficulty, self.timestamp)
    }

    /// Calculate initial credits based on PoW difficulty
    ///
    /// Credits scale exponentially with difficulty to reward harder work:
    /// - difficulty 8:  1,000 credits (easy - ~256 hashes)
    /// - difficulty 12: 2,000 credits (~4,096 hashes)
    /// - difficulty 16: 4,000 credits (~65,536 hashes)
    /// - difficulty 20: 8,000 credits (~1,048,576 hashes)
    /// - difficulty 24: 16,000 credits (~16,777,216 hashes)
    pub fn calculate_credits(&self) -> u64 {
        if self.difficulty < 8 {
            // Minimum difficulty 8
            return 100;
        }

        // Exponential scaling: 1000 * 2^((difficulty - 8) / 4)
        // This doubles credits every 4 difficulty levels
        let base_credits = 1000u64;
        let difficulty_factor = (self.difficulty - 8) / 4;

        base_credits * (1u64 << difficulty_factor)
    }

    /// Get recommended difficulty for different node types
    pub fn recommended_difficulty() -> u8 {
        12 // Medium difficulty - good balance (~4096 hashes, 2000 credits)
    }

    /// Minimum difficulty accepted by the network
    pub fn minimum_difficulty() -> u8 {
        8 // Easy difficulty for low-end devices
    }

    /// Maximum reasonable difficulty
    pub fn maximum_difficulty() -> u8 {
        28 // Very hard - for powerful nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_mining_easy() {
        let public_key = [0u8; 32];
        let difficulty = 8; // Easy - should find quickly

        let pow = ProofOfWork::mine(&public_key, difficulty);

        assert_eq!(pow.difficulty, difficulty);
        assert!(pow.verify(&public_key));
    }

    #[test]
    fn test_pow_verification() {
        let public_key = [1u8; 32];
        let difficulty = 8;

        let pow = ProofOfWork::mine(&public_key, difficulty);

        // Should verify with correct key
        assert!(pow.verify(&public_key));

        // Should fail with different key
        let wrong_key = [2u8; 32];
        assert!(!pow.verify(&wrong_key));
    }

    #[test]
    fn test_leading_zero_bits() {
        // 32 bytes * 8 bits = 256, but we cap at 255 (max u8)
        assert_eq!(ProofOfWork::count_leading_zero_bits(&[0u8; 32]), 255);
        assert_eq!(
            ProofOfWork::count_leading_zero_bits(&[0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            24
        ); // 3 full bytes + 0 bits from the 128 (10000000)
        assert_eq!(
            ProofOfWork::count_leading_zero_bits(&[0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            17
        ); // 2 full bytes + 1 bit from 64 (01000000)
    }

    #[test]
    fn test_credits_calculation() {
        assert_eq!(ProofOfWork { nonce: 0, difficulty: 8, timestamp: 0 }.calculate_credits(), 1000);
        assert_eq!(ProofOfWork { nonce: 0, difficulty: 12, timestamp: 0 }.calculate_credits(), 2000);
        assert_eq!(ProofOfWork { nonce: 0, difficulty: 16, timestamp: 0 }.calculate_credits(), 4000);
        assert_eq!(ProofOfWork { nonce: 0, difficulty: 20, timestamp: 0 }.calculate_credits(), 8000);
        assert_eq!(ProofOfWork { nonce: 0, difficulty: 24, timestamp: 0 }.calculate_credits(), 16000);

        // Below minimum
        assert_eq!(ProofOfWork { nonce: 0, difficulty: 4, timestamp: 0 }.calculate_credits(), 100);
    }

    #[test]
    fn test_pow_deterministic() {
        let public_key = [42u8; 32];
        let difficulty = 8;

        // Mining the same key twice should produce valid (possibly different) PoWs
        let pow1 = ProofOfWork::mine(&public_key, difficulty);
        let pow2 = ProofOfWork::mine(&public_key, difficulty);

        assert!(pow1.verify(&public_key));
        assert!(pow2.verify(&public_key));
    }

    #[test]
    fn test_pow_serialization() {
        let pow = ProofOfWork {
            nonce: 12345,
            difficulty: 12,
            timestamp: 1234567890,
        };

        let serialized = bincode::serialize(&pow).unwrap();
        let deserialized: ProofOfWork = bincode::deserialize(&serialized).unwrap();

        assert_eq!(pow, deserialized);
    }

    #[test]
    fn test_difficulty_ranges() {
        assert_eq!(ProofOfWork::minimum_difficulty(), 8);
        assert_eq!(ProofOfWork::recommended_difficulty(), 12);
        assert_eq!(ProofOfWork::maximum_difficulty(), 28);
    }
}
