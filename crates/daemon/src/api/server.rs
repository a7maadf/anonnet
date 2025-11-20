/// API Server implementation

use super::handlers::*;
use anonnet_core::Node;
use anyhow::Result;
use axum::{
    routing::get,
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
            // Add CORS middleware (allow browser extensions)
            .layer(CorsLayer::permissive())
            // Add shared state
            .with_state(state);

        info!("API server starting on {}", self.listen_addr);

        // Start the server
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("API server error: {}", e))?;

        Ok(())
    }
}
