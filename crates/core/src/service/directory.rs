/// Service directory - DHT-based .anon service discovery
///
/// Services publish their descriptors to the DHT, and clients
/// can look them up using the service address.

use crate::dht::RoutingTable;
use crate::identity::NodeId;
use crate::network::ConnectionManager;
use crate::protocol::messages::{
    FindValueMessage, Message, MessagePayload, StoreMessage, Signature64, ValueFoundMessage,
};
use crate::service::{ServiceAddress, ServiceDescriptor};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
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

    /// Optional shared descriptor store path for testing/development
    /// In production, this would be replaced with proper P2P DHT replication
    shared_store_path: Option<PathBuf>,

    /// Connection manager for P2P messaging (set after initialization)
    connection_manager: Arc<RwLock<Option<Arc<ConnectionManager>>>>,
}

impl ServiceDirectory {
    /// Create a new service directory
    pub fn new(local_id: NodeId, routing_table: Arc<RwLock<RoutingTable>>) -> Self {
        Self {
            routing_table,
            descriptor_cache: Arc::new(RwLock::new(HashMap::new())),
            local_id,
            shared_store_path: None,
            connection_manager: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the connection manager (called after initialization)
    ///
    /// Required for P2P descriptor replication over the network
    pub async fn set_connection_manager(&self, connection_manager: Arc<ConnectionManager>) {
        *self.connection_manager.write().await = Some(connection_manager);
    }

    /// Set the shared descriptor store path (for testing/development)
    ///
    /// This enables file-based descriptor replication between nodes.
    /// In production, this would be replaced with proper P2P DHT protocol.
    pub fn set_shared_store_path(&mut self, path: PathBuf) {
        self.shared_store_path = Some(path);
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
        let _key = self.descriptor_key(&descriptor.address);

        // Store in local cache first
        {
            let mut cache = self.descriptor_cache.write().await;
            cache.insert(descriptor.address, descriptor.clone());
        }

        // SHARED STORE REPLICATION (for testing/development)
        // Write descriptor to shared file store so all nodes can discover it
        // This is a fallback for local testing when P2P network isn't available
        if let Some(ref store_path) = self.shared_store_path {
            if let Err(e) = self.write_to_shared_store(&descriptor, store_path).await {
                tracing::warn!("Failed to write descriptor to shared store: {}", e);
                // Don't fail the entire operation if shared store write fails
            } else {
                tracing::info!(
                    "Replicated descriptor {} to shared store",
                    descriptor.address.to_hostname()
                );
            }
        }

        // P2P DHT REPLICATION (production mode)
        // Send STORE messages to K-closest nodes in the network
        if let Some(connection_manager) = self.connection_manager.read().await.clone() {
            let key = self.descriptor_key(&descriptor.address);

            // Get K-closest nodes (excluding ourselves)
            let closest_nodes = {
                let table = self.routing_table.read().await;
                table
                    .closest_nodes(&key, 20)
                    .iter()
                    .filter(|entry| entry.node_id != self.local_id)
                    .map(|entry| entry.node_id)
                    .collect::<Vec<_>>()
            };

            if !closest_nodes.is_empty() {
                tracing::info!(
                    "Replicating descriptor {} to {} nodes via DHT",
                    descriptor.address.to_hostname(),
                    closest_nodes.len()
                );

                // Serialize descriptor for DHT storage
                let descriptor_bytes = match serde_json::to_vec(&descriptor) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        tracing::warn!("Failed to serialize descriptor: {}", e);
                        return Ok(());
                    }
                };

                // Send STORE message to each closest node
                for node_id in closest_nodes {
                    if let Some(handler) = connection_manager.get_connection(&node_id) {
                        let store_msg = Message::new(MessagePayload::Store(StoreMessage {
                            key: *key.as_bytes(),
                            value: descriptor_bytes.clone(),
                            publisher: self.local_id,
                            ttl: descriptor.ttl.as_secs(),
                            signature: None,
                        }));

                        // Send as one-way message (don't wait for response)
                        if let Err(e) = handler.send_message(store_msg).await {
                            tracing::debug!("Failed to send STORE to {}: {}", node_id, e);
                        } else {
                            tracing::debug!("Sent STORE to {} for descriptor {}", node_id, descriptor.address.to_hostname());
                        }
                    }
                }
            } else {
                tracing::info!("No network peers available for DHT replication");
            }
        }

        Ok(())
    }

    /// Write descriptor to shared file store
    async fn write_to_shared_store(
        &self,
        descriptor: &ServiceDescriptor,
        store_path: &PathBuf,
    ) -> Result<()> {
        // Create store directory if it doesn't exist
        tokio::fs::create_dir_all(store_path).await?;

        // Use service address as filename
        let filename = format!("{}.json", descriptor.address.to_hostname());
        let file_path = store_path.join(filename);

        // Serialize descriptor to JSON
        let json = serde_json::to_string_pretty(descriptor)?;

        // Write to file
        tokio::fs::write(file_path, json).await?;

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
        // SHARED STORE LOOKUP (for testing/development)
        // Check shared file store before falling back to DHT query
        // This is a fallback for local testing when P2P network isn't available
        if let Some(ref store_path) = self.shared_store_path {
            if let Ok(descriptor) = self.read_from_shared_store(address, store_path).await {
                // Verify descriptor is valid and not expired
                if descriptor.validate().is_ok() && !descriptor.is_expired() {
                    // Store in local cache for future lookups
                    let mut cache = self.descriptor_cache.write().await;
                    cache.insert(*address, descriptor.clone());

                    tracing::info!(
                        "Found descriptor {} in shared store",
                        address.to_hostname()
                    );
                    return Ok(descriptor);
                }
            }
        }

        // P2P DHT LOOKUP (production mode)
        // Query K-closest nodes for the descriptor
        if let Some(connection_manager) = self.connection_manager.read().await.clone() {
            let key = self.descriptor_key(address);

            // Find closest nodes (excluding ourselves)
            let closest_nodes = {
                let table = self.routing_table.read().await;
                table
                    .closest_nodes(&key, 20)
                    .iter()
                    .filter(|entry| entry.node_id != self.local_id)
                    .map(|entry| entry.node_id)
                    .collect::<Vec<_>>()
            };

            if !closest_nodes.is_empty() {
                tracing::info!(
                    "Querying {} nodes for descriptor {}",
                    closest_nodes.len(),
                    address.to_hostname()
                );

                // Query each node for the descriptor
                for node_id in closest_nodes {
                    if let Some(handler) = connection_manager.get_connection(&node_id) {
                        let find_value_msg = Message::new(MessagePayload::FindValue(FindValueMessage {
                            key: *key.as_bytes(),
                        }));

                        // Send request and wait for response
                        match handler.send_request(find_value_msg).await {
                            Ok(response) => {
                                if let MessagePayload::ValueFound(value_found) = response.payload {
                                    if value_found.found && !value_found.values.is_empty() {
                                        // Deserialize the descriptor
                                        for stored_value in value_found.values {
                                            match serde_json::from_slice::<ServiceDescriptor>(&stored_value.data) {
                                                Ok(descriptor) => {
                                                    // Verify it's valid and not expired
                                                    if descriptor.validate().is_ok() && !descriptor.is_expired() {
                                                        tracing::info!(
                                                            "Found descriptor {} from node {}",
                                                            address.to_hostname(),
                                                            node_id
                                                        );

                                                        // Cache locally
                                                        let mut cache = self.descriptor_cache.write().await;
                                                        cache.insert(*address, descriptor.clone());

                                                        return Ok(descriptor);
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::warn!("Failed to deserialize descriptor from {}: {}", node_id, e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Failed to query {}: {}", node_id, e);
                            }
                        }
                    }
                }
            } else {
                tracing::warn!("No network peers available for DHT lookup");
            }
        }

        // Not found
        Err(DirectoryError::ServiceNotFound)
    }

    /// Read descriptor from shared file store
    async fn read_from_shared_store(
        &self,
        address: &ServiceAddress,
        store_path: &PathBuf,
    ) -> Result<ServiceDescriptor> {
        // Build file path
        let filename = format!("{}.json", address.to_hostname());
        let file_path = store_path.join(filename);

        // Read file
        let json = tokio::fs::read_to_string(file_path).await?;

        // Deserialize descriptor
        let descriptor: ServiceDescriptor = serde_json::from_str(&json)?;

        Ok(descriptor)
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
        let service_address = crate::service::ServiceAddress::from_public_key(&keypair.public_key());
        let intro_point = create_test_intro_point(&service_address);
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
        let service_address = crate::service::ServiceAddress::from_public_key(&keypair.public_key());
        let intro_point = create_test_intro_point(&service_address);

        let descriptor = crate::service::ServiceDescriptor::new(
            keypair.public_key(),
            vec![intro_point],
            Duration::from_secs(3600),
        );

        (descriptor, keypair)
    }

    fn create_test_intro_point(service_address: &crate::service::ServiceAddress) -> IntroductionPoint {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        let mut intro_point = IntroductionPoint::new(
            node_id,
            keypair.public_key(),
            ConnectionInfo {
                addresses: vec!["127.0.0.1".to_string()],
                port: 9001,
                protocol_version: 1,
            },
        );

        // IMPORTANT: Sign the introduction point with the node's keypair
        intro_point.sign(service_address, &keypair);

        intro_point
    }
}

