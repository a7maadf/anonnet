/// Proxy services for routing traffic through AnonNet
///
/// This module provides SOCKS5 and HTTP proxy servers that allow
/// applications to route their traffic through the anonymous network.

mod socks5;
mod http;

pub use socks5::Socks5Server;
pub use http::HttpProxy;

use anonnet_core::Node;
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

/// Proxy manager that runs both SOCKS5 and HTTP proxies
pub struct ProxyManager {
    socks5_addr: SocketAddr,
    http_addr: SocketAddr,
    node: Arc<Node>,
}

impl ProxyManager {
    /// Create a new proxy manager with a reference to the Node
    pub fn new(socks5_addr: SocketAddr, http_addr: SocketAddr, node: Arc<Node>) -> Self {
        Self {
            socks5_addr,
            http_addr,
            node,
        }
    }

    /// Start all proxy servers
    pub async fn start(self) -> Result<()> {
        info!("Starting proxy services...");

        let socks5 = Socks5Server::new(self.socks5_addr, self.node.clone());
        let http = HttpProxy::new(self.http_addr, self.node.clone());

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
