pub mod circuit;
pub mod consensus;
pub mod dht;
pub mod identity;
pub mod peer;
pub mod protocol;

pub use identity::{
    Distance, ExportableIdentity, Identity, KeyPair, KeyPairError, NodeId, NodeIdError, PublicKey,
};
pub use protocol::*;

// Re-export circuit types
pub use circuit::{
    Circuit, CircuitCleanupStats, CircuitId, CircuitManager, CircuitManagerStats, CircuitNode,
    CircuitPurpose, CircuitState, CryptoError, OnionCrypto, PathSelectionCriteria,
    PathSelectionError, PathSelector, RelayAction, RelayCell, RelayCellType, RelayError,
    RelayHandler,
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
