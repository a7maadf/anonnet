pub mod circuit;
pub mod consensus;
pub mod dht;
pub mod identity;
pub mod network;
pub mod node;
pub mod peer;
pub mod protocol;
pub mod service;
pub mod transport;

pub use identity::{
    Distance, ExportableIdentity, Identity, KeyPair, KeyPairError, NodeId, NodeIdError,
    ProofOfWork, PublicKey,
};
pub use protocol::*;

// Re-export circuit types
pub use circuit::{
    Circuit, CircuitCleanupStats, CircuitId, CircuitManager, CircuitManagerStats, CircuitNode,
    CircuitPool, CircuitPoolConfig, CircuitPoolError, CircuitPoolStats, CircuitPurpose,
    CircuitState, CryptoError, OnionCrypto, PathSelectionCriteria, PathSelectionError,
    PathSelector, PoolStats, RelayAction, RelayCell, RelayCellType, RelayError, RelayHandler,
};

// Re-export DHT types
pub use dht::{
    BootstrapNode, BootstrapState, BucketEntry, DHTStats, InsertError, InsertResult,
    LookupManager, MaintenanceActions, NodeLookup, RoutingTable, RoutingTableStats, DHT,
};

// Re-export peer types
pub use peer::{PeerConnection, PeerManager, PeerManagerStats, PeerState};

// Re-export consensus types
pub use consensus::{
    Block, BlockError, BlockHeader, Blockchain, CreditLedger, RelayProof, Transaction,
    TransactionError, TransactionId, TransactionType, TransactionValidator, Validator,
    ValidatorError, ValidatorSet,
};

// Re-export transport types
pub use transport::{
    Connection, ConnectionError, ConnectionStats, Endpoint, EndpointConfig, EndpointError,
    RecvStream, SendStream, StreamError,
};

// Re-export service types
pub use service::{
    ConnectionInfo, DescriptorError, DirectoryError, IntroductionPoint, RendezvousError,
    RendezvousId, RendezvousManager, ServiceAddress, ServiceAddressError, ServiceDescriptor,
    ServiceDirectory,
};

// Re-export network types
pub use network::{
    BandwidthConfig, BandwidthEstimator, NetworkBandwidthStats, NodeBandwidthStats,
    RateLimitConfig, RateLimitError, RateLimiter, RateLimitStats, RateLimitStatus,
};

// Re-export node runtime
pub use node::{Node, NodeStats};
