mod health;

use axum::Router;

use crate::state::AppState;

/// Build the complete application router.
///
/// Structure:
/// - `GET /health` — lightweight health check (used by Railway)
/// - `GET /api/v1/health` — detailed health check with database connectivity
pub fn router() -> Router<AppState> {
    let api_v1 = Router::new().merge(health::api_router());

    Router::new()
        .merge(health::root_router())
        .nest("/api/v1", api_v1)
}
