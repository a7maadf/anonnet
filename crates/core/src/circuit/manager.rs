use super::crypto::OnionCrypto;
use super::path_selection::{PathSelectionCriteria, PathSelector};
use super::types::{Circuit, CircuitId, CircuitNode, CircuitPurpose, CircuitState};
use crate::dht::RoutingTable;
use crate::identity::NodeId;
use anonnet_common::{routing, AnonNetError, Result};
use std::collections::HashMap;

/// Manager for all circuits
pub struct CircuitManager {
    /// Active circuits
    circuits: HashMap<CircuitId, Circuit>,

    /// Maximum number of circuits to maintain
    max_circuits: usize,

    /// Circuits by purpose
    circuits_by_purpose: HashMap<CircuitPurpose, Vec<CircuitId>>,
}

impl CircuitManager {
    pub fn new() -> Self {
        Self {
            circuits: HashMap::new(),
            max_circuits: routing::MAX_CIRCUITS,
            circuits_by_purpose: HashMap::new(),
        }
    }

    pub fn with_capacity(max_circuits: usize) -> Self {
        Self {
            circuits: HashMap::new(),
            max_circuits,
            circuits_by_purpose: HashMap::new(),
        }
    }

    /// Create a new circuit
    pub fn create_circuit(
        &mut self,
        routing_table: &RoutingTable,
        purpose: CircuitPurpose,
        criteria: Option<PathSelectionCriteria>,
    ) -> Result<CircuitId> {
        if self.circuits.len() >= self.max_circuits {
            return Err(AnonNetError::internal("Maximum circuits reached"));
        }

        // Select a path
        let criteria = criteria.unwrap_or_default();
        let path = PathSelector::select_path(routing_table, &criteria)
            .map_err(|e| AnonNetError::internal(format!("Path selection failed: {}", e)))?;

        // Create the circuit
        let circuit_id = CircuitId::generate();
        let mut circuit = Circuit::new(circuit_id, purpose);

        // Add nodes to circuit with encryption keys
        for node_id in path {
            if let Some(entry) = routing_table.find_node(&node_id) {
                // Generate encryption keys for this hop
                let enc_key = OnionCrypto::generate_key();
                let dec_key = OnionCrypto::generate_key();

                let node = CircuitNode::new(node_id, entry.public_key, enc_key, dec_key);
                circuit.add_node(node);
            }
        }

        // Store the circuit
        self.circuits.insert(circuit_id, circuit);

        // Track by purpose
        self.circuits_by_purpose
            .entry(purpose)
            .or_default()
            .push(circuit_id);

        Ok(circuit_id)
    }

    /// Get a circuit by ID
    pub fn get_circuit(&self, id: &CircuitId) -> Option<&Circuit> {
        self.circuits.get(id)
    }

    /// Get a mutable circuit by ID
    pub fn get_circuit_mut(&mut self, id: &CircuitId) -> Option<&mut Circuit> {
        self.circuits.get_mut(id)
    }

    /// Get a ready circuit for a specific purpose
    pub fn get_ready_circuit(&self, purpose: CircuitPurpose) -> Option<&Circuit> {
        self.circuits_by_purpose
            .get(&purpose)
            .and_then(|ids| {
                ids.iter()
                    .filter_map(|id| self.circuits.get(id))
                    .find(|c| c.is_ready())
            })
    }

    /// Get a ready circuit (any purpose)
    pub fn get_any_ready_circuit(&self) -> Option<&Circuit> {
        self.circuits
            .values()
            .find(|c| c.is_ready() && !c.is_expired())
    }

    /// Get all circuits
    pub fn all_circuits(&self) -> Vec<&Circuit> {
        self.circuits.values().collect()
    }

    /// Get ready circuits
    pub fn ready_circuits(&self) -> Vec<&Circuit> {
        self.circuits
            .values()
            .filter(|c| c.is_ready() && !c.is_expired())
            .collect()
    }

    /// Mark a circuit as failed
    pub fn mark_failed(&mut self, id: &CircuitId) -> bool {
        if let Some(circuit) = self.circuits.get_mut(id) {
            circuit.mark_failed();
            true
        } else {
            false
        }
    }

    /// Destroy a circuit
    pub fn destroy_circuit(&mut self, id: &CircuitId) -> Option<Circuit> {
        if let Some(mut circuit) = self.circuits.remove(id) {
            circuit.mark_closed();

            // Remove from purpose tracking
            if let Some(ids) = self.circuits_by_purpose.get_mut(&circuit.purpose) {
                ids.retain(|cid| cid != id);
            }

            Some(circuit)
        } else {
            None
        }
    }

    /// Clean up expired and failed circuits
    pub fn cleanup(&mut self) -> CircuitCleanupStats {
        let mut stats = CircuitCleanupStats::default();

        let to_remove: Vec<_> = self
            .circuits
            .iter()
            .filter(|(_, circuit)| {
                let expired = circuit.is_expired();
                let failed = circuit.state == CircuitState::Failed;
                let closed = circuit.state == CircuitState::Closed;

                if expired {
                    stats.expired += 1;
                } else if failed {
                    stats.failed += 1;
                } else if closed {
                    stats.closed += 1;
                }

                expired || failed || closed
            })
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            self.destroy_circuit(&id);
        }

        stats.total_removed = stats.expired + stats.failed + stats.closed;
        stats
    }

    /// Get statistics about circuits
    pub fn stats(&self) -> CircuitManagerStats {
        let total = self.circuits.len();
        let building = self
            .circuits
            .values()
            .filter(|c| c.state == CircuitState::Building)
            .count();
        let ready = self
            .circuits
            .values()
            .filter(|c| c.state == CircuitState::Ready)
            .count();
        let failed = self
            .circuits
            .values()
            .filter(|c| c.state == CircuitState::Failed)
            .count();

        let total_bytes_sent: u64 = self.circuits.values().map(|c| c.bytes_sent).sum();
        let total_bytes_received: u64 = self.circuits.values().map(|c| c.bytes_received).sum();

        CircuitManagerStats {
            total_circuits: total,
            building,
            ready,
            failed,
            max_circuits: self.max_circuits,
            total_bytes_sent,
            total_bytes_received,
        }
    }

    /// Ensure we have enough ready circuits
    pub fn ensure_circuits(
        &mut self,
        routing_table: &RoutingTable,
        purpose: CircuitPurpose,
        min_count: usize,
    ) -> Result<usize> {
        let current = self
            .circuits_by_purpose
            .get(&purpose)
            .map(|ids| {
                ids.iter()
                    .filter(|id| {
                        self.circuits
                            .get(id)
                            .map(|c| c.is_ready())
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0);

        let mut created = 0;

        while current + created < min_count && self.circuits.len() < self.max_circuits {
            match self.create_circuit(routing_table, purpose, None) {
                Ok(_) => created += 1,
                Err(_) => break,
            }
        }

        Ok(created)
    }
}

impl Default for CircuitManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about circuit cleanup
#[derive(Debug, Default)]
pub struct CircuitCleanupStats {
    pub expired: usize,
    pub failed: usize,
    pub closed: usize,
    pub total_removed: usize,
}

/// Statistics about the circuit manager
#[derive(Debug)]
pub struct CircuitManagerStats {
    pub total_circuits: usize,
    pub building: usize,
    pub ready: usize,
    pub failed: usize,
    pub max_circuits: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
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

        // Add 10 test nodes
        for i in 0..10 {
            let keypair = KeyPair::generate();
            let node_id = NodeId::from_public_key(&keypair.public_key());
            let addr = NetworkAddress::from_socket(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8 + 1)),
                8080,
            ));

            table.insert(node_id, keypair.public_key(), vec![addr]).ok();
        }

        table
    }

    #[test]
    fn test_circuit_manager_create() {
        let manager = CircuitManager::new();
        assert_eq!(manager.all_circuits().len(), 0);
    }

    #[test]
    fn test_create_circuit() {
        let mut manager = CircuitManager::new();
        let table = create_test_routing_table();

        let result = manager.create_circuit(&table, CircuitPurpose::General, None);
        assert!(result.is_ok());

        let stats = manager.stats();
        assert_eq!(stats.total_circuits, 1);
    }

    #[test]
    fn test_get_circuit() {
        let mut manager = CircuitManager::new();
        let table = create_test_routing_table();

        let id = manager
            .create_circuit(&table, CircuitPurpose::General, None)
            .unwrap();

        let circuit = manager.get_circuit(&id);
        assert!(circuit.is_some());
        assert_eq!(circuit.unwrap().id, id);
    }

    #[test]
    fn test_destroy_circuit() {
        let mut manager = CircuitManager::new();
        let table = create_test_routing_table();

        let id = manager
            .create_circuit(&table, CircuitPurpose::General, None)
            .unwrap();

        let destroyed = manager.destroy_circuit(&id);
        assert!(destroyed.is_some());

        assert!(manager.get_circuit(&id).is_none());
    }

    #[test]
    fn test_circuit_manager_stats() {
        let mut manager = CircuitManager::new();
        let table = create_test_routing_table();

        // Create a few circuits
        for _ in 0..3 {
            manager
                .create_circuit(&table, CircuitPurpose::General, None)
                .ok();
        }

        let stats = manager.stats();
        assert_eq!(stats.total_circuits, 3);
    }

    #[test]
    fn test_ensure_circuits() {
        let mut manager = CircuitManager::new();
        let table = create_test_routing_table();

        let created = manager
            .ensure_circuits(&table, CircuitPurpose::General, 3)
            .unwrap();

        assert!(created > 0);
        assert!(manager.ready_circuits().len() >= 3 || manager.all_circuits().len() >= 3);
    }
}
