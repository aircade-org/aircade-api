use std::time::Duration;

use axum::http::{header, Method, Request};
use axum::response::Response;
use axum::Router;
use migration::{Migrator, MigratorTrait};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use aircade_api::config::{Config, Environment};
use aircade_api::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize structured logging
    init_tracing(&config.log_level);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        environment = ?config.environment,
        "Starting AirCade API"
    );

    // Connect to database
    tracing::info!("Connecting to database...");
    let db = aircade_api::db::connect(&config.database_url).await?;
    tracing::info!("Database connected");

    // Run migrations
    tracing::info!("Running database migrations...");
    Migrator::up(&db, None).await?;
    tracing::info!("Migrations applied");

    // Build application state
    let state = AppState {
        db,
        config: config.clone(),
    };

    // Build the application with middleware
    let app = build_app(state, &config);

    // Start the server
    let addr = config.socket_addr();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "Server listening");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Build the full application router with all middleware layers.
fn build_app(state: AppState, config: &Config) -> Router {
    let cors = if config.environment == Environment::Production {
        let origin = config
            .frontend_url
            .parse::<axum::http::HeaderValue>()
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("http://localhost:3001"));

        CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
            .allow_credentials(true)
            .max_age(Duration::from_secs(3600))
    } else {
        CorsLayer::permissive()
    };

    let trace = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                status_code = tracing::field::Empty,
            )
        })
        .on_response(|response: &Response, latency: Duration, span: &Span| {
            span.record("status_code", response.status().as_u16());
            tracing::info!(latency_ms = latency.as_millis(), "response");
        });

    aircade_api::routes::router()
        .with_state(state)
        .layer(cors)
        .layer(trace)
}

/// Initialize the `tracing` subscriber with an environment-based filter.
fn init_tracing(log_level: &str) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("aircade_api={log_level},tower_http=info,sea_orm=warn").into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
