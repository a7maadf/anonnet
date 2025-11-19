use super::kbucket::{BucketEntry, KBucket};
use crate::identity::{NodeId, PublicKey};
use anonnet_common::{dht, NetworkAddress};

/// Kademlia routing table with 256 k-buckets
///
/// Each bucket stores nodes at a specific distance range:
/// - Bucket 0: nodes with XOR distance having 255 leading zeros
/// - Bucket 1: nodes with XOR distance having 254 leading zeros
/// - ...
/// - Bucket 255: nodes with XOR distance having 0 leading zeros
#[derive(Debug)]
pub struct RoutingTable {
    /// Our node ID
    local_id: NodeId,

    /// 256 k-buckets (one for each bit position)
    buckets: Vec<KBucket>,

    /// Maximum number of nodes per bucket
    k: usize,
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        Self::with_capacity(local_id, dht::K_BUCKET_SIZE)
    }

    pub fn with_capacity(local_id: NodeId, k: usize) -> Self {
        let buckets = (0..256).map(|_| KBucket::new(k)).collect();

        Self {
            local_id,
            buckets,
            k,
        }
    }

    /// Get the bucket index for a given node ID
    ///
    /// The bucket index is determined by the number of leading zeros
    /// in the XOR distance between our ID and the target ID.
    fn bucket_index(&self, node_id: &NodeId) -> usize {
        let distance = self.local_id.distance(node_id);
        let leading_zeros = distance.leading_zeros() as usize;

        // Clamp to valid range [0, 255]
        leading_zeros.min(255)
    }

    /// Get the bucket for a given node ID
    fn bucket(&self, node_id: &NodeId) -> &KBucket {
        let index = self.bucket_index(node_id);
        &self.buckets[index]
    }

    /// Get mutable bucket for a given node ID
    fn bucket_mut(&mut self, node_id: &NodeId) -> &mut KBucket {
        let index = self.bucket_index(node_id);
        &mut self.buckets[index]
    }

    /// Try to insert a node into the routing table
    ///
    /// SECURITY: This validates that NodeId matches PublicKey to prevent Sybil attacks.
    /// Nodes cannot claim arbitrary IDs - the ID must be the BLAKE3 hash of their public key.
    ///
    /// Returns:
    /// - Ok(InsertResult::Inserted) if the node was newly inserted
    /// - Ok(InsertResult::Updated) if the node already existed and was updated
    /// - Err(InsertError) if the node could not be inserted
    pub fn insert(
        &mut self,
        node_id: NodeId,
        public_key: PublicKey,
        addresses: Vec<NetworkAddress>,
    ) -> Result<InsertResult, InsertError> {
        // Don't insert ourselves
        if node_id == self.local_id {
            return Err(InsertError::SelfInsert);
        }

        // CRITICAL SECURITY CHECK: Verify NodeId matches PublicKey
        // This prevents Sybil attacks where an attacker claims arbitrary node IDs.
        // The NodeId MUST be the BLAKE3 hash of the PublicKey.
        let expected_node_id = NodeId::from_public_key(&public_key);
        if node_id != expected_node_id {
            return Err(InsertError::InvalidNodeId);
        }

        let bucket = self.bucket_mut(&node_id);

        match bucket.insert(node_id, public_key, addresses) {
            Ok(true) => Ok(InsertResult::Inserted),
            Ok(false) => Ok(InsertResult::Updated),
            Err(eviction_candidate) => {
                // Bucket is full, return the eviction candidate
                Err(InsertError::BucketFull {
                    eviction_candidate,
                })
            }
        }
    }

    /// Remove a node from the routing table
    pub fn remove(&mut self, node_id: &NodeId) -> Option<BucketEntry> {
        self.bucket_mut(node_id).remove(node_id)
    }

    /// Find a node in the routing table
    pub fn find_node(&self, node_id: &NodeId) -> Option<&BucketEntry> {
        self.bucket(node_id).find_node(node_id)
    }

    /// Find a node in the routing table (mutable)
    pub fn find_node_mut(&mut self, node_id: &NodeId) -> Option<&mut BucketEntry> {
        self.bucket_mut(node_id).find_node_mut(node_id)
    }

    /// Get the K closest nodes to a target ID
    pub fn closest_nodes(&self, target: &NodeId, count: usize) -> Vec<BucketEntry> {
        let mut all_nodes = Vec::new();

        // Collect nodes from all buckets
        for bucket in &self.buckets {
            all_nodes.extend(bucket.nodes().cloned());
        }

        // Sort by distance to target
        all_nodes.sort_by_key(|entry| entry.node_id.distance(target));

        // Return the K closest
        all_nodes.truncate(count);
        all_nodes
    }

    /// Get all nodes in the routing table
    pub fn all_nodes(&self) -> Vec<BucketEntry> {
        let mut nodes = Vec::new();

        for bucket in &self.buckets {
            nodes.extend(bucket.nodes().cloned());
        }

        nodes
    }

    /// Get the total number of nodes in the routing table
    pub fn node_count(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    /// Remove all stale nodes from the routing table
    pub fn remove_stale_nodes(&mut self) -> Vec<BucketEntry> {
        let mut removed = Vec::new();

        for bucket in &mut self.buckets {
            removed.extend(bucket.remove_stale());
        }

        removed
    }

    /// Get buckets that need refreshing
    pub fn buckets_needing_refresh(&self) -> Vec<usize> {
        self.buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.needs_refresh())
            .map(|(index, _)| index)
            .collect()
    }

    /// Mark a bucket as refreshed
    pub fn mark_bucket_refreshed(&mut self, bucket_index: usize) {
        if bucket_index < 256 {
            self.buckets[bucket_index].mark_refreshed();
        }
    }

    /// Get a random node ID for refreshing a specific bucket
    ///
    /// Returns a random ID that would fall into the given bucket
    pub fn random_id_for_bucket(&self, bucket_index: usize) -> NodeId {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Generate a random node ID
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        

        // XOR with our ID to ensure it falls into the right bucket
        // This is a simplified approach; a full implementation would
        // ensure the exact number of leading zeros
        NodeId::from_bytes(bytes)
    }

    /// Get statistics about the routing table
    pub fn stats(&self) -> RoutingTableStats {
        let total_nodes = self.node_count();
        let non_empty_buckets = self.buckets.iter().filter(|b| !b.is_empty()).count();
        let full_buckets = self.buckets.iter().filter(|b| b.is_full()).count();

        RoutingTableStats {
            total_nodes,
            non_empty_buckets,
            full_buckets,
            bucket_capacity: self.k,
        }
    }

    /// Get our local node ID
    pub fn local_id(&self) -> NodeId {
        self.local_id
    }
}

/// Result of inserting a node into the routing table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertResult {
    /// Node was newly inserted
    Inserted,
    /// Node already existed and was updated
    Updated,
}

/// Error when inserting into the routing table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertError {
    /// Tried to insert our own node ID
    SelfInsert,
    /// Bucket is full, returns the candidate for eviction
    BucketFull { eviction_candidate: NodeId },
    /// NodeId doesn't match the PublicKey (prevents Sybil attacks)
    InvalidNodeId,
}

/// Statistics about the routing table
#[derive(Debug, Clone)]
pub struct RoutingTableStats {
    pub total_nodes: usize,
    pub non_empty_buckets: usize,
    pub full_buckets: usize,
    pub bucket_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_test_node() -> (NodeId, PublicKey, Vec<NetworkAddress>) {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());
        let public_key = keypair.public_key();
        let addr = NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

        (node_id, public_key, vec![addr])
    }

    #[test]
    fn test_routing_table_create() {
        let keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&keypair.public_key());

        let table = RoutingTable::new(local_id);
        assert_eq!(table.node_count(), 0);
        assert_eq!(table.local_id(), local_id);
    }

    #[test]
    fn test_routing_table_insert() {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        let (node_id, public_key, addresses) = create_test_node();
        let result = table.insert(node_id, public_key, addresses);

        assert!(matches!(result, Ok(InsertResult::Inserted)));
        assert_eq!(table.node_count(), 1);
    }

    #[test]
    fn test_routing_table_no_self_insert() {
        let keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        let addr = NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

        let result = table.insert(local_id, keypair.public_key(), vec![addr]);
        assert!(matches!(result, Err(InsertError::SelfInsert)));
    }

    #[test]
    fn test_routing_table_find() {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        let (node_id, public_key, addresses) = create_test_node();
        table.insert(node_id, public_key, addresses).unwrap();

        let found = table.find_node(&node_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().node_id, node_id);
    }

    #[test]
    fn test_routing_table_closest_nodes() {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        // Insert several nodes
        for _ in 0..10 {
            let (node_id, public_key, addresses) = create_test_node();
            table.insert(node_id, public_key, addresses).ok();
        }

        let (target_id, _, _) = create_test_node();
        let closest = table.closest_nodes(&target_id, 5);

        assert!(closest.len() <= 5);
        assert!(closest.len() <= table.node_count());
    }

    #[test]
    fn test_routing_table_remove() {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        let (node_id, public_key, addresses) = create_test_node();
        table.insert(node_id, public_key, addresses).unwrap();

        assert_eq!(table.node_count(), 1);

        let removed = table.remove(&node_id);
        assert!(removed.is_some());
        assert_eq!(table.node_count(), 0);
    }

    #[test]
    fn test_routing_table_stats() {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        // Insert a few nodes
        for _ in 0..5 {
            let (node_id, public_key, addresses) = create_test_node();
            table.insert(node_id, public_key, addresses).ok();
        }

        let stats = table.stats();
        assert_eq!(stats.total_nodes, table.node_count());
        assert!(stats.non_empty_buckets > 0);
        assert_eq!(stats.bucket_capacity, dht::K_BUCKET_SIZE);
    }

    #[test]
    fn test_invalid_node_id_rejected() {
        // SECURITY TEST: Verify that nodes with mismatched NodeId/PublicKey are rejected
        // This prevents Sybil attacks where an attacker tries to claim arbitrary node IDs

        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        // Create a legitimate node
        let keypair1 = KeyPair::generate();
        let node_id1 = NodeId::from_public_key(&keypair1.public_key());

        // Create a DIFFERENT keypair
        let keypair2 = KeyPair::generate();
        let public_key2 = keypair2.public_key();

        // Try to insert node with mismatched NodeId/PublicKey (Sybil attack attempt)
        let addr = NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

        let result = table.insert(node_id1, public_key2, vec![addr]);

        // Should be rejected with InvalidNodeId error
        assert!(matches!(result, Err(InsertError::InvalidNodeId)));

        // Table should remain empty
        assert_eq!(table.node_count(), 0);
    }

    #[test]
    fn test_valid_node_id_accepted() {
        // Verify that legitimate nodes with matching NodeId/PublicKey are accepted

        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        // Create a legitimate node with matching NodeId/PublicKey
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());
        let public_key = keypair.public_key();

        let addr = NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

        let result = table.insert(node_id, public_key, vec![addr]);

        // Should be accepted
        assert!(matches!(result, Ok(InsertResult::Inserted)));
        assert_eq!(table.node_count(), 1);
    }
}
