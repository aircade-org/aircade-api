mod common;

use axum::Router;
use axum::http::StatusCode;

use aircade_api::config::{Config, Environment};
use aircade_api::state::AppState;

/// Build the app router backed by an in-memory `SQLite` database.
/// This avoids needing `PostgreSQL` for simple route tests.
async fn test_app() -> Router {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap_or_default();

    let state = AppState {
        db,
        config: Config {
            database_url: String::new(),
            server_host: std::net::IpAddr::from([127, 0, 0, 1]),
            server_port: 0,
            environment: Environment::Development,
            log_level: "warn".to_string(),
        },
    };

    aircade_api::routes::router().with_state(state)
}

#[tokio::test]
async fn health_root_returns_200() {
    let app = test_app().await;
    let (status, body) = common::get(&app, "/health").await;

    assert_eq!(status, StatusCode::OK);

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn health_api_returns_200() {
    let app = test_app().await;
    let (status, body) = common::get(&app, "/api/v1/health").await;

    assert_eq!(status, StatusCode::OK);

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    // SQLite in-memory should report connected
    assert_eq!(json["database"]["connected"], true);
    assert!(json["database"]["latency_ms"].is_number());
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = test_app().await;
    let (status, _body) = common::get(&app, "/nonexistent").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}
