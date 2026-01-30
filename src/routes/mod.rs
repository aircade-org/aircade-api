mod auth;
pub mod games;
mod health;
mod sessions;
mod users;

use axum::Router;

use crate::state::AppState;

/// Build the complete application router.
///
/// Structure:
/// - `GET /health` — lightweight health check (used by Railway)
/// - `GET /api/v1/health` — detailed health check with database connectivity
/// - `/api/v1/auth/...` — authentication endpoints
/// - `/api/v1/users/...` — user profile and management endpoints
/// - `/api/v1/games/...` — game management endpoints
/// - `/api/v1/tags` — platform tag listing
/// - `/api/v1/sessions/...` — game session management and `WebSocket` relay
pub fn router() -> Router<AppState> {
    let api_v1 = Router::new()
        .merge(health::api_router())
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/games", games::router())
        .nest("/tags", games::tags_router())
        .nest("/sessions", sessions::router());

    Router::new()
        .merge(health::root_router())
        .nest("/api/v1", api_v1)
}
