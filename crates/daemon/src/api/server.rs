/// API Server implementation

use super::handlers::*;
use anonnet_core::Node;
use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

/// API Server for exposing daemon status and credit information
pub struct ApiServer {
    listen_addr: SocketAddr,
    node: Arc<Node>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(listen_addr: SocketAddr, node: Arc<Node>) -> Self {
        Self { listen_addr, node }
    }

    /// Start the API server
    pub async fn start(self) -> Result<()> {
        let state = AppState {
            node: self.node.clone(),
        };

        // Build the router with all endpoints
        let app = Router::new()
            // Health check
            .route("/health", get(health_check))
            // Credit endpoints
            .route("/api/credits/balance", get(get_credit_balance))
            .route("/api/credits/stats", get(get_credit_stats))
            // Network endpoints
            .route("/api/network/status", get(get_network_status))
            .route("/api/circuits/active", get(get_active_circuits))
            // Service endpoints (.anon service hosting)
            .route("/api/services/register", post(register_service))
            .route("/api/services/list", get(list_services))
            // Add CORS middleware (allow browser extensions)
            .layer(CorsLayer::permissive())
            // Add shared state
            .with_state(state);

        // Bind to the address (port 0 = let OS choose a free port)
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;
        let actual_addr = listener.local_addr()?;

        info!("╔═══════════════════════════════════════════════════╗");
        info!("║   API Server Started on {}   ║", actual_addr);
        info!("╚═══════════════════════════════════════════════════╝");

        // Save port to file for extension to discover
        if let Err(e) = std::fs::write("./data/api_port.txt", actual_addr.port().to_string()) {
            tracing::warn!("Failed to write API port file: {}", e);
        }

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("API server error: {}", e))?;

        Ok(())
    }
}
