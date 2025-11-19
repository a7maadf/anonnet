use super::routing_table::RoutingTable;
use crate::identity::NodeId;
use anonnet_common::dht;
use std::collections::{HashMap, HashSet};

/// State for an iterative node lookup operation
#[derive(Debug)]
pub struct NodeLookup {
    /// Target node ID we're looking for
    target: NodeId,

    /// Nodes we've queried (to avoid duplicate queries)
    queried: HashSet<NodeId>,

    /// Nodes we've discovered but haven't queried yet
    pending: Vec<NodeId>,

    /// Best nodes found so far (closest to target)
    closest: Vec<NodeId>,

    /// Maximum number of nodes to return
    k: usize,

    /// Number of parallel queries (alpha in Kademlia)
    alpha: usize,

    /// Whether the lookup is complete
    complete: bool,
}

impl NodeLookup {
    pub fn new(target: NodeId, initial_nodes: Vec<NodeId>) -> Self {
        let mut lookup = Self {
            target,
            queried: HashSet::new(),
            pending: initial_nodes,
            closest: Vec::new(),
            k: dht::K_BUCKET_SIZE,
            alpha: dht::ALPHA,
            complete: false,
        };

        // Check if we're already complete (e.g., no initial nodes)
        lookup.check_completion();
        lookup
    }

    /// Get the target node ID
    pub fn target(&self) -> NodeId {
        self.target
    }

    /// Check if the lookup is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Get the closest nodes found so far
    pub fn closest_nodes(&self) -> &[NodeId] {
        &self.closest
    }

    /// Get the next batch of nodes to query
    ///
    /// Returns up to `alpha` nodes that haven't been queried yet
    pub fn next_queries(&mut self) -> Vec<NodeId> {
        let mut queries = Vec::new();

        // Sort pending by distance to target
        self.pending
            .sort_by_key(|id| id.distance(&self.target));

        // Take up to alpha nodes
        while queries.len() < self.alpha && !self.pending.is_empty() {
            if let Some(node_id) = self.pending.pop() {
                if !self.queried.contains(&node_id) {
                    queries.push(node_id);
                }
            }
        }

        // Mark these as queried
        for node_id in &queries {
            self.queried.insert(*node_id);
        }

        queries
    }

    /// Process the response from a node query
    ///
    /// Updates the lookup state with newly discovered nodes
    pub fn process_response(&mut self, from_node: NodeId, discovered_nodes: Vec<NodeId>) {
        // Add newly discovered nodes to pending
        for node_id in discovered_nodes {
            if node_id != self.target
                && !self.queried.contains(&node_id)
                && !self.pending.contains(&node_id)
            {
                self.pending.push(node_id);
            }
        }

        // Update closest nodes
        self.update_closest();

        // Check if we're done
        self.check_completion();
    }

    /// Mark a query as failed
    pub fn mark_failed(&mut self, node_id: NodeId) {
        self.queried.insert(node_id);
        self.check_completion();
    }

    /// Update the list of closest nodes
    fn update_closest(&mut self) {
        // Collect all known nodes (queried + pending)
        let mut all_nodes: Vec<_> = self
            .queried
            .iter()
            .chain(self.pending.iter())
            .copied()
            .collect();

        // Sort by distance to target
        all_nodes.sort_by_key(|id| id.distance(&self.target));

        // Keep the K closest
        all_nodes.truncate(self.k);

        self.closest = all_nodes;
    }

    /// Check if the lookup is complete
    fn check_completion(&mut self) {
        // Lookup is complete if:
        // 1. We have no more pending queries, OR
        // 2. All of the K closest nodes have been queried

        if self.pending.is_empty() {
            self.complete = true;
            return;
        }

        // Check if all K closest have been queried
        // Only if we have some closest nodes
        if !self.closest.is_empty() {
            let all_closest_queried = self
                .closest
                .iter()
                .take(self.k)
                .all(|id| self.queried.contains(id));

            if all_closest_queried {
                self.complete = true;
            }
        }
    }
}

/// Manager for tracking multiple concurrent lookups
#[derive(Debug)]
pub struct LookupManager {
    /// Active lookups by target node ID
    lookups: HashMap<NodeId, NodeLookup>,
}

impl LookupManager {
    pub fn new() -> Self {
        Self {
            lookups: HashMap::new(),
        }
    }

    /// Start a new lookup
    pub fn start_lookup(&mut self, target: NodeId, initial_nodes: Vec<NodeId>) {
        let lookup = NodeLookup::new(target, initial_nodes);
        self.lookups.insert(target, lookup);
    }

    /// Get a lookup by target
    pub fn get_lookup(&self, target: &NodeId) -> Option<&NodeLookup> {
        self.lookups.get(target)
    }

    /// Get a mutable lookup by target
    pub fn get_lookup_mut(&mut self, target: &NodeId) -> Option<&mut NodeLookup> {
        self.lookups.get_mut(target)
    }

    /// Remove a completed lookup
    pub fn remove_lookup(&mut self, target: &NodeId) -> Option<NodeLookup> {
        self.lookups.remove(target)
    }

    /// Get all active lookups
    pub fn active_lookups(&self) -> impl Iterator<Item = (&NodeId, &NodeLookup)> {
        self.lookups.iter()
    }

    /// Remove all completed lookups
    pub fn cleanup_completed(&mut self) -> Vec<NodeId> {
        let completed: Vec<_> = self
            .lookups
            .iter()
            .filter(|(_, lookup)| lookup.is_complete())
            .map(|(target, _)| *target)
            .collect();

        for target in &completed {
            self.lookups.remove(target);
        }

        completed
    }
}

impl Default for LookupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Find nodes closest to a target using the routing table
pub fn find_closest_nodes(
    routing_table: &RoutingTable,
    target: &NodeId,
    count: usize,
) -> Vec<NodeId> {
    routing_table
        .closest_nodes(target, count)
        .into_iter()
        .map(|entry| entry.node_id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    fn create_test_node_id() -> NodeId {
        let keypair = KeyPair::generate();
        NodeId::from_public_key(&keypair.public_key())
    }

    #[test]
    fn test_node_lookup_create() {
        let target = create_test_node_id();
        let initial = vec![create_test_node_id(), create_test_node_id()];

        let lookup = NodeLookup::new(target, initial.clone());

        assert_eq!(lookup.target(), target);
        assert!(!lookup.is_complete());
        assert_eq!(lookup.closest_nodes().len(), 0);
    }

    #[test]
    fn test_node_lookup_next_queries() {
        let target = create_test_node_id();
        let initial = vec![
            create_test_node_id(),
            create_test_node_id(),
            create_test_node_id(),
            create_test_node_id(),
        ];

        let mut lookup = NodeLookup::new(target, initial);

        let queries = lookup.next_queries();
        assert!(queries.len() <= dht::ALPHA);
        assert!(queries.len() > 0);
    }

    #[test]
    fn test_node_lookup_process_response() {
        let target = create_test_node_id();
        let initial = vec![create_test_node_id()];
        let mut lookup = NodeLookup::new(target, initial.clone());

        let from_node = initial[0];
        let discovered = vec![create_test_node_id(), create_test_node_id()];

        lookup.process_response(from_node, discovered);

        // Should have more pending nodes now
        assert!(lookup.next_queries().len() > 0);
    }

    #[test]
    fn test_lookup_manager() {
        let mut manager = LookupManager::new();
        let target = create_test_node_id();
        let initial = vec![create_test_node_id()];

        manager.start_lookup(target, initial);

        assert!(manager.get_lookup(&target).is_some());
        assert_eq!(manager.active_lookups().count(), 1);
    }

    #[test]
    fn test_lookup_manager_cleanup() {
        let mut manager = LookupManager::new();
        let target = create_test_node_id();

        // Start a lookup with no initial nodes (will complete immediately)
        manager.start_lookup(target, vec![]);

        let completed = manager.cleanup_completed();
        assert_eq!(completed.len(), 1);
        assert_eq!(manager.active_lookups().count(), 0);
    }
}
