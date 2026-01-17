use crate::{config::Config, errors::AppError, migrations::Migrator, routes};
use axum::Router;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<RwLock<Option<DatabaseConnection>>>,
    #[allow(dead_code)]
    pub config: Config,
}

pub struct Api;

impl Api {
    /// Initialize and launch the API server
    pub async fn launch() -> Result<(), AppError> {
        // Initialize basic tracing first so we can see startup errors
        Self::init_early_tracing();

        tracing::info!("AirCade API starting up...");

        // Load configuration
        let config = match Config::from_env() {
            Ok(cfg) => {
                tracing::info!("Configuration loaded successfully");
                cfg
            }
            Err(e) => {
                tracing::error!("Failed to load configuration: {}", e);
                return Err(e);
            }
        };

        tracing::info!("Starting AirCade API server");
        tracing::debug!(
            "Configuration: host={}, port={}",
            config.server_host,
            config.server_port
        );

        // Create application state with no DB connection yet
        let state = Arc::new(AppState {
            db: Arc::new(RwLock::new(None)),
            config: config.clone(),
        });

        // Build router
        let app = Self::build_router(Arc::clone(&state));

        // Spawn database connection in the background so the server starts immediately
        let db_state = Arc::clone(&state);
        let db_config = config.clone();
        tokio::spawn(async move {
            match Self::connect_database(&db_config).await {
                Ok(db) => {
                    // Run migrations
                    if let Err(e) = Self::run_migrations(&db).await {
                        tracing::error!("Migration failed: {e}");
                        return;
                    }
                    *(db_state.db.write().await) = Some(db);
                    tracing::info!("Database ready");
                }
                Err(e) => {
                    tracing::error!("Database initialization failed: {e}");
                }
            }
        });

        // Start server immediately (health endpoint available right away)
        Self::start_server(app, &config).await?;

        Ok(())
    }

    /// Initialize early tracing so we can see startup errors before config is loaded
    fn init_early_tracing() {
        // Use a simple format that works before config is available
        let _ = tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .try_init();
    }

    /// Initialize tracing subscriber for structured logging (no-op if already initialized)
    #[allow(dead_code)]
    const fn init_tracing(_config: &Config) {
        // Early tracing already initialized, this is kept for API compatibility
    }

    /// Connect to the database with timeout
    async fn connect_database(config: &Config) -> Result<DatabaseConnection, AppError> {
        tracing::info!("Connecting to database...");

        // Log the database host (not the full URL for security)
        if let Some(host) = config.database_url.split('@').nth(1) {
            tracing::info!(
                "Database host: {}",
                host.split('/').next().unwrap_or("unknown")
            );
        }

        // Add a 30-second timeout for database connection
        let connect_future = Database::connect(&config.database_url);
        let db = tokio::time::timeout(std::time::Duration::from_secs(30), connect_future)
            .await
            .map_err(|_| {
                tracing::error!("Database connection timed out after 30 seconds");
                AppError::Database("Database connection timed out".to_string())
            })?
            .map_err(|e| {
                tracing::error!("Database connection failed: {}", e);
                AppError::Database(format!("Failed to connect to database: {e}"))
            })?;

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
            .merge(routes::games::routes())
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
            match signal::unix::signal(signal::unix::SignalKind::terminate()) {
                Ok(mut sig) => {
                    sig.recv().await;
                }
                Err(e) => {
                    tracing::error!("Failed to install signal handler: {e}");
                    std::future::pending::<()>().await;
                }
            }
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
