/// DHT storage for key-value pairs
///
/// Implements storage for the DHT, primarily for service descriptors
/// and other distributed data

use crate::identity::NodeId;
use anonnet_common::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Maximum number of values per key
const MAX_VALUES_PER_KEY: usize = 20;

/// Default value TTL (24 hours)
const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Stored value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredValue {
    /// The stored data
    pub data: Vec<u8>,

    /// Publisher's node ID
    pub publisher: NodeId,

    /// When the value was stored
    pub stored_at: Timestamp,

    /// Time-to-live for this value
    pub ttl: Duration,

    /// Optional signature for verification
    pub signature: Option<Vec<u8>>,
}

impl StoredValue {
    /// Create a new stored value
    pub fn new(data: Vec<u8>, publisher: NodeId) -> Self {
        Self {
            data,
            publisher,
            stored_at: Timestamp::now(),
            ttl: DEFAULT_TTL,
            signature: None,
        }
    }

    /// Create with custom TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Create with signature
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Check if value has expired
    pub fn is_expired(&self) -> bool {
        let now = Timestamp::now();
        let age_secs = now.as_secs() - self.stored_at.as_secs();
        age_secs > self.ttl.as_secs()
    }
}

/// DHT storage implementation
#[derive(Debug)]
pub struct DHTStorage {
    /// Key-value store
    /// Key is typically a hash of the content or service address
    storage: HashMap<[u8; 32], Vec<StoredValue>>,

    /// Maximum number of keys to store
    max_keys: usize,
}

impl DHTStorage {
    /// Create a new DHT storage
    pub fn new(max_keys: usize) -> Self {
        Self {
            storage: HashMap::new(),
            max_keys,
        }
    }

    /// Store a value
    pub fn store(&mut self, key: [u8; 32], value: StoredValue) -> Result<(), StorageError> {
        // Check if we're at capacity for new keys
        if !self.storage.contains_key(&key) && self.storage.len() >= self.max_keys {
            return Err(StorageError::StorageFull);
        }

        let values = self.storage.entry(key).or_insert_with(Vec::new);

        // Check if we already have this exact value from this publisher
        if let Some(existing) = values
            .iter_mut()
            .find(|v| v.publisher == value.publisher)
        {
            // Update existing value
            *existing = value;
        } else {
            // Add new value
            if values.len() >= MAX_VALUES_PER_KEY {
                // Remove oldest value
                if let Some((idx, _)) = values
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.stored_at.as_secs())
                {
                    values.remove(idx);
                }
            }
            values.push(value);
        }

        Ok(())
    }

    /// Retrieve values for a key
    pub fn get(&self, key: &[u8; 32]) -> Option<Vec<StoredValue>> {
        self.storage
            .get(key)
            .map(|values| {
                // Filter out expired values
                values
                    .iter()
                    .filter(|v| !v.is_expired())
                    .cloned()
                    .collect()
            })
            .filter(|v: &Vec<StoredValue>| !v.is_empty())
    }

    /// Remove expired values
    pub fn cleanup_expired(&mut self) -> usize {
        let mut removed_count = 0;

        // Remove expired values
        for values in self.storage.values_mut() {
            let before = values.len();
            values.retain(|v| !v.is_expired());
            removed_count += before - values.len();
        }

        // Remove empty keys
        self.storage.retain(|_, v| !v.is_empty());

        removed_count
    }

    /// Get total number of stored keys
    pub fn key_count(&self) -> usize {
        self.storage.len()
    }

    /// Get total number of stored values
    pub fn value_count(&self) -> usize {
        self.storage.values().map(|v| v.len()).sum()
    }

    /// Get storage statistics
    pub fn stats(&self) -> StorageStats {
        StorageStats {
            total_keys: self.key_count(),
            total_values: self.value_count(),
            capacity: self.max_keys,
        }
    }

    /// Remove a specific key
    pub fn remove(&mut self, key: &[u8; 32]) -> Option<Vec<StoredValue>> {
        self.storage.remove(key)
    }
}

/// Storage error types
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage is full")]
    StorageFull,

    #[error("Value too large")]
    ValueTooLarge,

    #[error("Invalid signature")]
    InvalidSignature,
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_keys: usize,
    pub total_values: usize,
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{KeyPair, NodeId};

    #[test]
    fn test_store_and_get() {
        let mut storage = DHTStorage::new(100);

        let keypair = KeyPair::generate();
        let publisher = NodeId::from_public_key(&keypair.public_key());

        let key = [1u8; 32];
        let value = StoredValue::new(b"test data".to_vec(), publisher);

        storage.store(key, value).unwrap();

        let retrieved = storage.get(&key).unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].data, b"test data");
    }

    #[test]
    fn test_update_existing_value() {
        let mut storage = DHTStorage::new(100);

        let keypair = KeyPair::generate();
        let publisher = NodeId::from_public_key(&keypair.public_key());

        let key = [1u8; 32];

        // Store initial value
        let value1 = StoredValue::new(b"data 1".to_vec(), publisher);
        storage.store(key, value1).unwrap();

        // Update with new value from same publisher
        let value2 = StoredValue::new(b"data 2".to_vec(), publisher);
        storage.store(key, value2).unwrap();

        let retrieved = storage.get(&key).unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].data, b"data 2");
    }

    #[test]
    fn test_multiple_publishers() {
        let mut storage = DHTStorage::new(100);

        let kp1 = KeyPair::generate();
        let pub1 = NodeId::from_public_key(&kp1.public_key());

        let kp2 = KeyPair::generate();
        let pub2 = NodeId::from_public_key(&kp2.public_key());

        let key = [1u8; 32];

        storage.store(key, StoredValue::new(b"data 1".to_vec(), pub1)).unwrap();
        storage.store(key, StoredValue::new(b"data 2".to_vec(), pub2)).unwrap();

        let retrieved = storage.get(&key).unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_storage_full() {
        let mut storage = DHTStorage::new(2);

        let keypair = KeyPair::generate();
        let publisher = NodeId::from_public_key(&keypair.public_key());

        // Fill storage
        storage.store([1u8; 32], StoredValue::new(b"data 1".to_vec(), publisher)).unwrap();
        storage.store([2u8; 32], StoredValue::new(b"data 2".to_vec(), publisher)).unwrap();

        // Try to store a third key
        let result = storage.store([3u8; 32], StoredValue::new(b"data 3".to_vec(), publisher));
        assert!(matches!(result, Err(StorageError::StorageFull)));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut storage = DHTStorage::new(100);

        let keypair = KeyPair::generate();
        let publisher = NodeId::from_public_key(&keypair.public_key());

        let key = [1u8; 32];

        // Store value with very short TTL
        let value = StoredValue::new(b"test".to_vec(), publisher)
            .with_ttl(Duration::from_secs(0));

        storage.store(key, value).unwrap();

        // Should be expired immediately
        let removed = storage.cleanup_expired();
        assert_eq!(removed, 1);
        assert_eq!(storage.key_count(), 0);
    }
}
