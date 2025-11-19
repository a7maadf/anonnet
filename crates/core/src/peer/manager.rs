use crate::identity::{NodeId, PublicKey};
use anonnet_common::{NetworkAddress, Reputation, Timestamp};
use std::collections::HashMap;

/// Connection state for a peer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    /// Attempting to connect
    Connecting,

    /// Successfully connected
    Connected,

    /// Connection failed
    Failed,

    /// Disconnected
    Disconnected,
}

/// Information about a connected peer
#[derive(Debug, Clone)]
pub struct PeerConnection {
    /// Peer's node ID
    pub node_id: NodeId,

    /// Peer's public key
    pub public_key: PublicKey,

    /// Peer's addresses
    pub addresses: Vec<NetworkAddress>,

    /// Current connection state
    pub state: PeerState,

    /// When the connection was established
    pub connected_at: Option<Timestamp>,

    /// Last time we received data from this peer
    pub last_activity: Timestamp,

    /// Peer's reputation
    pub reputation: Reputation,

    /// Number of failed connection attempts
    pub failed_attempts: u32,

    /// Whether this peer accepts relay traffic
    pub accepts_relay: bool,

    /// Bytes sent to this peer
    pub bytes_sent: u64,

    /// Bytes received from this peer
    pub bytes_received: u64,
}

impl PeerConnection {
    pub fn new(node_id: NodeId, public_key: PublicKey, addresses: Vec<NetworkAddress>) -> Self {
        Self {
            node_id,
            public_key,
            addresses,
            state: PeerState::Connecting,
            connected_at: None,
            last_activity: Timestamp::now(),
            reputation: Reputation::INITIAL,
            failed_attempts: 0,
            accepts_relay: false,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Mark the connection as established
    pub fn mark_connected(&mut self) {
        self.state = PeerState::Connected;
        self.connected_at = Some(Timestamp::now());
        self.failed_attempts = 0;
        self.last_activity = Timestamp::now();
    }

    /// Mark the connection as failed
    pub fn mark_failed(&mut self) {
        self.state = PeerState::Failed;
        self.failed_attempts += 1;
    }

    /// Mark the connection as disconnected
    pub fn mark_disconnected(&mut self) {
        self.state = PeerState::Disconnected;
        self.connected_at = None;
    }

    /// Update last activity time
    pub fn update_activity(&mut self) {
        self.last_activity = Timestamp::now();
    }

    /// Check if this peer is idle (no activity for a while)
    pub fn is_idle(&self, timeout_secs: u64) -> bool {
        self.last_activity.elapsed().as_secs() > timeout_secs
    }

    /// Record bytes sent
    pub fn add_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.update_activity();
    }

    /// Record bytes received
    pub fn add_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.update_activity();
    }
}

/// Manager for peer connections
#[derive(Debug)]
pub struct PeerManager {
    /// Connected peers
    peers: HashMap<NodeId, PeerConnection>,

    /// Maximum number of peers to maintain
    max_peers: usize,

    /// Minimum number of peers to maintain
    min_peers: usize,
}

impl PeerManager {
    pub fn new(max_peers: usize) -> Self {
        Self {
            peers: HashMap::new(),
            max_peers,
            min_peers: max_peers / 2,
        }
    }

    /// Get the number of connected peers
    pub fn peer_count(&self) -> usize {
        self.peers
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .count()
    }

    /// Get the total number of peers (all states)
    pub fn total_peers(&self) -> usize {
        self.peers.len()
    }

    /// Check if we have room for more peers
    pub fn has_capacity(&self) -> bool {
        self.total_peers() < self.max_peers
    }

    /// Check if we need more peers
    pub fn needs_more_peers(&self) -> bool {
        self.peer_count() < self.min_peers
    }

    /// Add a peer
    pub fn add_peer(
        &mut self,
        node_id: NodeId,
        public_key: PublicKey,
        addresses: Vec<NetworkAddress>,
    ) -> bool {
        if self.peers.contains_key(&node_id) {
            return false; // Already exists
        }

        if !self.has_capacity() {
            return false; // No room
        }

        let peer = PeerConnection::new(node_id, public_key, addresses);
        self.peers.insert(node_id, peer);
        true
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, node_id: &NodeId) -> Option<PeerConnection> {
        self.peers.remove(node_id)
    }

    /// Get a peer by ID
    pub fn get_peer(&self, node_id: &NodeId) -> Option<&PeerConnection> {
        self.peers.get(node_id)
    }

    /// Get a mutable peer by ID
    pub fn get_peer_mut(&mut self, node_id: &NodeId) -> Option<&mut PeerConnection> {
        self.peers.get_mut(node_id)
    }

    /// Mark a peer as connected
    pub fn mark_connected(&mut self, node_id: &NodeId) -> bool {
        if let Some(peer) = self.peers.get_mut(node_id) {
            peer.mark_connected();
            true
        } else {
            false
        }
    }

    /// Mark a peer as failed
    pub fn mark_failed(&mut self, node_id: &NodeId) -> bool {
        if let Some(peer) = self.peers.get_mut(node_id) {
            peer.mark_failed();
            true
        } else {
            false
        }
    }

    /// Get all connected peers
    pub fn connected_peers(&self) -> Vec<&PeerConnection> {
        self.peers
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .collect()
    }

    /// Get all peers
    pub fn all_peers(&self) -> Vec<&PeerConnection> {
        self.peers.values().collect()
    }

    /// Remove idle peers
    pub fn remove_idle_peers(&mut self, timeout_secs: u64) -> Vec<NodeId> {
        let idle: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, peer)| {
                peer.state == PeerState::Connected && peer.is_idle(timeout_secs)
            })
            .map(|(id, _)| *id)
            .collect();

        for id in &idle {
            self.remove_peer(id);
        }

        idle
    }

    /// Remove failed peers
    pub fn remove_failed_peers(&mut self, max_attempts: u32) -> Vec<NodeId> {
        let failed: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, peer)| peer.failed_attempts >= max_attempts)
            .map(|(id, _)| *id)
            .collect();

        for id in &failed {
            self.remove_peer(id);
        }

        failed
    }

    /// Get statistics about peer connections
    pub fn stats(&self) -> PeerManagerStats {
        let connected = self.peer_count();
        let connecting = self
            .peers
            .values()
            .filter(|p| p.state == PeerState::Connecting)
            .count();
        let failed = self
            .peers
            .values()
            .filter(|p| p.state == PeerState::Failed)
            .count();

        let total_sent: u64 = self.peers.values().map(|p| p.bytes_sent).sum();
        let total_received: u64 = self.peers.values().map(|p| p.bytes_received).sum();

        PeerManagerStats {
            total_peers: self.total_peers(),
            connected,
            connecting,
            failed,
            max_peers: self.max_peers,
            total_bytes_sent: total_sent,
            total_bytes_received: total_received,
        }
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Statistics about the peer manager
#[derive(Debug, Clone)]
pub struct PeerManagerStats {
    pub total_peers: usize,
    pub connected: usize,
    pub connecting: usize,
    pub failed: usize,
    pub max_peers: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_test_peer() -> (NodeId, PublicKey, Vec<NetworkAddress>) {
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
    fn test_peer_connection_create() {
        let (node_id, public_key, addresses) = create_test_peer();
        let conn = PeerConnection::new(node_id, public_key, addresses);

        assert_eq!(conn.node_id, node_id);
        assert_eq!(conn.state, PeerState::Connecting);
        assert_eq!(conn.failed_attempts, 0);
    }

    #[test]
    fn test_peer_connection_state_transitions() {
        let (node_id, public_key, addresses) = create_test_peer();
        let mut conn = PeerConnection::new(node_id, public_key, addresses);

        conn.mark_connected();
        assert_eq!(conn.state, PeerState::Connected);
        assert!(conn.connected_at.is_some());

        conn.mark_disconnected();
        assert_eq!(conn.state, PeerState::Disconnected);
        assert!(conn.connected_at.is_none());

        conn.mark_failed();
        assert_eq!(conn.state, PeerState::Failed);
        assert_eq!(conn.failed_attempts, 1);
    }

    #[test]
    fn test_peer_manager_add() {
        let mut manager = PeerManager::new(10);
        let (node_id, public_key, addresses) = create_test_peer();

        assert!(manager.add_peer(node_id, public_key, addresses.clone()));
        assert_eq!(manager.total_peers(), 1);

        // Adding same peer again should fail
        assert!(!manager.add_peer(node_id, public_key, addresses));
    }

    #[test]
    fn test_peer_manager_capacity() {
        let mut manager = PeerManager::new(2);

        for _ in 0..2 {
            let (node_id, public_key, addresses) = create_test_peer();
            manager.add_peer(node_id, public_key, addresses);
        }

        assert!(!manager.has_capacity());

        // Try to add one more
        let (node_id, public_key, addresses) = create_test_peer();
        assert!(!manager.add_peer(node_id, public_key, addresses));
    }

    #[test]
    fn test_peer_manager_remove() {
        let mut manager = PeerManager::new(10);
        let (node_id, public_key, addresses) = create_test_peer();

        manager.add_peer(node_id, public_key, addresses);
        assert_eq!(manager.total_peers(), 1);

        let removed = manager.remove_peer(&node_id);
        assert!(removed.is_some());
        assert_eq!(manager.total_peers(), 0);
    }

    #[test]
    fn test_peer_manager_stats() {
        let mut manager = PeerManager::new(10);

        for _ in 0..5 {
            let (node_id, public_key, addresses) = create_test_peer();
            manager.add_peer(node_id, public_key, addresses);
        }

        let stats = manager.stats();
        assert_eq!(stats.total_peers, 5);
        assert_eq!(stats.max_peers, 10);
    }
}
