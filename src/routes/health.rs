use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde::Serialize;

use crate::error::AppError;
use crate::state::AppState;

/// Root-level health check router: `GET /health`.
///
/// Used by Railway's health check probe. Lightweight — does not touch the database.
pub fn root_router() -> Router<AppState> {
    Router::new().route("/health", get(health_simple))
}

/// API-level health check router: `GET /api/v1/health`.
///
/// Returns server status, version, and database connectivity.
pub fn api_router() -> Router<AppState> {
    Router::new().route("/health", get(health_detailed))
}

#[derive(Serialize)]
struct SimpleHealth {
    status: &'static str,
    version: &'static str,
}

/// Lightweight health check — confirms the server process is alive.
async fn health_simple() -> Json<SimpleHealth> {
    Json(SimpleHealth {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[derive(Serialize)]
struct DetailedHealth {
    status: &'static str,
    version: &'static str,
    database: DatabaseStatus,
}

#[derive(Serialize)]
struct DatabaseStatus {
    connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Detailed health check — verifies database connectivity with a `SELECT 1` query.
async fn health_detailed(State(state): State<AppState>) -> Result<Json<DetailedHealth>, AppError> {
    let start = std::time::Instant::now();

    let db_result = state
        .db
        .execute(Statement::from_string(
            DbBackend::Postgres,
            "SELECT 1".to_string(),
        ))
        .await;

    let elapsed = start.elapsed();

    let database = match db_result {
        Ok(_) => DatabaseStatus {
            connected: true,
            latency_ms: Some(elapsed.as_millis()),
            error: None,
        },
        Err(e) => {
            tracing::warn!("Health check database ping failed: {e}");
            DatabaseStatus {
                connected: false,
                latency_ms: None,
                error: Some(e.to_string()),
            }
        }
    };

    Ok(Json(DetailedHealth {
        status: if database.connected {
            "healthy"
        } else {
            "degraded"
        },
        version: env!("CARGO_PKG_VERSION"),
        database,
    }))
}
