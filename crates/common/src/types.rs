use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Credit amount for the network economy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Credits(pub u64);

impl Credits {
    pub const ZERO: Credits = Credits(0);
    pub const INITIAL_BALANCE: Credits = Credits(1000);

    pub fn new(amount: u64) -> Self {
        Self(amount)
    }

    pub fn amount(&self) -> u64 {
        self.0
    }

    pub fn checked_add(&self, other: Credits) -> Option<Credits> {
        self.0.checked_add(other.0).map(Credits)
    }

    pub fn checked_sub(&self, other: Credits) -> Option<Credits> {
        self.0.checked_sub(other.0).map(Credits)
    }

    pub fn saturating_add(&self, other: Credits) -> Credits {
        Credits(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(&self, other: Credits) -> Credits {
        Credits(self.0.saturating_sub(other.0))
    }
}

impl std::fmt::Display for Credits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} credits", self.0)
    }
}

impl std::ops::Add for Credits {
    type Output = Credits;

    fn add(self, other: Credits) -> Credits {
        Credits(self.0 + other.0)
    }
}

impl std::ops::Sub for Credits {
    type Output = Credits;

    fn sub(self, other: Credits) -> Credits {
        Credits(self.0 - other.0)
    }
}

/// Timestamp in Unix epoch seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before Unix epoch");
        Self(duration.as_secs())
    }

    pub fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    pub fn as_secs(&self) -> u64 {
        self.0
    }

    pub fn elapsed(&self) -> Duration {
        let now = Self::now();
        Duration::from_secs(now.0.saturating_sub(self.0))
    }
}

/// Network address for peer connections
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkAddress {
    /// Standard socket address (IP + port)
    Socket(SocketAddr),
    /// Domain name + port (for DNS-based connections)
    Domain { host: String, port: u16 },
}

impl NetworkAddress {
    pub fn from_socket(addr: SocketAddr) -> Self {
        Self::Socket(addr)
    }

    pub fn from_domain(host: String, port: u16) -> Self {
        Self::Domain { host, port }
    }
}

impl std::fmt::Display for NetworkAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Socket(addr) => write!(f, "{}", addr),
            Self::Domain { host, port } => write!(f, "{}:{}", host, port),
        }
    }
}

/// Bandwidth amount in bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Bandwidth(pub u64);

impl Bandwidth {
    pub fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    pub fn from_kb(kb: u64) -> Self {
        Self(kb * 1024)
    }

    pub fn from_mb(mb: u64) -> Self {
        Self(mb * 1024 * 1024)
    }

    pub fn from_gb(gb: u64) -> Self {
        Self(gb * 1024 * 1024 * 1024)
    }

    pub fn as_bytes(&self) -> u64 {
        self.0
    }

    pub fn as_kb(&self) -> f64 {
        self.0 as f64 / 1024.0
    }

    pub fn as_mb(&self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0)
    }

    pub fn as_gb(&self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

impl std::fmt::Display for Bandwidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 1024 {
            write!(f, "{} B", self.0)
        } else if self.0 < 1024 * 1024 {
            write!(f, "{:.2} KB", self.as_kb())
        } else if self.0 < 1024 * 1024 * 1024 {
            write!(f, "{:.2} MB", self.as_mb())
        } else {
            write!(f, "{:.2} GB", self.as_gb())
        }
    }
}

impl std::ops::Add for Bandwidth {
    type Output = Bandwidth;

    fn add(self, other: Bandwidth) -> Bandwidth {
        Bandwidth(self.0 + other.0)
    }
}

impl std::ops::Sub for Bandwidth {
    type Output = Bandwidth;

    fn sub(self, other: Bandwidth) -> Bandwidth {
        Bandwidth(self.0 - other.0)
    }
}

/// Reputation score for a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Reputation(pub u32);

impl Reputation {
    pub const MIN: Reputation = Reputation(0);
    pub const MAX: Reputation = Reputation(10000);
    pub const INITIAL: Reputation = Reputation(100);

    pub fn new(score: u32) -> Self {
        Self(score.min(Self::MAX.0))
    }

    pub fn score(&self) -> u32 {
        self.0
    }

    pub fn increase(&mut self, amount: u32) {
        self.0 = self.0.saturating_add(amount).min(Self::MAX.0);
    }

    pub fn decrease(&mut self, amount: u32) {
        self.0 = self.0.saturating_sub(amount);
    }
}

impl std::fmt::Display for Reputation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/10000", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credits_arithmetic() {
        let c1 = Credits::new(100);
        let c2 = Credits::new(50);

        assert_eq!(c1 + c2, Credits::new(150));
        assert_eq!(c1 - c2, Credits::new(50));
        assert_eq!(c1.checked_sub(Credits::new(200)), None);
    }

    #[test]
    fn test_bandwidth_conversion() {
        let bw = Bandwidth::from_mb(10);
        assert_eq!(bw.as_bytes(), 10 * 1024 * 1024);
        assert_eq!(bw.as_mb(), 10.0);
    }

    #[test]
    fn test_reputation_bounds() {
        let mut rep = Reputation::new(100);
        rep.increase(20000); // Try to go over max
        assert_eq!(rep.score(), Reputation::MAX.0);

        rep.decrease(20000); // Try to go under min
        assert_eq!(rep.score(), 0);
    }

    #[test]
    fn test_timestamp() {
        let ts1 = Timestamp::now();
        let ts2 = Timestamp::from_secs(ts1.as_secs() - 10);
        let elapsed = ts2.elapsed();
        assert!(elapsed.as_secs() >= 10);
    }
}
