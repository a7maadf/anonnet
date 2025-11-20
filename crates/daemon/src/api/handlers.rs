/// API request handlers

use super::responses::*;
use anonnet_core::Node;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tracing::{debug, error};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub node: Arc<Node>,
}

/// Handler for GET /api/credits/balance
pub async fn get_credit_balance(
    State(state): State<AppState>,
) -> Result<Json<CreditBalanceResponse>, AppError> {
    debug!("API: GET /api/credits/balance");

    let node_id = state.node.node_id();
    let balance = state.node.get_credit_balance().await;

    Ok(Json(CreditBalanceResponse {
        balance: balance.amount(),
        node_id: node_id.to_string(),
    }))
}

/// Handler for GET /api/credits/stats
pub async fn get_credit_stats(
    State(state): State<AppState>,
) -> Result<Json<CreditStatsResponse>, AppError> {
    debug!("API: GET /api/credits/stats");

    let balance = state.node.get_credit_balance().await;
    let stats = state.node.get_credit_stats().await;

    Ok(Json(CreditStatsResponse {
        balance: balance.amount(),
        total_earned: stats.total_earned,
        total_spent: stats.total_spent,
        earning_rate: stats.earning_rate,
        spending_rate: stats.spending_rate,
    }))
}

/// Handler for GET /api/network/status
pub async fn get_network_status(
    State(state): State<AppState>,
) -> Result<Json<NetworkStatusResponse>, AppError> {
    debug!("API: GET /api/network/status");

    let stats = state.node.get_stats().await;

    Ok(Json(NetworkStatusResponse {
        node_id: stats.node_id.to_string(),
        is_running: stats.is_running,
        peer_count: stats.peer_count,
        active_peers: stats.active_peers,
        total_circuits: stats.circuits,
        active_circuits: stats.active_circuits,
        bandwidth: stats.bandwidth,
    }))
}

/// Handler for GET /api/circuits/active
pub async fn get_active_circuits(
    State(state): State<AppState>,
) -> Result<Json<ActiveCircuitsResponse>, AppError> {
    debug!("API: GET /api/circuits/active");

    let circuits = state.node.get_active_circuits().await;

    let circuit_infos: Vec<CircuitInfo> = circuits
        .into_iter()
        .map(|c| CircuitInfo {
            circuit_id: c.circuit_id.to_string(),
            purpose: format!("{:?}", c.purpose),
            state: format!("{:?}", c.state),
            hops: c.hops,
            age_seconds: c.age_seconds,
            use_count: c.use_count,
        })
        .collect();

    let total = circuit_infos.len();

    Ok(Json(ActiveCircuitsResponse {
        circuits: circuit_infos,
        total,
    }))
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    debug!("API: GET /health");
    (StatusCode::OK, "OK")
}

/// Application error type
pub struct AppError {
    message: String,
    status_code: StatusCode,
}

impl AppError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::NOT_FOUND,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("API Error: {}", self.message);

        let body = Json(ErrorResponse::new(
            self.message,
            self.status_code.as_u16(),
        ));

        (self.status_code, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::internal(err.to_string())
    }
}
