use crate::types::{Credits, NetworkAddress, Reputation, Timestamp};
use serde::{Deserialize, Serialize};

/// Information about a peer node in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Network addresses where the node can be reached
    pub addresses: Vec<NetworkAddress>,

    /// When this node info was last updated
    pub last_seen: Timestamp,

    /// Node's reputation score
    pub reputation: Reputation,

    /// Estimated latency to this node (in milliseconds)
    pub latency_ms: Option<u32>,

    /// Whether this node accepts relay traffic
    pub accepts_relay: bool,

    /// Node's advertised bandwidth capacity
    pub bandwidth_capacity: Option<u64>,

    /// Protocol version the node is running
    pub protocol_version: u32,
}

impl NodeInfo {
    pub fn new(address: NetworkAddress) -> Self {
        Self {
            addresses: vec![address],
            last_seen: Timestamp::now(),
            reputation: Reputation::INITIAL,
            latency_ms: None,
            accepts_relay: true,
            bandwidth_capacity: None,
            protocol_version: 1,
        }
    }

    pub fn with_addresses(addresses: Vec<NetworkAddress>) -> Self {
        Self {
            addresses,
            last_seen: Timestamp::now(),
            reputation: Reputation::INITIAL,
            latency_ms: None,
            accepts_relay: true,
            bandwidth_capacity: None,
            protocol_version: 1,
        }
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = Timestamp::now();
    }

    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        self.last_seen.elapsed().as_secs() > max_age_secs
    }

    pub fn add_address(&mut self, address: NetworkAddress) {
        if !self.addresses.contains(&address) {
            self.addresses.push(address);
        }
    }
}

/// Account balance and statistics for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Current credit balance
    pub balance: Credits,

    /// Total credits earned by relaying
    pub total_earned: Credits,

    /// Total credits spent on routing
    pub total_spent: Credits,

    /// Total bandwidth relayed (in bytes)
    pub total_relayed: u64,

    /// Total bandwidth sent (in bytes)
    pub total_sent: u64,

    /// Account creation timestamp
    pub created_at: Timestamp,

    /// Last transaction timestamp
    pub last_transaction: Timestamp,
}

impl AccountInfo {
    pub fn new() -> Self {
        Self {
            balance: Credits::INITIAL_BALANCE,
            total_earned: Credits::ZERO,
            total_spent: Credits::ZERO,
            total_relayed: 0,
            total_sent: 0,
            created_at: Timestamp::now(),
            last_transaction: Timestamp::now(),
        }
    }

    pub fn can_spend(&self, amount: Credits) -> bool {
        self.balance >= amount
    }

    pub fn spend(&mut self, amount: Credits) -> Result<(), &'static str> {
        if !self.can_spend(amount) {
            return Err("Insufficient balance");
        }

        self.balance = self.balance.saturating_sub(amount);
        self.total_spent = self.total_spent.saturating_add(amount);
        self.last_transaction = Timestamp::now();

        Ok(())
    }

    pub fn earn(&mut self, amount: Credits) {
        self.balance = self.balance.saturating_add(amount);
        self.total_earned = self.total_earned.saturating_add(amount);
        self.last_transaction = Timestamp::now();
    }

    pub fn net_contribution(&self) -> i64 {
        self.total_earned.amount() as i64 - self.total_spent.amount() as i64
    }
}

impl Default for AccountInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn test_node_info_creation() {
        let addr = NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));
        let info = NodeInfo::new(addr);

        assert_eq!(info.addresses.len(), 1);
        assert_eq!(info.reputation, Reputation::INITIAL);
    }

    #[test]
    fn test_node_info_stale() {
        let mut info = NodeInfo::new(NetworkAddress::from_socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        )));

        // Fresh info should not be stale
        assert!(!info.is_stale(3600));

        // Simulate old timestamp
        info.last_seen = Timestamp::from_secs(0);
        assert!(info.is_stale(3600));
    }

    #[test]
    fn test_account_spend_earn() {
        let mut account = AccountInfo::new();
        let initial = account.balance;

        // Earn credits
        account.earn(Credits::new(500));
        assert_eq!(account.balance, initial + Credits::new(500));
        assert_eq!(account.total_earned, Credits::new(500));

        // Spend credits
        account.spend(Credits::new(200)).unwrap();
        assert_eq!(account.balance, initial + Credits::new(300));
        assert_eq!(account.total_spent, Credits::new(200));

        // Cannot overspend
        let result = account.spend(Credits::new(100000));
        assert!(result.is_err());
    }

    #[test]
    fn test_net_contribution() {
        let mut account = AccountInfo::new();

        account.earn(Credits::new(1000));
        account.spend(Credits::new(300)).unwrap();

        assert_eq!(account.net_contribution(), 700);
    }
}
