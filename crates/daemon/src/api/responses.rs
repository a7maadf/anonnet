/// API Response types

use serde::{Deserialize, Serialize};

/// Credit balance response
#[derive(Debug, Serialize, Deserialize)]
pub struct CreditBalanceResponse {
    /// Current credit balance
    pub balance: u64,
    /// Node ID
    pub node_id: String,
}

/// Credit statistics response
#[derive(Debug, Serialize, Deserialize)]
pub struct CreditStatsResponse {
    /// Current balance
    pub balance: u64,
    /// Total credits earned from relaying
    pub total_earned: u64,
    /// Total credits spent on circuits
    pub total_spent: u64,
    /// Current earning rate (credits per hour estimate)
    pub earning_rate: f64,
    /// Current spending rate (credits per hour estimate)
    pub spending_rate: f64,
}

/// Network status response
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkStatusResponse {
    /// Node ID
    pub node_id: String,
    /// Whether the node is running
    pub is_running: bool,
    /// Number of known peers in routing table
    pub peer_count: usize,
    /// Number of active connected peers
    pub active_peers: usize,
    /// Total number of circuits
    pub total_circuits: usize,
    /// Number of active circuits
    pub active_circuits: usize,
    /// Total bandwidth (bytes/sec)
    pub bandwidth: u64,
}

/// Active circuit information
#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitInfo {
    /// Circuit ID
    pub circuit_id: String,
    /// Circuit purpose
    pub purpose: String,
    /// Circuit state
    pub state: String,
    /// Number of hops in the circuit
    pub hops: usize,
    /// Age of the circuit in seconds
    pub age_seconds: u64,
    /// Number of times this circuit has been used
    pub use_count: usize,
}

/// Active circuits response
#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveCircuitsResponse {
    /// List of active circuits
    pub circuits: Vec<CircuitInfo>,
    /// Total number of circuits
    pub total: usize,
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// Error code
    pub code: u16,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, code: u16) -> Self {
        Self {
            error: error.into(),
            code,
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(message, 500)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(message, 404)
    }
}
