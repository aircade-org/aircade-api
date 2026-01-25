use crate::{config::Config, errors::AppError, migrations::Migrator, routes};
use axum::Router;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::sync::Arc;
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    #[allow(dead_code)]
    pub config: Config,
}

pub struct Api;

impl Api {
    /// Initialize and launch the API server
    pub async fn launch() -> Result<(), AppError> {
        // Load configuration
        let config = Config::from_env()?;

        // Initialize tracing/logging
        Self::init_tracing(&config);

        tracing::info!("Starting AirCade API server");
        tracing::debug!("Configuration loaded: {:?}", config);

        // Connect to database
        let db = Self::connect_database(&config).await?;

        // Run migrations
        Self::run_migrations(&db).await?;

        // Create application state
        let state = Arc::new(AppState {
            db,
            config: config.clone(),
        });

        // Build router
        let app = Self::build_router(state);

        // Start server
        Self::start_server(app, &config).await?;

        Ok(())
    }

    /// Initialize tracing subscriber for structured logging
    fn init_tracing(config: &Config) {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| config.log_level.clone().into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    /// Connect to the database
    async fn connect_database(config: &Config) -> Result<DatabaseConnection, AppError> {
        tracing::info!("Connecting to database...");

        let db = Database::connect(&config.database_url)
            .await
            .map_err(|e| AppError::Database(format!("Failed to connect to database: {e}")))?;

        tracing::info!("Database connection established");

        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(db: &DatabaseConnection) -> Result<(), AppError> {
        tracing::info!("Running database migrations...");

        Migrator::up(db, None)
            .await
            .map_err(|e| AppError::Database(format!("Failed to run migrations: {e}")))?;

        tracing::info!("Database migrations completed");

        Ok(())
    }

    /// Build the Axum router with all routes and middleware
    fn build_router(state: Arc<AppState>) -> Router {
        Router::new()
            .merge(routes::health::routes())
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }

    /// Start the HTTP server
    async fn start_server(app: Router, config: &Config) -> Result<(), AppError> {
        let addr = config.server_address();
        tracing::info!("Server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to bind to {addr}: {e}")))?;

        axum::serve(listener, app)
            .with_graceful_shutdown(Self::shutdown_signal())
            .await
            .map_err(|e| AppError::Internal(format!("Server error: {e}")))?;

        Ok(())
    }

    /// Wait for shutdown signal (Ctrl+C)
    async fn shutdown_signal() {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .map_err(|e| tracing::error!("Failed to install Ctrl+C handler: {e}"))
                .ok();
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .map_err(|e| tracing::error!("Failed to install signal handler: {e}"))
                .ok()
                .and_then(|mut s| async move { s.recv().await }.into());
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            () = ctrl_c => {
                tracing::info!("Received Ctrl+C signal, shutting down gracefully...");
            },
            () = terminate => {
                tracing::info!("Received terminate signal, shutting down gracefully...");
            },
        }
    }
}
