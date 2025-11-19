use crate::identity::{NodeId, PublicKey};
use anonnet_common::{dht, NetworkAddress, Reputation, Timestamp};
use std::collections::VecDeque;

/// Entry in a k-bucket
#[derive(Debug, Clone)]
pub struct BucketEntry {
    /// Node ID
    pub node_id: NodeId,

    /// Public key
    pub public_key: PublicKey,

    /// Network addresses
    pub addresses: Vec<NetworkAddress>,

    /// Last time we successfully contacted this node
    pub last_seen: Timestamp,

    /// Number of failed contact attempts
    pub failed_attempts: u32,

    /// Is this node currently being pinged?
    pub pending_ping: bool,

    /// Node's reputation score
    pub reputation: Reputation,

    /// Whether this node accepts relay traffic
    pub accepts_relay: bool,
}

impl BucketEntry {
    pub fn new(node_id: NodeId, public_key: PublicKey, addresses: Vec<NetworkAddress>) -> Self {
        Self {
            node_id,
            public_key,
            addresses,
            last_seen: Timestamp::now(),
            failed_attempts: 0,
            pending_ping: false,
            reputation: Reputation::INITIAL,
            accepts_relay: true,
        }
    }

    /// Mark this entry as seen (successful contact)
    pub fn mark_seen(&mut self) {
        self.last_seen = Timestamp::now();
        self.failed_attempts = 0;
        self.pending_ping = false;
    }

    /// Mark a failed contact attempt
    pub fn mark_failed(&mut self) {
        self.failed_attempts += 1;
        self.pending_ping = false;
    }

    /// Check if this node is stale
    pub fn is_stale(&self) -> bool {
        self.last_seen.elapsed().as_secs() > dht::MAX_NODE_AGE_SECS
    }

    /// Check if this node should be considered dead
    pub fn is_dead(&self) -> bool {
        self.failed_attempts >= 3
    }
}

/// A k-bucket storing up to K nodes at a specific distance range
#[derive(Debug)]
pub struct KBucket {
    /// Nodes in this bucket (most recently seen at the back)
    nodes: VecDeque<BucketEntry>,

    /// Maximum size of the bucket
    capacity: usize,

    /// Last time this bucket was refreshed
    last_refreshed: Timestamp,
}

impl KBucket {
    pub fn new(capacity: usize) -> Self {
        Self {
            nodes: VecDeque::with_capacity(capacity),
            capacity,
            last_refreshed: Timestamp::now(),
        }
    }

    /// Get the number of nodes in this bucket
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the bucket is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Check if the bucket is full
    pub fn is_full(&self) -> bool {
        self.nodes.len() >= self.capacity
    }

    /// Get all nodes in this bucket
    pub fn nodes(&self) -> impl Iterator<Item = &BucketEntry> {
        self.nodes.iter()
    }

    /// Get mutable access to all nodes
    pub fn nodes_mut(&mut self) -> impl Iterator<Item = &mut BucketEntry> {
        self.nodes.iter_mut()
    }

    /// Find a node by ID
    pub fn find_node(&self, node_id: &NodeId) -> Option<&BucketEntry> {
        self.nodes.iter().find(|entry| &entry.node_id == node_id)
    }

    /// Find a node by ID (mutable)
    pub fn find_node_mut(&mut self, node_id: &NodeId) -> Option<&mut BucketEntry> {
        self.nodes.iter_mut().find(|entry| &entry.node_id == node_id)
    }

    /// Try to insert a node into the bucket
    ///
    /// Returns:
    /// - Ok(true) if the node was inserted
    /// - Ok(false) if the node was already in the bucket and was updated
    /// - Err(node_id) if the bucket is full and returns the least-recently-seen node
    pub fn insert(
        &mut self,
        node_id: NodeId,
        public_key: PublicKey,
        addresses: Vec<NetworkAddress>,
    ) -> Result<bool, NodeId> {
        // Check if node already exists
        if let Some(entry) = self.find_node_mut(&node_id) {
            // Update existing entry
            entry.addresses = addresses;
            entry.mark_seen();

            // Move to back (most recently seen)
            let index = self
                .nodes
                .iter()
                .position(|e| e.node_id == node_id)
                .unwrap();
            if let Some(entry) = self.nodes.remove(index) {
                self.nodes.push_back(entry);
            }

            return Ok(false);
        }

        // If bucket is full, return the least-recently-seen node
        if self.is_full() {
            // Return the front node (least recently seen) for potential eviction
            return Err(self.nodes.front().unwrap().node_id);
        }

        // Insert new node at the back (most recently seen)
        self.nodes
            .push_back(BucketEntry::new(node_id, public_key, addresses));

        Ok(true)
    }

    /// Remove a node from the bucket
    pub fn remove(&mut self, node_id: &NodeId) -> Option<BucketEntry> {
        if let Some(index) = self.nodes.iter().position(|e| &e.node_id == node_id) {
            self.nodes.remove(index)
        } else {
            None
        }
    }

    /// Remove all stale nodes from the bucket
    pub fn remove_stale(&mut self) -> Vec<BucketEntry> {
        let mut removed = Vec::new();

        self.nodes.retain(|entry| {
            if entry.is_stale() || entry.is_dead() {
                removed.push(entry.clone());
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get the least-recently-seen node
    pub fn least_recent(&self) -> Option<&BucketEntry> {
        self.nodes.front()
    }

    /// Get the most-recently-seen node
    pub fn most_recent(&self) -> Option<&BucketEntry> {
        self.nodes.back()
    }

    /// Check if this bucket needs refreshing
    pub fn needs_refresh(&self) -> bool {
        self.last_refreshed.elapsed().as_secs() > dht::REFRESH_INTERVAL_SECS
    }

    /// Mark this bucket as refreshed
    pub fn mark_refreshed(&mut self) {
        self.last_refreshed = Timestamp::now();
    }

    /// Get the K closest nodes to a target
    pub fn closest_nodes(&self, target: &NodeId, count: usize) -> Vec<BucketEntry> {
        let mut nodes: Vec<_> = self.nodes.iter().cloned().collect();

        // Sort by distance to target
        nodes.sort_by_key(|entry| entry.node_id.distance(target));

        nodes.truncate(count);
        nodes
    }
}

impl Default for KBucket {
    fn default() -> Self {
        Self::new(dht::K_BUCKET_SIZE)
    }
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
    fn test_kbucket_insert() {
        let mut bucket = KBucket::new(3);
        let (node_id, public_key, addresses) = create_test_node();

        let result = bucket.insert(node_id, public_key, addresses.clone());
        assert!(result.is_ok());
        assert_eq!(bucket.len(), 1);
    }

    #[test]
    fn test_kbucket_full() {
        let mut bucket = KBucket::new(2);

        for _ in 0..2 {
            let (node_id, public_key, addresses) = create_test_node();
            bucket.insert(node_id, public_key, addresses).unwrap();
        }

        assert!(bucket.is_full());

        // Try to insert one more
        let (node_id, public_key, addresses) = create_test_node();
        let result = bucket.insert(node_id, public_key, addresses);
        assert!(result.is_err());
    }

    #[test]
    fn test_kbucket_update_existing() {
        let mut bucket = KBucket::new(3);
        let (node_id, public_key, addresses) = create_test_node();

        bucket
            .insert(node_id, public_key, addresses.clone())
            .unwrap();
        let result = bucket.insert(node_id, public_key, addresses);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // Not a new insertion
        assert_eq!(bucket.len(), 1);
    }

    #[test]
    fn test_kbucket_remove() {
        let mut bucket = KBucket::new(3);
        let (node_id, public_key, addresses) = create_test_node();

        bucket.insert(node_id, public_key, addresses).unwrap();
        assert_eq!(bucket.len(), 1);

        let removed = bucket.remove(&node_id);
        assert!(removed.is_some());
        assert_eq!(bucket.len(), 0);
    }

    #[test]
    fn test_kbucket_lru_order() {
        let mut bucket = KBucket::new(3);
        let nodes: Vec<_> = (0..3).map(|_| create_test_node()).collect();

        // Insert all nodes
        for (node_id, public_key, addresses) in &nodes {
            bucket
                .insert(*node_id, *public_key, addresses.clone())
                .unwrap();
        }

        // First node should be least recent
        assert_eq!(bucket.least_recent().unwrap().node_id, nodes[0].0);

        // Last node should be most recent
        assert_eq!(bucket.most_recent().unwrap().node_id, nodes[2].0);
    }
}
