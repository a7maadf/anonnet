use thiserror::Error;

/// Common error types for AnonNet
#[derive(Debug, Error)]
pub enum AnonNetError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Insufficient credits: need {needed}, have {available}")]
    InsufficientCredits { needed: u64, available: u64 },

    #[error("Invalid node ID: {0}")]
    InvalidNodeId(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("Connection refused")]
    ConnectionRefused,

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Circuit creation failed: {0}")]
    CircuitCreationFailed(String),

    #[error("Relay failed: {0}")]
    RelayFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for AnonNet operations
pub type Result<T> = std::result::Result<T, AnonNetError>;

impl AnonNetError {
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    pub fn invalid_node_id(msg: impl Into<String>) -> Self {
        Self::InvalidNodeId(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}
