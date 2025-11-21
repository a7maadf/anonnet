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
    let circuits = state.node.get_active_circuits().await;

    // Calculate average circuit hops
    let total_hops: usize = circuits.iter().map(|c| c.hops).sum();
    let average_hops = if circuits.is_empty() {
        0.0
    } else {
        total_hops as f32 / circuits.len() as f32
    };

    // Check for insecure circuits (< 3 hops)
    let insecure = circuits.iter().any(|c| c.hops < 3);
    let security_warning = if insecure {
        Some("Network still growing: Using reduced-hop circuits. Anonymity may be limited.".to_string())
    } else {
        None
    };

    Ok(Json(NetworkStatusResponse {
        node_id: stats.node_id.to_string(),
        is_running: stats.is_running,
        peer_count: stats.peer_count,
        active_peers: stats.active_peers,
        total_circuits: stats.circuits,
        active_circuits: stats.active_circuits,
        bandwidth: stats.bandwidth,
        average_circuit_hops: average_hops,
        insecure_circuits: insecure,
        security_warning,
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

/// Handler for POST /api/services/register
pub async fn register_service(
    State(state): State<AppState>,
    Json(req): Json<ServiceRegistrationRequest>,
) -> Result<Json<ServiceRegistrationResponse>, AppError> {
    debug!(
        "API: POST /api/services/register ({}:{})",
        req.local_host, req.local_port
    );

    // Validate inputs
    if req.ttl_hours < 1 || req.ttl_hours > 24 {
        return Err(AppError::internal("TTL must be between 1 and 24 hours"));
    }

    if req.local_port == 0 {
        return Err(AppError::internal("Invalid port number"));
    }

    // Register the service
    let (address, _keypair) = state
        .node
        .register_service(req.local_host, req.local_port, req.ttl_hours)
        .await?;

    // Calculate expiry timestamp
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + (req.ttl_hours * 3600);

    // Get intro point count from the descriptor
    let descriptors = state.node.get_published_services().await;
    let intro_points = descriptors
        .iter()
        .find(|d| d.address == address)
        .map(|d| d.introduction_points.len())
        .unwrap_or(0);

    Ok(Json(ServiceRegistrationResponse {
        anon_address: address.to_hostname(),
        public_key: hex::encode(address.as_bytes()),
        intro_points,
        expires_at,
    }))
}

/// Handler for GET /api/services/list
pub async fn list_services(
    State(state): State<AppState>,
) -> Result<Json<ServiceListResponse>, AppError> {
    debug!("API: GET /api/services/list");

    let descriptors = state.node.get_published_services().await;

    let services: Vec<ServiceInfo> = descriptors
        .into_iter()
        .map(|d| {
            let created_at = d
                .created_at
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            ServiceInfo {
                anon_address: d.address.to_hostname(),
                public_key: hex::encode(d.public_key.as_bytes()),
                intro_points: d.introduction_points.len(),
                created_at,
                ttl_seconds: d.ttl.as_secs(),
                is_expired: d.is_expired(),
            }
        })
        .collect();

    let total = services.len();

    Ok(Json(ServiceListResponse { services, total }))
}
