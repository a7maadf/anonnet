/// Proxy services for routing traffic through AnonNet
///
/// This module provides SOCKS5 and HTTP proxy servers that allow
/// applications to route their traffic through the anonymous network.

mod socks5;
mod http;

pub use socks5::Socks5Server;
pub use http::HttpProxy;

use anyhow::Result;
use std::net::SocketAddr;
use tracing::info;

/// Proxy manager that runs both SOCKS5 and HTTP proxies
pub struct ProxyManager {
    socks5_addr: SocketAddr,
    http_addr: SocketAddr,
}

impl ProxyManager {
    /// Create a new proxy manager
    pub fn new(socks5_addr: SocketAddr, http_addr: SocketAddr) -> Self {
        Self {
            socks5_addr,
            http_addr,
        }
    }

    /// Start all proxy servers
    pub async fn start(self) -> Result<()> {
        info!("Starting proxy services...");

        let socks5 = Socks5Server::new(self.socks5_addr);
        let http = HttpProxy::new(self.http_addr);

        // Run both proxies concurrently
        tokio::select! {
            result = socks5.start() => {
                result?;
            }
            result = http.start() => {
                result?;
            }
        }

        Ok(())
    }
}
