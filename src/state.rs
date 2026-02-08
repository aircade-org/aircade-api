use sea_orm::DatabaseConnection;

use crate::config::Config;

/// Shared application state available to all request handlers via Axum's `State` extractor.
#[derive(Debug, Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
}
