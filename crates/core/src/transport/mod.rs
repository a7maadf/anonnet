/// Transport layer for AnonNet using QUIC
///
/// Provides fast, encrypted, multiplexed connections with:
/// - 0-RTT reconnections
/// - UDP-based performance
/// - Stream multiplexing
/// - Built-in encryption

mod connection;
mod endpoint;
mod stream;

pub use connection::{Connection, ConnectionError, ConnectionStats};
pub use endpoint::{Endpoint, EndpointConfig, EndpointError};
pub use stream::{RecvStream, SendStream, StreamError};
