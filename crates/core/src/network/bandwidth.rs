/// Bandwidth estimation and tracking
///
/// Tracks bandwidth usage across the network for relay nodes,
/// circuits, and overall network health. Used for path selection
/// and credit calculations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::identity::NodeId;

/// Bandwidth estimator for network nodes
pub struct BandwidthEstimator {
    /// Per-node bandwidth measurements
    node_stats: Arc<RwLock<HashMap<NodeId, NodeBandwidthStats>>>,

    /// Overall network statistics
    network_stats: Arc<RwLock<NetworkBandwidthStats>>,

    /// Configuration
    config: BandwidthConfig,
}

/// Configuration for bandwidth estimation
#[derive(Debug, Clone)]
pub struct BandwidthConfig {
    /// Window size for moving average calculations
    pub measurement_window: Duration,

    /// Minimum samples required for reliable estimate
    pub min_samples: usize,

    /// How often to update estimates
    pub update_interval: Duration,
}

impl Default for BandwidthConfig {
    fn default() -> Self {
        Self {
            measurement_window: Duration::from_secs(60),
            min_samples: 10,
            update_interval: Duration::from_secs(5),
        }
    }
}

/// Bandwidth statistics for a single node
#[derive(Debug, Clone)]
pub struct NodeBandwidthStats {
    /// Node ID
    pub node_id: NodeId,

    /// Total bytes sent through this node
    pub bytes_sent: u64,

    /// Total bytes received from this node
    pub bytes_received: u64,

    /// Estimated bandwidth (bytes per second)
    pub estimated_bandwidth: u64,

    /// Average latency in milliseconds
    pub avg_latency_ms: u32,

    /// Number of successful transfers
    pub successful_transfers: u64,

    /// Number of failed transfers
    pub failed_transfers: u64,

    /// Last update time
    pub last_updated: Instant,

    /// Recent measurements for moving average
    recent_measurements: Vec<BandwidthMeasurement>,
}

/// A single bandwidth measurement
#[derive(Debug, Clone)]
struct BandwidthMeasurement {
    /// When this measurement was taken
    timestamp: Instant,

    /// Bytes transferred
    bytes: u64,

    /// Duration of transfer
    duration: Duration,

    /// Latency in milliseconds
    latency_ms: u32,
}

/// Network-wide bandwidth statistics
#[derive(Debug, Clone)]
pub struct NetworkBandwidthStats {
    /// Total bandwidth across all nodes (bytes/sec)
    pub total_bandwidth: u64,

    /// Average bandwidth per node
    pub avg_bandwidth_per_node: u64,

    /// Number of active nodes
    pub active_nodes: usize,

    /// Total bytes transferred in network
    pub total_bytes_transferred: u64,

    /// Network-wide average latency
    pub network_avg_latency_ms: u32,

    /// Last update time
    pub last_updated: Instant,
}

impl BandwidthEstimator {
    /// Create a new bandwidth estimator
    pub fn new(config: BandwidthConfig) -> Self {
        Self {
            node_stats: Arc::new(RwLock::new(HashMap::new())),
            network_stats: Arc::new(RwLock::new(NetworkBandwidthStats {
                total_bandwidth: 0,
                avg_bandwidth_per_node: 0,
                active_nodes: 0,
                total_bytes_transferred: 0,
                network_avg_latency_ms: 0,
                last_updated: Instant::now(),
            })),
            config,
        }
    }

    /// Record a bandwidth measurement for a node
    pub async fn record_transfer(
        &self,
        node_id: NodeId,
        bytes: u64,
        duration: Duration,
        latency_ms: u32,
    ) {
        let mut stats = self.node_stats.write().await;

        let node_stats = stats.entry(node_id).or_insert_with(|| NodeBandwidthStats {
            node_id,
            bytes_sent: 0,
            bytes_received: 0,
            estimated_bandwidth: 0,
            avg_latency_ms: 0,
            successful_transfers: 0,
            failed_transfers: 0,
            last_updated: Instant::now(),
            recent_measurements: Vec::new(),
        });

        // Add measurement
        node_stats.recent_measurements.push(BandwidthMeasurement {
            timestamp: Instant::now(),
            bytes,
            duration,
            latency_ms,
        });

        // Remove old measurements outside window
        let cutoff = Instant::now() - self.config.measurement_window;
        node_stats
            .recent_measurements
            .retain(|m| m.timestamp > cutoff);

        // Update statistics
        node_stats.bytes_sent += bytes;
        node_stats.successful_transfers += 1;
        node_stats.last_updated = Instant::now();

        // Recalculate estimates
        self.update_estimates_for_node(node_stats);
    }

    /// Record a failed transfer
    pub async fn record_failure(&self, node_id: NodeId) {
        let mut stats = self.node_stats.write().await;

        if let Some(node_stats) = stats.get_mut(&node_id) {
            node_stats.failed_transfers += 1;
            node_stats.last_updated = Instant::now();
        }
    }

    /// Get bandwidth estimate for a node
    pub async fn get_node_bandwidth(&self, node_id: &NodeId) -> Option<u64> {
        let stats = self.node_stats.read().await;
        stats.get(node_id).map(|s| s.estimated_bandwidth)
    }

    /// Get latency estimate for a node
    pub async fn get_node_latency(&self, node_id: &NodeId) -> Option<u32> {
        let stats = self.node_stats.read().await;
        stats.get(node_id).map(|s| s.avg_latency_ms)
    }

    /// Get full statistics for a node
    pub async fn get_node_stats(&self, node_id: &NodeId) -> Option<NodeBandwidthStats> {
        let stats = self.node_stats.read().await;
        stats.get(node_id).cloned()
    }

    /// Get network-wide statistics
    pub async fn get_network_stats(&self) -> NetworkBandwidthStats {
        let network_stats = self.network_stats.read().await;
        network_stats.clone()
    }

    /// Update network-wide statistics
    pub async fn update_network_stats(&self) {
        let node_stats = self.node_stats.read().await;

        let mut total_bandwidth = 0;
        let mut total_latency = 0;
        let mut active_nodes = 0;
        let mut total_bytes = 0;

        for stats in node_stats.values() {
            total_bandwidth += stats.estimated_bandwidth;
            total_latency += stats.avg_latency_ms as u64;
            total_bytes += stats.bytes_sent + stats.bytes_received;
            active_nodes += 1;
        }

        let avg_bandwidth = if active_nodes > 0 {
            total_bandwidth / active_nodes as u64
        } else {
            0
        };

        let avg_latency = if active_nodes > 0 {
            (total_latency / active_nodes as u64) as u32
        } else {
            0
        };

        let mut network_stats = self.network_stats.write().await;
        network_stats.total_bandwidth = total_bandwidth;
        network_stats.avg_bandwidth_per_node = avg_bandwidth;
        network_stats.active_nodes = active_nodes;
        network_stats.total_bytes_transferred = total_bytes;
        network_stats.network_avg_latency_ms = avg_latency;
        network_stats.last_updated = Instant::now();
    }

    /// Update estimates for a specific node
    fn update_estimates_for_node(&self, stats: &mut NodeBandwidthStats) {
        if stats.recent_measurements.len() < self.config.min_samples {
            return;
        }

        // Calculate bandwidth as moving average
        let total_bytes: u64 = stats.recent_measurements.iter().map(|m| m.bytes).sum();
        let total_duration: Duration = stats
            .recent_measurements
            .iter()
            .map(|m| m.duration)
            .sum();

        if !total_duration.is_zero() {
            stats.estimated_bandwidth = total_bytes / total_duration.as_secs().max(1);
        }

        // Calculate average latency
        let total_latency: u32 = stats.recent_measurements.iter().map(|m| m.latency_ms).sum();
        stats.avg_latency_ms = total_latency / stats.recent_measurements.len() as u32;
    }

    /// Get top N nodes by bandwidth
    pub async fn get_top_nodes(&self, n: usize) -> Vec<NodeBandwidthStats> {
        let stats = self.node_stats.read().await;

        let mut nodes: Vec<_> = stats.values().cloned().collect();
        nodes.sort_by(|a, b| b.estimated_bandwidth.cmp(&a.estimated_bandwidth));
        nodes.truncate(n);
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[tokio::test]
    async fn test_bandwidth_estimator_creation() {
        let estimator = BandwidthEstimator::new(BandwidthConfig::default());
        let stats = estimator.get_network_stats().await;

        assert_eq!(stats.active_nodes, 0);
        assert_eq!(stats.total_bandwidth, 0);
    }

    #[tokio::test]
    async fn test_record_transfer() {
        let estimator = BandwidthEstimator::new(BandwidthConfig::default());
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        // Record a transfer
        estimator
            .record_transfer(node_id, 1024, Duration::from_secs(1), 50)
            .await;

        // Should have statistics
        let stats = estimator.get_node_stats(&node_id).await;
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.successful_transfers, 1);
    }

    #[tokio::test]
    async fn test_network_stats_update() {
        let estimator = BandwidthEstimator::new(BandwidthConfig::default());
        let keypair1 = KeyPair::generate();
        let node1 = NodeId::from_public_key(&keypair1.public_key());

        // Record some transfers
        for _ in 0..20 {
            estimator
                .record_transfer(node1, 1024, Duration::from_secs(1), 50)
                .await;
        }

        // Update network stats
        estimator.update_network_stats().await;

        let network_stats = estimator.get_network_stats().await;
        assert_eq!(network_stats.active_nodes, 1);
        assert!(network_stats.total_bandwidth > 0);
    }
}
