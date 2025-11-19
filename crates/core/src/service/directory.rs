/// Service directory - DHT-based .anon service discovery
///
/// Services publish their descriptors to the DHT, and clients
/// can look them up using the service address.

use crate::dht::RoutingTable;
use crate::identity::NodeId;
use crate::service::{ServiceAddress, ServiceDescriptor};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Service directory for .anon services
///
/// Manages publication and lookup of service descriptors using the DHT.
pub struct ServiceDirectory {
    /// Local routing table for DHT operations
    routing_table: Arc<RwLock<RoutingTable>>,

    /// Local cache of service descriptors
    descriptor_cache: Arc<RwLock<HashMap<ServiceAddress, ServiceDescriptor>>>,

    /// Our node ID
    local_id: NodeId,
}

impl ServiceDirectory {
    /// Create a new service directory
    pub fn new(local_id: NodeId, routing_table: Arc<RwLock<RoutingTable>>) -> Self {
        Self {
            routing_table,
            descriptor_cache: Arc::new(RwLock::new(HashMap::new())),
            local_id,
        }
    }

    /// Publish a service descriptor to the DHT
    ///
    /// The descriptor is stored at nodes closest to the service address
    /// in the DHT keyspace.
    pub async fn publish_descriptor(
        &self,
        descriptor: ServiceDescriptor,
    ) -> Result<(), DirectoryError> {
        // Validate descriptor
        descriptor
            .validate()
            .map_err(|e| DirectoryError::InvalidDescriptor(e.to_string()))?;

        // Get the DHT key for this service
        let key = self.descriptor_key(&descriptor.address);

        // Store in local cache first
        {
            let mut cache = self.descriptor_cache.write().await;
            cache.insert(descriptor.address, descriptor.clone());
        }

        // Find the k-closest nodes to this key for network publishing
        let closest_nodes = {
            let table = self.routing_table.read().await;
            table
                .closest_nodes(&key, 20)
                .iter()
                .map(|entry| entry.node_id)
                .collect::<Vec<_>>()
        };

        if closest_nodes.is_empty() {
            // No network peers, but we stored locally
            return Ok(());
        }

        // TODO: Send STORE_DESCRIPTOR messages to closest nodes
        // This will be implemented when we have the full P2P message protocol

        Ok(())
    }

    /// Look up a service descriptor by address
    ///
    /// First checks local cache, then queries the DHT.
    pub async fn lookup_descriptor(
        &self,
        address: &ServiceAddress,
    ) -> Result<ServiceDescriptor, DirectoryError> {
        // Check local cache first
        {
            let cache = self.descriptor_cache.read().await;
            if let Some(descriptor) = cache.get(address) {
                // Verify it's not expired
                if !descriptor.is_expired() {
                    return Ok(descriptor.clone());
                }
            }
        }

        // Not in cache or expired, query DHT
        self.lookup_from_dht(address).await
    }

    /// Look up a descriptor from the DHT
    async fn lookup_from_dht(
        &self,
        address: &ServiceAddress,
    ) -> Result<ServiceDescriptor, DirectoryError> {
        // Get the DHT key for this service
        let key = self.descriptor_key(address);

        // Find closest nodes
        let closest_nodes = {
            let table = self.routing_table.read().await;
            table
                .closest_nodes(&key, 20)
                .iter()
                .map(|entry| entry.node_id)
                .collect::<Vec<_>>()
        };

        if closest_nodes.is_empty() {
            return Err(DirectoryError::ServiceNotFound);
        }

        // TODO: Send FIND_DESCRIPTOR messages to closest nodes
        // This will be implemented when we have the full P2P message protocol

        // For now, return error
        Err(DirectoryError::ServiceNotFound)
    }

    /// Store a descriptor received from the network
    pub async fn store_descriptor(&self, descriptor: ServiceDescriptor) -> Result<(), DirectoryError> {
        // Validate the descriptor
        descriptor
            .validate()
            .map_err(|e| DirectoryError::InvalidDescriptor(e.to_string()))?;

        // Check if we should store this (are we close to the key?)
        let key = self.descriptor_key(&descriptor.address);
        let should_store = {
            let table = self.routing_table.read().await;
            let closest = table
                .closest_nodes(&key, 20)
                .iter()
                .map(|entry| entry.node_id)
                .collect::<Vec<_>>();

            // Store if we're in the top 20 closest nodes, OR if we're the only node
            closest.is_empty() || closest.iter().any(|id| id == &self.local_id)
        };

        if should_store {
            let mut cache = self.descriptor_cache.write().await;
            cache.insert(descriptor.address, descriptor);
        }

        Ok(())
    }

    /// Get the DHT key for a service address
    fn descriptor_key(&self, address: &ServiceAddress) -> NodeId {
        // Use the service address bytes as the DHT key
        NodeId::from_bytes(*address.as_bytes())
    }

    /// Clean up expired descriptors from cache
    pub async fn cleanup_expired(&self) {
        let mut cache = self.descriptor_cache.write().await;
        cache.retain(|_, descriptor| !descriptor.is_expired());
    }

    /// Get all cached descriptors (for debugging/monitoring)
    pub async fn get_cached_descriptors(&self) -> Vec<ServiceDescriptor> {
        let cache = self.descriptor_cache.read().await;
        cache.values().cloned().collect()
    }
}

/// Service directory errors
#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("Invalid descriptor: {0}")]
    InvalidDescriptor(String),

    #[error("Service not found")]
    ServiceNotFound,

    #[error("No nodes available for storage")]
    NoNodesAvailable,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Timeout waiting for response")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use crate::service::descriptor::{ConnectionInfo, IntroductionPoint};
    use std::time::Duration;

    #[tokio::test]
    async fn test_directory_creation() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(node_id)));

        let directory = ServiceDirectory::new(node_id, routing_table);

        // Should start with empty cache
        assert_eq!(directory.get_cached_descriptors().await.len(), 0);
    }

    #[tokio::test]
    async fn test_publish_descriptor() {
        let node_id_keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&node_id_keypair.public_key());
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(node_id)));

        let directory = ServiceDirectory::new(node_id, routing_table);

        let (mut descriptor, keypair) = create_test_descriptor_with_keypair();
        descriptor.sign(&keypair);

        // Should succeed even without network peers (stored locally)
        directory.publish_descriptor(descriptor.clone()).await.unwrap();

        // Should be in cache
        let cached = directory.get_cached_descriptors().await;
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].address, descriptor.address);
    }

    #[tokio::test]
    async fn test_lookup_from_cache() {
        let node_id_keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&node_id_keypair.public_key());
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(node_id)));

        let directory = ServiceDirectory::new(node_id, routing_table);

        let (mut descriptor, keypair) = create_test_descriptor_with_keypair();
        descriptor.sign(&keypair);

        // Publish descriptor
        directory.publish_descriptor(descriptor.clone()).await.unwrap();

        // Should be able to look it up
        let found = directory.lookup_descriptor(&descriptor.address).await;
        assert!(found.is_ok());
        assert_eq!(found.unwrap().address, descriptor.address);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let node_id_keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&node_id_keypair.public_key());
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(node_id)));

        let directory = ServiceDirectory::new(node_id, routing_table);

        // Create an expired descriptor
        let keypair = KeyPair::generate();
        let intro_point = create_test_intro_point();
        let mut descriptor = crate::service::ServiceDescriptor::new(
            keypair.public_key(),
            vec![intro_point],
            Duration::from_secs(0), // Instant expiry
        );
        descriptor.sign(&keypair);

        // Directly insert into cache (bypass validation since this is for testing expiry)
        {
            let mut cache = directory.descriptor_cache.write().await;
            cache.insert(descriptor.address, descriptor);
        }

        // Should be in cache
        assert_eq!(directory.get_cached_descriptors().await.len(), 1);

        // Cleanup
        directory.cleanup_expired().await;

        // Should be removed
        assert_eq!(directory.get_cached_descriptors().await.len(), 0);
    }

    fn create_test_descriptor() -> crate::service::ServiceDescriptor {
        let (descriptor, _keypair) = create_test_descriptor_with_keypair();
        descriptor
    }

    fn create_test_descriptor_with_keypair() -> (crate::service::ServiceDescriptor, KeyPair) {
        let keypair = KeyPair::generate();
        let intro_point = create_test_intro_point();

        let descriptor = crate::service::ServiceDescriptor::new(
            keypair.public_key(),
            vec![intro_point],
            Duration::from_secs(3600),
        );

        (descriptor, keypair)
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
            auth_signature: crate::service::descriptor::Signature([0u8; 64]),
        }
    }
}

