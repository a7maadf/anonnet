use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Network protocol constants
pub mod protocol {
    /// Current protocol version
    pub const VERSION: u32 = 1;

    /// Default port for node communication
    pub const DEFAULT_PORT: u16 = 9090;

    /// Maximum message size (10 MB)
    pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

    /// Connection timeout
    pub const CONNECTION_TIMEOUT_SECS: u64 = 30;

    /// Handshake timeout
    pub const HANDSHAKE_TIMEOUT_SECS: u64 = 10;

    /// Keep-alive interval
    pub const KEEPALIVE_INTERVAL_SECS: u64 = 30;

    /// Maximum concurrent connections per peer
    pub const MAX_CONNECTIONS_PER_PEER: usize = 3;
}

/// DHT (Distributed Hash Table) constants
pub mod dht {
    /// K-bucket size (Kademlia parameter)
    pub const K_BUCKET_SIZE: usize = 20;

    /// Alpha (concurrency parameter for parallel lookups)
    pub const ALPHA: usize = 3;

    /// Number of closest nodes to maintain
    pub const REPLICATION_FACTOR: usize = 20;

    /// Node info refresh interval
    pub const REFRESH_INTERVAL_SECS: u64 = 3600;

    /// Bootstrap node timeout
    pub const BOOTSTRAP_TIMEOUT_SECS: u64 = 60;

    /// Maximum age for node info before considering stale
    pub const MAX_NODE_AGE_SECS: u64 = 7200; // 2 hours
}

/// Circuit routing constants
pub mod routing {
    /// Default circuit length (number of hops)
    pub const DEFAULT_CIRCUIT_LENGTH: usize = 3;

    /// Minimum circuit length (reduced to 1 for early network growth)
    /// WARNING: 1-2 hop circuits provide reduced anonymity
    pub const MIN_CIRCUIT_LENGTH: usize = 1;

    /// Recommended circuit length for security
    pub const RECOMMENDED_CIRCUIT_LENGTH: usize = 3;

    /// Maximum circuit length
    pub const MAX_CIRCUIT_LENGTH: usize = 8;

    /// Circuit lifetime before rotation
    pub const CIRCUIT_LIFETIME_SECS: u64 = 600; // 10 minutes

    /// Maximum circuits per node
    pub const MAX_CIRCUITS: usize = 100;

    /// Number of parallel circuits for multi-path
    pub const MULTIPATH_CIRCUITS: usize = 3;
}

/// Credit system constants
pub mod credits {
    /// Initial credit balance for new nodes
    pub const INITIAL_BALANCE: u64 = 1000;

    /// Credits per GB relayed (base rate)
    pub const CREDITS_PER_GB: u64 = 1000;

    /// Minimum balance to send traffic
    pub const MIN_BALANCE_TO_SEND: u64 = 100;

    /// Block time for credit consensus (seconds)
    pub const BLOCK_TIME_SECS: u64 = 30;

    /// Maximum credits per transaction
    pub const MAX_TRANSACTION_AMOUNT: u64 = 1_000_000;
}

/// Consensus constants
pub mod consensus {
    /// Minimum number of validators
    pub const MIN_VALIDATORS: usize = 4;

    /// Target number of validators
    pub const TARGET_VALIDATORS: usize = 21;

    /// Consensus timeout
    pub const CONSENSUS_TIMEOUT_SECS: u64 = 10;

    /// Number of confirmations required
    pub const REQUIRED_CONFIRMATIONS: usize = 14; // 2/3 of 21
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Listen address
    pub listen_addr: String,

    /// Listen port
    pub listen_port: u16,

    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,

    /// Whether to accept relay traffic
    pub accept_relay: bool,

    /// Maximum number of peers to maintain
    pub max_peers: usize,

    /// Data directory
    pub data_dir: String,

    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: protocol::DEFAULT_PORT,
            bootstrap_nodes: vec![
                "37.114.50.194:9090".to_string(),
            ],
            accept_relay: true,
            max_peers: 50,
            data_dir: "./data".to_string(),
            verbose: false,
        }
    }
}

impl NodeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.listen_port = port;
        self
    }

    pub fn with_bootstrap_nodes(mut self, nodes: Vec<String>) -> Self {
        self.bootstrap_nodes = nodes;
        self
    }

    pub fn with_data_dir(mut self, dir: String) -> Self {
        self.data_dir = dir;
        self
    }

    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(protocol::CONNECTION_TIMEOUT_SECS)
    }

    pub fn keepalive_interval(&self) -> Duration {
        Duration::from_secs(protocol::KEEPALIVE_INTERVAL_SECS)
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError(e.to_string()))?;

        toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &PathBuf) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        std::fs::write(path, contents).map_err(|e| ConfigError::WriteError(e.to_string()))?;

        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(String),

    #[error("Failed to parse config: {0}")]
    ParseError(String),

    #[error("Failed to serialize config: {0}")]
    SerializeError(String),

    #[error("Failed to write config file: {0}")]
    WriteError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.listen_port, protocol::DEFAULT_PORT);
        assert!(config.accept_relay);
    }

    #[test]
    fn test_config_builder() {
        let config = NodeConfig::new()
            .with_port(8080)
            .with_bootstrap_nodes(vec!["node1:9090".to_string()])
            .with_data_dir("/tmp/data".to_string());

        assert_eq!(config.listen_port, 8080);
        assert_eq!(config.bootstrap_nodes.len(), 1);
        assert_eq!(config.data_dir, "/tmp/data");
    }
}
