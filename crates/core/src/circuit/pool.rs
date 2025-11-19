/// Circuit pooling for efficient reuse
///
/// Maintains a pool of ready-to-use circuits to reduce latency
/// when establishing connections. Circuits are categorized by purpose
/// and reused when appropriate.

use super::{CircuitId, CircuitManager, CircuitPurpose};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for the circuit pool
#[derive(Debug, Clone)]
pub struct CircuitPoolConfig {
    /// Target number of circuits to maintain per purpose
    pub target_pool_size: usize,

    /// Maximum age of a circuit before retirement
    pub max_circuit_age: Duration,

    /// Minimum idle time before circuit can be reused
    pub min_idle_time: Duration,

    /// Maximum number of times a circuit can be reused
    pub max_reuse_count: usize,
}

impl Default for CircuitPoolConfig {
    fn default() -> Self {
        Self {
            target_pool_size: 3,
            max_circuit_age: Duration::from_secs(600), // 10 minutes
            min_idle_time: Duration::from_secs(5),
            max_reuse_count: 10,
        }
    }
}

/// Pooled circuit metadata
#[derive(Debug)]
struct PooledCircuit {
    /// The circuit ID
    circuit_id: CircuitId,

    /// When this circuit was created
    created_at: Instant,

    /// When this circuit was last used
    last_used: Instant,

    /// Number of times this circuit has been reused
    reuse_count: usize,

    /// Whether this circuit is currently in use
    in_use: bool,
}

/// Circuit pool manager
pub struct CircuitPool {
    /// Configuration
    config: CircuitPoolConfig,

    /// Pools organized by purpose
    pools: Arc<RwLock<HashMap<CircuitPurpose, Vec<PooledCircuit>>>>,

    /// Reference to circuit manager
    circuit_manager: Arc<RwLock<CircuitManager>>,
}

impl CircuitPool {
    /// Create a new circuit pool
    pub fn new(
        config: CircuitPoolConfig,
        circuit_manager: Arc<RwLock<CircuitManager>>,
    ) -> Self {
        Self {
            config,
            pools: Arc::new(RwLock::new(HashMap::new())),
            circuit_manager,
        }
    }

    /// Get a circuit from the pool, or create a new one if needed
    pub async fn acquire_circuit(
        &self,
        purpose: CircuitPurpose,
    ) -> Result<CircuitId, CircuitPoolError> {
        // Try to get an existing circuit from pool
        {
            let mut pools = self.pools.write().await;
            let pool = pools.entry(purpose).or_insert_with(Vec::new);

            // Find a reusable circuit
            for circuit in pool.iter_mut() {
                if self.is_reusable(circuit) && !circuit.in_use {
                    circuit.in_use = true;
                    circuit.reuse_count += 1;
                    circuit.last_used = Instant::now();
                    return Ok(circuit.circuit_id);
                }
            }
        }

        // No suitable circuit found, create a new one
        // For now, return an error since we don't have full integration
        Err(CircuitPoolError::NoCircuitsAvailable)
    }

    /// Release a circuit back to the pool
    pub async fn release_circuit(&self, circuit_id: CircuitId, purpose: CircuitPurpose) {
        let mut pools = self.pools.write().await;
        if let Some(pool) = pools.get_mut(&purpose) {
            for circuit in pool.iter_mut() {
                if circuit.circuit_id == circuit_id {
                    circuit.in_use = false;
                    circuit.last_used = Instant::now();
                    break;
                }
            }
        }
    }

    /// Add a newly created circuit to the pool
    pub async fn add_circuit(&self, circuit_id: CircuitId, purpose: CircuitPurpose) {
        let mut pools = self.pools.write().await;
        let pool = pools.entry(purpose).or_insert_with(Vec::new);

        pool.push(PooledCircuit {
            circuit_id,
            created_at: Instant::now(),
            last_used: Instant::now(),
            reuse_count: 0,
            in_use: false,
        });
    }

    /// Clean up old and overused circuits
    pub async fn cleanup(&self) {
        let mut pools = self.pools.write().await;

        for pool in pools.values_mut() {
            pool.retain(|circuit| {
                // Remove if too old
                if circuit.created_at.elapsed() > self.config.max_circuit_age {
                    return false;
                }

                // Remove if reused too many times
                if circuit.reuse_count >= self.config.max_reuse_count {
                    return false;
                }

                true
            });
        }
    }

    /// Check if a circuit is reusable
    fn is_reusable(&self, circuit: &PooledCircuit) -> bool {
        // Not too old
        if circuit.created_at.elapsed() > self.config.max_circuit_age {
            return false;
        }

        // Not reused too many times
        if circuit.reuse_count >= self.config.max_reuse_count {
            return false;
        }

        // Has been idle long enough
        if circuit.last_used.elapsed() < self.config.min_idle_time {
            return false;
        }

        true
    }

    /// Get pool statistics
    pub async fn stats(&self) -> CircuitPoolStats {
        let pools = self.pools.read().await;

        let mut total_circuits = 0;
        let mut in_use_circuits = 0;
        let mut pools_by_purpose = HashMap::new();

        for (purpose, pool) in pools.iter() {
            total_circuits += pool.len();
            in_use_circuits += pool.iter().filter(|c| c.in_use).count();

            pools_by_purpose.insert(
                *purpose,
                PoolStats {
                    total: pool.len(),
                    in_use: pool.iter().filter(|c| c.in_use).count(),
                    available: pool.iter().filter(|c| !c.in_use).count(),
                },
            );
        }

        CircuitPoolStats {
            total_circuits,
            in_use_circuits,
            available_circuits: total_circuits - in_use_circuits,
            pools_by_purpose,
        }
    }
}

/// Statistics for a single pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total: usize,
    pub in_use: usize,
    pub available: usize,
}

/// Overall circuit pool statistics
#[derive(Debug, Clone)]
pub struct CircuitPoolStats {
    pub total_circuits: usize,
    pub in_use_circuits: usize,
    pub available_circuits: usize,
    pub pools_by_purpose: HashMap<CircuitPurpose, PoolStats>,
}

/// Circuit pool errors
#[derive(Debug, thiserror::Error)]
pub enum CircuitPoolError {
    #[error("No circuits available")]
    NoCircuitsAvailable,

    #[error("Circuit creation failed: {0}")]
    CreationFailed(String),

    #[error("Invalid circuit purpose")]
    InvalidPurpose,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use crate::identity::NodeId;
    use crate::dht::RoutingTable;

    #[tokio::test]
    async fn test_circuit_pool_creation() {
        let circuit_manager = Arc::new(RwLock::new(CircuitManager::new()));

        let pool = CircuitPool::new(CircuitPoolConfig::default(), circuit_manager);

        let stats = pool.stats().await;
        assert_eq!(stats.total_circuits, 0);
    }

    #[tokio::test]
    async fn test_add_and_release_circuit() {
        let circuit_manager = Arc::new(RwLock::new(CircuitManager::new()));

        let pool = CircuitPool::new(CircuitPoolConfig::default(), circuit_manager);

        // Add a circuit
        let circuit_id = CircuitId(1);
        pool.add_circuit(circuit_id, CircuitPurpose::General).await;

        let stats = pool.stats().await;
        assert_eq!(stats.total_circuits, 1);
        assert_eq!(stats.available_circuits, 1);
    }

    #[tokio::test]
    async fn test_cleanup_old_circuits() {
        let circuit_manager = Arc::new(RwLock::new(CircuitManager::new()));

        let mut config = CircuitPoolConfig::default();
        config.max_circuit_age = Duration::from_millis(1); // Very short for testing

        let pool = CircuitPool::new(config, circuit_manager);

        // Add a circuit
        pool.add_circuit(CircuitId(1), CircuitPurpose::General).await;

        // Wait for it to age
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cleanup should remove it
        pool.cleanup().await;

        let stats = pool.stats().await;
        assert_eq!(stats.total_circuits, 0);
    }
}
