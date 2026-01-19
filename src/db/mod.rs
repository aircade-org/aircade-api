use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

/// Establish a connection to the database with connection pooling.
///
/// # Errors
///
/// Returns an error if the connection cannot be established.
pub async fn connect(database_url: &str) -> anyhow::Result<DatabaseConnection> {
    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(20)
        .min_connections(2)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false);

    let db = Database::connect(opts).await?;
    Ok(db)
}
