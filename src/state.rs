use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::sessions::SessionManager;

/// Shared application state available to all request handlers via Axum's `State` extractor.
#[derive(Debug, Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub session_manager: SessionManager,
}
