/// Rate limiting for relay nodes
///
/// Implements token bucket and leaky bucket algorithms to prevent
/// abuse and ensure fair bandwidth allocation across the network.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::identity::NodeId;

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    /// Per-node rate limits
    node_limits: Arc<RwLock<HashMap<NodeId, TokenBucket>>>,

    /// Global rate limit configuration
    config: RateLimitConfig,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum tokens (bytes) a node can accumulate
    pub max_tokens: u64,

    /// Token refill rate (bytes per second)
    pub refill_rate: u64,

    /// Burst size (maximum instant transfer)
    pub burst_size: u64,

    /// Penalty for violations
    pub violation_penalty: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_tokens: 10 * 1024 * 1024,   // 10 MB
            refill_rate: 1024 * 1024,        // 1 MB/s
            burst_size: 5 * 1024 * 1024,     // 5 MB
            violation_penalty: Duration::from_secs(60),
        }
    }
}

/// Token bucket for a single node
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of tokens
    tokens: u64,

    /// Maximum tokens
    max_tokens: u64,

    /// Refill rate (tokens per second)
    refill_rate: u64,

    /// Last refill time
    last_refill: Instant,

    /// Penalty end time (if node is being penalized)
    penalty_until: Option<Instant>,

    /// Number of violations
    violations: u32,
}

impl TokenBucket {
    fn new(max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
            penalty_until: None,
            violations: 0,
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let new_tokens = (elapsed.as_secs() * self.refill_rate)
            + (elapsed.subsec_nanos() as u64 * self.refill_rate / 1_000_000_000);

        self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Try to consume tokens
    fn try_consume(&mut self, amount: u64) -> bool {
        // Check if penalized
        if let Some(penalty_until) = self.penalty_until {
            if Instant::now() < penalty_until {
                return false;
            } else {
                self.penalty_until = None;
            }
        }

        self.refill();

        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }

    /// Apply penalty for violation
    fn apply_penalty(&mut self, duration: Duration) {
        self.violations += 1;
        self.penalty_until = Some(Instant::now() + duration);
        self.tokens = 0; // Drain all tokens
    }
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            node_limits: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if a transfer is allowed
    pub async fn check_and_consume(
        &self,
        node_id: NodeId,
        bytes: u64,
    ) -> Result<(), RateLimitError> {
        let mut limits = self.node_limits.write().await;

        let bucket = limits.entry(node_id).or_insert_with(|| {
            TokenBucket::new(self.config.max_tokens, self.config.refill_rate)
        });

        // Check burst limit
        if bytes > self.config.burst_size {
            bucket.apply_penalty(self.config.violation_penalty);
            return Err(RateLimitError::BurstLimitExceeded);
        }

        // Try to consume tokens
        if bucket.try_consume(bytes) {
            Ok(())
        } else {
            Err(RateLimitError::RateLimitExceeded)
        }
    }

    /// Record a violation and apply penalty
    pub async fn record_violation(&self, node_id: NodeId) {
        let mut limits = self.node_limits.write().await;

        if let Some(bucket) = limits.get_mut(&node_id) {
            bucket.apply_penalty(self.config.violation_penalty);
        }
    }

    /// Get current rate limit status for a node
    pub async fn get_status(&self, node_id: &NodeId) -> Option<RateLimitStatus> {
        let limits = self.node_limits.read().await;

        limits.get(node_id).map(|bucket| {
            let mut status_bucket = bucket.clone();
            status_bucket.refill();

            RateLimitStatus {
                available_tokens: status_bucket.tokens,
                max_tokens: status_bucket.max_tokens,
                refill_rate: status_bucket.refill_rate,
                violations: status_bucket.violations,
                is_penalized: status_bucket
                    .penalty_until
                    .map(|t| Instant::now() < t)
                    .unwrap_or(false),
            }
        })
    }

    /// Reset rate limit for a node (admin function)
    pub async fn reset_node(&self, node_id: &NodeId) {
        let mut limits = self.node_limits.write().await;
        limits.remove(node_id);
    }

    /// Get statistics for all nodes
    pub async fn get_stats(&self) -> RateLimitStats {
        let limits = self.node_limits.read().await;

        let total_nodes = limits.len();
        let penalized_nodes = limits
            .values()
            .filter(|b| {
                b.penalty_until
                    .map(|t| Instant::now() < t)
                    .unwrap_or(false)
            })
            .count();

        let total_violations: u32 = limits.values().map(|b| b.violations).sum();

        RateLimitStats {
            total_nodes,
            penalized_nodes,
            total_violations,
        }
    }
}

/// Rate limit status for a node
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub available_tokens: u64,
    pub max_tokens: u64,
    pub refill_rate: u64,
    pub violations: u32,
    pub is_penalized: bool,
}

/// Overall rate limiting statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub total_nodes: usize,
    pub penalized_nodes: usize,
    pub total_violations: u32,
}

/// Rate limiting errors
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded - too many requests")]
    RateLimitExceeded,

    #[error("Burst limit exceeded - request too large")]
    BurstLimitExceeded,

    #[error("Node is currently penalized")]
    NodePenalized,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        let stats = limiter.get_stats().await;

        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.penalized_nodes, 0);
    }

    #[tokio::test]
    async fn test_rate_limit_consumption() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        // Should allow initial transfer
        let result = limiter.check_and_consume(node_id, 1024).await;
        assert!(result.is_ok());

        // Should have consumed tokens
        let status = limiter.get_status(&node_id).await;
        assert!(status.is_some());
    }

    #[tokio::test]
    async fn test_burst_limit() {
        let mut config = RateLimitConfig::default();
        config.burst_size = 1024; // Small burst limit

        let limiter = RateLimiter::new(config);
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        // Should reject transfer larger than burst
        let result = limiter.check_and_consume(node_id, 2048).await;
        assert!(matches!(result, Err(RateLimitError::BurstLimitExceeded)));

        // Should be penalized
        let status = limiter.get_status(&node_id).await.unwrap();
        assert!(status.is_penalized);
    }

    #[tokio::test]
    async fn test_token_refill() {
        let mut config = RateLimitConfig::default();
        config.max_tokens = 1000;
        config.refill_rate = 1000; // 1000 tokens/second

        let limiter = RateLimiter::new(config);
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        // Consume all tokens
        limiter.check_and_consume(node_id, 1000).await.unwrap();

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Should have refilled
        let result = limiter.check_and_consume(node_id, 500).await;
        assert!(result.is_ok());
    }
}
