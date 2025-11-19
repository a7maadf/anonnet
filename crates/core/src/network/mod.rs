/// Network performance and monitoring
///
/// This module provides tools for monitoring network performance,
/// estimating bandwidth, tracking relay statistics, and rate limiting.

pub mod bandwidth;
pub mod rate_limit;
pub mod message_handler;
pub mod connection_manager;
pub mod message_dispatcher;

pub use bandwidth::{
    BandwidthConfig, BandwidthEstimator, NetworkBandwidthStats, NodeBandwidthStats,
};
pub use rate_limit::{RateLimitConfig, RateLimitError, RateLimiter, RateLimitStats, RateLimitStatus};
pub use message_handler::{ConnectionHandler, MessageCodec};
pub use connection_manager::{ConnectionManager, PeerConnection};
pub use message_dispatcher::MessageDispatcher;
