pub mod peer_discovery {
    use anonnet_common::NodeId;
    use serde::{Deserialize, Serialize};

    use crate::CoreError;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct PeerInfo {
        pub id: NodeId,
        pub addresses: Vec<String>,
        pub reputation: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct DiscoveryQuery {
        pub target_reputation: Option<u32>,
        pub limit: usize,
    }

    pub trait PeerDiscovery: Send + Sync {
        fn local_peer(&self) -> NodeId;
        fn list_peers(&self) -> Vec<PeerInfo>;
        fn discover(&self, query: DiscoveryQuery) -> Result<Vec<PeerInfo>, CoreError>;
    }

    #[derive(Debug, Default)]
    pub struct DefaultPeerDiscovery;

    impl PeerDiscovery for DefaultPeerDiscovery {
        fn local_peer(&self) -> NodeId {
            todo!("peer discovery local_peer")
        }

        fn list_peers(&self) -> Vec<PeerInfo> {
            todo!("peer discovery list_peers")
        }

        fn discover(&self, _query: DiscoveryQuery) -> Result<Vec<PeerInfo>, CoreError> {
            todo!("peer discovery discover")
        }
    }
}

pub mod circuit {
    use anonnet_common::NodeId;
    use serde::{Deserialize, Serialize};

    use crate::CoreError;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct CircuitRequest {
        pub path_length: usize,
        pub entry_guard: Option<NodeId>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct CircuitHandle {
        pub id: u64,
        pub path: Vec<NodeId>,
    }

    pub trait CircuitBuilder: Send + Sync {
        fn build_circuit(&self, request: CircuitRequest) -> Result<CircuitHandle, CoreError>;
        fn refresh_circuit(&self, handle: &CircuitHandle) -> Result<CircuitHandle, CoreError>;
        fn teardown_circuit(&self, handle: CircuitHandle) -> Result<(), CoreError>;
    }

    #[derive(Debug, Default)]
    pub struct DefaultCircuitBuilder;

    impl CircuitBuilder for DefaultCircuitBuilder {
        fn build_circuit(&self, _request: CircuitRequest) -> Result<CircuitHandle, CoreError> {
            todo!("circuit builder build_circuit")
        }

        fn refresh_circuit(&self, _handle: &CircuitHandle) -> Result<CircuitHandle, CoreError> {
            todo!("circuit builder refresh_circuit")
        }

        fn teardown_circuit(&self, _handle: CircuitHandle) -> Result<(), CoreError> {
            todo!("circuit builder teardown_circuit")
        }
    }
}

pub mod credit {
    use anonnet_common::{CreditBalance, MessageEnvelope, NodeId};
    use serde::{Deserialize, Serialize};

    use crate::CoreError;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct CreditTransaction {
        pub from: NodeId,
        pub to: NodeId,
        pub amount: u64,
        pub proof: MessageEnvelope,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct LedgerSnapshot {
        pub entries: Vec<(NodeId, CreditBalance)>,
    }

    pub trait CreditLedger: Send + Sync {
        fn balance(&self, node: &NodeId) -> Result<CreditBalance, CoreError>;
        fn apply(&self, tx: CreditTransaction) -> Result<(), CoreError>;
        fn snapshot(&self) -> Result<LedgerSnapshot, CoreError>;
    }

    #[derive(Debug, Default)]
    pub struct DefaultCreditLedger;

    impl CreditLedger for DefaultCreditLedger {
        fn balance(&self, _node: &NodeId) -> Result<CreditBalance, CoreError> {
            todo!("credit ledger balance")
        }

        fn apply(&self, _tx: CreditTransaction) -> Result<(), CoreError> {
            todo!("credit ledger apply")
        }

        fn snapshot(&self) -> Result<LedgerSnapshot, CoreError> {
            todo!("credit ledger snapshot")
        }
    }
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("domain error: {0}")]
    Domain(#[from] anonnet_common::DomainError),
    #[error("network not initialized: {0}")]
    NotReady(&'static str),
    #[error("feature not implemented: {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use super::{circuit::DefaultCircuitBuilder, credit::DefaultCreditLedger, peer_discovery::DefaultPeerDiscovery};
    use super::{circuit::CircuitBuilder, credit::CreditLedger, peer_discovery::PeerDiscovery};

    fn assert_peer_discovery<T: PeerDiscovery>() {}
    fn assert_circuit_builder<T: CircuitBuilder>() {}
    fn assert_credit_ledger<T: CreditLedger>() {}

    #[test]
    fn default_structs_compile() {
        assert_peer_discovery::<DefaultPeerDiscovery>();
        assert_circuit_builder::<DefaultCircuitBuilder>();
        assert_credit_ledger::<DefaultCreditLedger>();
    }
}
