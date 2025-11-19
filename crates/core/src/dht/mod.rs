mod kbucket;
mod lookup;
mod routing_table;
mod storage;

pub use kbucket::{BucketEntry, KBucket};
pub use lookup::{LookupManager, NodeLookup};
pub use routing_table::{InsertError, InsertResult, RoutingTable, RoutingTableStats};
pub use storage::{DHTStorage, StorageError, StorageStats, StoredValue};

use crate::identity::{NodeId, PublicKey};
use anonnet_common::{dht, AnonNetError, NetworkAddress, Result};

/// DHT (Distributed Hash Table) for peer discovery
///
/// Implements a Kademlia-based DHT for decentralized peer discovery
#[derive(Debug)]
pub struct DHT {
    /// Our node's identity
    local_id: NodeId,

    /// Routing table with k-buckets
    routing_table: RoutingTable,

    /// Active node lookups
    lookup_manager: LookupManager,

    /// Bootstrap nodes
    bootstrap_nodes: Vec<BootstrapNode>,

    /// Whether we've successfully bootstrapped
    bootstrapped: bool,

    /// DHT storage for key-value pairs
    storage: DHTStorage,
}

impl DHT {
    /// Create a new DHT instance
    pub fn new(local_id: NodeId, bootstrap_nodes: Vec<BootstrapNode>) -> Self {
        Self {
            local_id,
            routing_table: RoutingTable::new(local_id),
            lookup_manager: LookupManager::new(),
            bootstrap_nodes,
            bootstrapped: false,
            storage: DHTStorage::new(10000), // Store up to 10k keys
        }
    }

    /// Get our local node ID
    pub fn local_id(&self) -> NodeId {
        self.local_id
    }

    /// Check if we've bootstrapped successfully
    pub fn is_bootstrapped(&self) -> bool {
        self.bootstrapped
    }

    /// Add a node to the routing table
    pub fn add_node(
        &mut self,
        node_id: NodeId,
        public_key: PublicKey,
        addresses: Vec<NetworkAddress>,
    ) -> Result<InsertResult> {
        match self.routing_table.insert(node_id, public_key, addresses) {
            Ok(result) => Ok(result),
            Err(InsertError::SelfInsert) => {
                Err(AnonNetError::invalid_node_id("Cannot insert self"))
            }
            Err(InsertError::InvalidNodeId) => {
                // SECURITY: Reject nodes where NodeId doesn't match PublicKey
                // This prevents Sybil attacks
                Err(AnonNetError::invalid_node_id(
                    "NodeId doesn't match PublicKey (possible Sybil attack)",
                ))
            }
            Err(InsertError::BucketFull { eviction_candidate }) => {
                // In a real implementation, we would ping the eviction candidate
                // and potentially replace it if it doesn't respond
                Err(AnonNetError::internal(format!(
                    "Bucket full, candidate for eviction: {}",
                    eviction_candidate
                )))
            }
        }
    }

    /// Remove a node from the routing table
    pub fn remove_node(&mut self, node_id: &NodeId) -> Option<BucketEntry> {
        self.routing_table.remove(node_id)
    }

    /// Find a node in the routing table
    pub fn find_node(&self, node_id: &NodeId) -> Option<&BucketEntry> {
        self.routing_table.find_node(node_id)
    }

    /// Get the K closest nodes to a target
    pub fn closest_nodes(&self, target: &NodeId, count: usize) -> Vec<BucketEntry> {
        self.routing_table.closest_nodes(target, count)
    }

    /// Start an iterative node lookup
    ///
    /// Returns the lookup ID (target node ID)
    pub fn start_lookup(&mut self, target: NodeId) -> NodeId {
        let initial_nodes = self
            .routing_table
            .closest_nodes(&target, dht::K_BUCKET_SIZE)
            .into_iter()
            .map(|entry| entry.node_id)
            .collect();

        self.lookup_manager.start_lookup(target, initial_nodes);
        target
    }

    /// Get an active lookup
    pub fn get_lookup(&self, target: &NodeId) -> Option<&NodeLookup> {
        self.lookup_manager.get_lookup(target)
    }

    /// Get a mutable lookup
    pub fn get_lookup_mut(&mut self, target: &NodeId) -> Option<&mut NodeLookup> {
        self.lookup_manager.get_lookup_mut(target)
    }

    /// Get nodes to query for a lookup
    pub fn next_lookup_queries(&mut self, target: &NodeId) -> Option<Vec<NodeId>> {
        self.lookup_manager
            .get_lookup_mut(target)
            .map(|lookup| lookup.next_queries())
    }

    /// Process a FindNode response
    pub fn process_find_node_response(
        &mut self,
        target: &NodeId,
        from_node: NodeId,
        discovered_nodes: Vec<(NodeId, PublicKey, Vec<NetworkAddress>)>,
    ) {
        // Add discovered nodes to routing table
        for (node_id, public_key, addresses) in &discovered_nodes {
            self.add_node(*node_id, *public_key, addresses.clone()).ok();
        }

        // Update the lookup
        if let Some(lookup) = self.lookup_manager.get_lookup_mut(target) {
            let node_ids = discovered_nodes
                .into_iter()
                .map(|(id, _, _)| id)
                .collect();
            lookup.process_response(from_node, node_ids);
        }
    }

    /// Mark a lookup query as failed
    pub fn mark_lookup_failed(&mut self, target: &NodeId, node_id: NodeId) {
        if let Some(lookup) = self.lookup_manager.get_lookup_mut(target) {
            lookup.mark_failed(node_id);
        }
    }

    /// Get the result of a completed lookup
    pub fn finish_lookup(&mut self, target: &NodeId) -> Option<Vec<NodeId>> {
        if let Some(lookup) = self.lookup_manager.remove_lookup(target) {
            if lookup.is_complete() {
                return Some(lookup.closest_nodes().to_vec());
            }
        }
        None
    }

    /// Bootstrap the DHT by connecting to bootstrap nodes
    pub fn bootstrap(&mut self) -> BootstrapState {
        if self.bootstrap_nodes.is_empty() {
            return BootstrapState::NoBootstrapNodes;
        }

        // In a real implementation, we would:
        // 1. Connect to each bootstrap node
        // 2. Add them to the routing table
        // 3. Perform a lookup for our own ID to populate the routing table
        // 4. Verify we have enough nodes

        // For now, just mark that we need to perform the bootstrap
        BootstrapState::Pending {
            nodes_to_try: self.bootstrap_nodes.clone(),
        }
    }

    /// Mark bootstrap as complete
    pub fn mark_bootstrapped(&mut self) {
        self.bootstrapped = true;
    }

    /// Perform periodic maintenance
    pub fn maintenance(&mut self) -> MaintenanceActions {
        let mut actions = MaintenanceActions::default();

        // Remove stale nodes
        let removed = self.routing_table.remove_stale_nodes();
        actions.removed_stale = removed.len();

        // Find buckets that need refreshing
        let buckets = self.routing_table.buckets_needing_refresh();
        for bucket_index in buckets {
            // Generate a random ID in this bucket's range
            let random_id = self.routing_table.random_id_for_bucket(bucket_index);
            actions.buckets_to_refresh.push((bucket_index, random_id));
        }

        // Cleanup completed lookups
        let completed = self.lookup_manager.cleanup_completed();
        actions.completed_lookups = completed.len();

        actions
    }

    /// Store a value in the DHT
    pub fn store(&mut self, key: [u8; 32], value: StoredValue) -> std::result::Result<(), StorageError> {
        self.storage.store(key, value)
    }

    /// Find a value in the DHT storage
    pub fn find_value(&self, key: &[u8; 32]) -> Option<Vec<StoredValue>> {
        self.storage.get(key)
    }

    /// Get storage statistics
    pub fn storage_stats(&self) -> StorageStats {
        self.storage.stats()
    }

    /// Get routing table statistics
    pub fn stats(&self) -> DHTStats {
        let rt_stats = self.routing_table.stats();

        DHTStats {
            total_nodes: rt_stats.total_nodes,
            non_empty_buckets: rt_stats.non_empty_buckets,
            full_buckets: rt_stats.full_buckets,
            active_lookups: self.lookup_manager.active_lookups().count(),
            bootstrapped: self.bootstrapped,
        }
    }

    /// Get the routing table
    pub fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }
}

/// Bootstrap node information
#[derive(Debug, Clone)]
pub struct BootstrapNode {
    pub address: NetworkAddress,
    pub node_id: Option<NodeId>,
    pub public_key: Option<PublicKey>,
}

impl BootstrapNode {
    pub fn new(address: NetworkAddress) -> Self {
        Self {
            address,
            node_id: None,
            public_key: None,
        }
    }

    pub fn with_identity(
        address: NetworkAddress,
        node_id: NodeId,
        public_key: PublicKey,
    ) -> Self {
        Self {
            address,
            node_id: Some(node_id),
            public_key: Some(public_key),
        }
    }
}

/// State of the bootstrap process
#[derive(Debug)]
pub enum BootstrapState {
    /// No bootstrap nodes configured
    NoBootstrapNodes,

    /// Bootstrap is pending
    Pending { nodes_to_try: Vec<BootstrapNode> },

    /// Bootstrap is in progress
    InProgress { connected: usize, failed: usize },

    /// Bootstrap completed successfully
    Complete,

    /// Bootstrap failed
    Failed { reason: String },
}

/// Actions to perform from maintenance
#[derive(Debug, Default)]
pub struct MaintenanceActions {
    /// Number of stale nodes removed
    pub removed_stale: usize,

    /// Buckets that need refreshing (bucket_index, random_target_id)
    pub buckets_to_refresh: Vec<(usize, NodeId)>,

    /// Number of completed lookups cleaned up
    pub completed_lookups: usize,
}

/// DHT statistics
#[derive(Debug)]
pub struct DHTStats {
    pub total_nodes: usize,
    pub non_empty_buckets: usize,
    pub full_buckets: usize,
    pub active_lookups: usize,
    pub bootstrapped: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_test_identity() -> (NodeId, PublicKey) {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());
        let public_key = keypair.public_key();
        (node_id, public_key)
    }

    #[test]
    fn test_dht_create() {
        let (local_id, _) = create_test_identity();
        let dht = DHT::new(local_id, vec![]);

        assert_eq!(dht.local_id(), local_id);
        assert!(!dht.is_bootstrapped());
    }

    #[test]
    fn test_dht_add_node() {
        let (local_id, _) = create_test_identity();
        let mut dht = DHT::new(local_id, vec![]);

        let (node_id, public_key) = create_test_identity();
        let addr = NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

        let result = dht.add_node(node_id, public_key, vec![addr]);
        assert!(result.is_ok());

        let stats = dht.stats();
        assert_eq!(stats.total_nodes, 1);
    }

    #[test]
    fn test_dht_start_lookup() {
        let (local_id, _) = create_test_identity();
        let mut dht = DHT::new(local_id, vec![]);

        // Add some nodes first
        for _ in 0..5 {
            let (node_id, public_key) = create_test_identity();
            let addr = NetworkAddress::from_socket(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                8080,
            ));
            dht.add_node(node_id, public_key, vec![addr]).ok();
        }

        let (target, _) = create_test_identity();
        let lookup_id = dht.start_lookup(target);

        assert_eq!(lookup_id, target);
        assert!(dht.get_lookup(&target).is_some());
    }

    #[test]
    fn test_dht_bootstrap() {
        let (local_id, _) = create_test_identity();
        let bootstrap_addr = NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            9090,
        ));

        let bootstrap_nodes = vec![BootstrapNode::new(bootstrap_addr)];
        let mut dht = DHT::new(local_id, bootstrap_nodes);

        let state = dht.bootstrap();
        assert!(matches!(state, BootstrapState::Pending { .. }));
    }

    #[test]
    fn test_dht_maintenance() {
        let (local_id, _) = create_test_identity();
        let mut dht = DHT::new(local_id, vec![]);

        let actions = dht.maintenance();
        assert_eq!(actions.removed_stale, 0);
    }
}
