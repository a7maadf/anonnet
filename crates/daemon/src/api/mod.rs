/// REST API module for AnonNet daemon
///
/// Provides HTTP endpoints for browser extension and other clients to query:
/// - Credit balance and statistics
/// - Network status (peers, circuits)
/// - Bandwidth usage
/// - Active circuit information

pub mod handlers;
pub mod responses;
pub mod server;

pub use server::ApiServer;
pub use responses::*;
