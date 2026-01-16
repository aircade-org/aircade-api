use crate::api::AppState;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    database: String,
}

/// Health check endpoint handler
async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Check database connectivity
    let db_lock = state.db.read().await.to_owned();
    let db_status = match db_lock.as_ref() {
        Some(db) => match db.ping().await {
            Ok(()) => "connected",
            Err(_) => "disconnected",
        },
        None => "connecting",
    };

    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status.to_string(),
    };

    (StatusCode::OK, Json(response))
}

/// Register health check routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health_check))
}
