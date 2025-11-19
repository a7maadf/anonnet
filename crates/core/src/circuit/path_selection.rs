use crate::dht::{BucketEntry, RoutingTable};
use crate::identity::NodeId;
use anonnet_common::{routing, Reputation};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;

/// Criteria for selecting nodes for a circuit
#[derive(Debug, Clone)]
pub struct PathSelectionCriteria {
    /// Minimum reputation score required
    pub min_reputation: Reputation,

    /// Whether to require relay capability
    pub require_relay: bool,

    /// Nodes to exclude from selection
    pub excluded_nodes: HashSet<NodeId>,

    /// Desired circuit length
    pub circuit_length: usize,
}

impl Default for PathSelectionCriteria {
    fn default() -> Self {
        Self {
            min_reputation: Reputation::new(50),
            require_relay: true,
            excluded_nodes: HashSet::new(),
            circuit_length: routing::DEFAULT_CIRCUIT_LENGTH,
        }
    }
}

impl PathSelectionCriteria {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_min_reputation(mut self, reputation: Reputation) -> Self {
        self.min_reputation = reputation;
        self
    }

    pub fn with_circuit_length(mut self, length: usize) -> Self {
        self.circuit_length = length.max(routing::MIN_CIRCUIT_LENGTH)
            .min(routing::MAX_CIRCUIT_LENGTH);
        self
    }

    pub fn exclude_node(mut self, node_id: NodeId) -> Self {
        self.excluded_nodes.insert(node_id);
        self
    }

    pub fn exclude_nodes(mut self, nodes: &[NodeId]) -> Self {
        self.excluded_nodes.extend(nodes);
        self
    }
}

/// Select nodes for a circuit path
pub struct PathSelector;

impl PathSelector {
    /// Select a random path through the network
    pub fn select_path(
        routing_table: &RoutingTable,
        criteria: &PathSelectionCriteria,
    ) -> Result<Vec<NodeId>, PathSelectionError> {
        let mut rng = rand::thread_rng();

        // Get all potential nodes
        let candidates = Self::get_candidates(routing_table, criteria);

        if candidates.len() < criteria.circuit_length {
            return Err(PathSelectionError::InsufficientNodes {
                available: candidates.len(),
                required: criteria.circuit_length,
            });
        }

        // Group candidates by quality (reputation-based)
        let high_quality: Vec<_> = candidates
            .iter()
            .filter(|c| c.1 >= Reputation::new(200))
            .collect();

        let medium_quality: Vec<_> = candidates
            .iter()
            .filter(|c| c.1 >= Reputation::new(100) && c.1 < Reputation::new(200))
            .collect();

        let low_quality: Vec<_> = candidates
            .iter()
            .filter(|c| c.1 < Reputation::new(100))
            .collect();

        let mut selected = Vec::new();

        // Strategy: Use high-quality nodes when possible, fallback to medium/low
        for i in 0..criteria.circuit_length {
            let pool = if i == 0 || i == criteria.circuit_length - 1 {
                // Entry and exit nodes should be high quality
                if !high_quality.is_empty() {
                    &high_quality
                } else {
                    &medium_quality
                }
            } else {
                // Middle nodes can be medium or high quality
                if !medium_quality.is_empty() && !high_quality.is_empty() {
                    if rng.gen_bool(0.5) {
                        &high_quality
                    } else {
                        &medium_quality
                    }
                } else if !high_quality.is_empty() {
                    &high_quality
                } else {
                    &medium_quality
                }
            };

            if pool.is_empty() {
                return Err(PathSelectionError::InsufficientQualityNodes);
            }

            // Select a random node from the pool that we haven't used yet
            let available: Vec<_> = pool
                .iter()
                .filter(|(node_id, _)| !selected.contains(node_id))
                .collect();

            if available.is_empty() {
                return Err(PathSelectionError::InsufficientUniqueNodes);
            }

            let chosen = available.choose(&mut rng).unwrap();
            selected.push(chosen.0);
        }

        Ok(selected)
    }

    /// Get candidate nodes from routing table
    fn get_candidates(
        routing_table: &RoutingTable,
        criteria: &PathSelectionCriteria,
    ) -> Vec<(NodeId, Reputation)> {
        routing_table
            .all_nodes()
            .into_iter()
            .filter(|entry| {
                // Exclude explicitly excluded nodes
                if criteria.excluded_nodes.contains(&entry.node_id) {
                    return false;
                }

                // Check reputation
                if entry.reputation < criteria.min_reputation {
                    return false;
                }

                // Check relay capability
                if criteria.require_relay && !entry.accepts_relay {
                    return false;
                }

                // Check freshness (not stale)
                if entry.is_stale() {
                    return false;
                }

                true
            })
            .map(|entry| (entry.node_id, entry.reputation))
            .collect()
    }

    /// Select a path with diversity (try to avoid related nodes)
    pub fn select_diverse_path(
        routing_table: &RoutingTable,
        criteria: &PathSelectionCriteria,
    ) -> Result<Vec<NodeId>, PathSelectionError> {
        // For now, use the basic selection
        // In a real implementation, this would also check:
        // - IP address diversity (/16 subnet)
        // - Geographic diversity
        // - Operator diversity (based on contact info)
        // - Autonomous System diversity

        Self::select_path(routing_table, criteria)
    }

    /// Select an exit node (last hop)
    pub fn select_exit_node(
        routing_table: &RoutingTable,
        excluded: &[NodeId],
    ) -> Result<NodeId, PathSelectionError> {
        let criteria = PathSelectionCriteria::default()
            .exclude_nodes(excluded)
            .with_min_reputation(Reputation::new(150)); // Higher reputation for exit

        let candidates = Self::get_candidates(routing_table, &criteria);

        if candidates.is_empty() {
            return Err(PathSelectionError::NoSuitableExit);
        }

        let mut rng = rand::thread_rng();
        let selected = candidates.choose(&mut rng).unwrap();

        Ok(selected.0)
    }

    /// Select an entry guard (first hop)
    ///
    /// Entry guards should be stable, long-lived nodes with high reputation
    pub fn select_entry_guard(
        routing_table: &RoutingTable,
        excluded: &[NodeId],
    ) -> Result<NodeId, PathSelectionError> {
        let criteria = PathSelectionCriteria::default()
            .exclude_nodes(excluded)
            .with_min_reputation(Reputation::new(200)); // High reputation for entry

        let candidates = Self::get_candidates(routing_table, &criteria);

        if candidates.is_empty() {
            return Err(PathSelectionError::NoSuitableEntry);
        }

        // For entry guards, prefer nodes with highest reputation
        let best = candidates
            .iter()
            .max_by_key(|(_, rep)| rep)
            .unwrap();

        Ok(best.0)
    }
}

/// Errors that can occur during path selection
#[derive(Debug, thiserror::Error)]
pub enum PathSelectionError {
    #[error("Insufficient nodes available: {available} < {required}")]
    InsufficientNodes { available: usize, required: usize },

    #[error("Insufficient quality nodes available")]
    InsufficientQualityNodes,

    #[error("Insufficient unique nodes available")]
    InsufficientUniqueNodes,

    #[error("No suitable exit node found")]
    NoSuitableExit,

    #[error("No suitable entry guard found")]
    NoSuitableEntry,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use anonnet_common::NetworkAddress;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_test_routing_table() -> RoutingTable {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        // Add 20 test nodes with varying reputations
        for i in 0..20 {
            let keypair = KeyPair::generate();
            let node_id = NodeId::from_public_key(&keypair.public_key());
            let addr = NetworkAddress::from_socket(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8 + 1)),
                8080,
            ));

            table.insert(node_id, keypair.public_key(), vec![addr]).ok();

            // Set varying reputations
            if let Some(entry) = table.all_nodes().iter_mut().find(|e| e.node_id == node_id) {
                entry.reputation = Reputation::new((i * 10 + 50) as u32);
                entry.accepts_relay = true;
            }
        }

        table
    }

    #[test]
    fn test_path_selection() {
        let table = create_test_routing_table();
        let criteria = PathSelectionCriteria::default();

        let path = PathSelector::select_path(&table, &criteria);
        assert!(path.is_ok());

        let nodes = path.unwrap();
        assert_eq!(nodes.len(), routing::DEFAULT_CIRCUIT_LENGTH);

        // Ensure all nodes are unique
        let unique: HashSet<_> = nodes.iter().collect();
        assert_eq!(unique.len(), nodes.len());
    }

    #[test]
    fn test_path_selection_exclusions() {
        let table = create_test_routing_table();
        let all_nodes: Vec<_> = table.all_nodes().into_iter().map(|e| e.node_id).collect();

        let criteria = PathSelectionCriteria::default()
            .exclude_node(all_nodes[0])
            .exclude_node(all_nodes[1]);

        let path = PathSelector::select_path(&table, &criteria).unwrap();

        // Excluded nodes should not appear
        assert!(!path.contains(&all_nodes[0]));
        assert!(!path.contains(&all_nodes[1]));
    }

    #[test]
    fn test_insufficient_nodes() {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let table = RoutingTable::new(local_id); // Empty table

        let criteria = PathSelectionCriteria::default();
        let result = PathSelector::select_path(&table, &criteria);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PathSelectionError::InsufficientNodes { .. }
        ));
    }

    #[test]
    fn test_custom_circuit_length() {
        let table = create_test_routing_table();
        let criteria = PathSelectionCriteria::default().with_circuit_length(5);

        let path = PathSelector::select_path(&table, &criteria).unwrap();
        assert_eq!(path.len(), 5);
    }
}
