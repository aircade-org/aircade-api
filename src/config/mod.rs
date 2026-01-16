use crate::errors::AppError;
use std::env;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Database connection URL
    pub database_url: String,
    /// Server host address
    pub server_host: String,
    /// Server port
    pub server_port: u16,
    /// Environment (development, production)
    pub environment: String,
    // Log level (trace, debug, info, warn, error)
    // pub log_level: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, AppError> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").map_err(|_| {
            AppError::Config("DATABASE_URL environment variable not set".to_string())
        })?;

        // Default to 0.0.0.0 for container deployments (Railway, Docker)
        let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        // Railway provides PORT, fallback to SERVER_PORT or 3000
        let server_port = env::var("PORT")
            .or_else(|_| env::var("SERVER_PORT"))
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| AppError::Config("Invalid PORT/SERVER_PORT value".to_string()))?;

        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        // let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            database_url,
            server_host,
            server_port,
            environment,
            // log_level,
        })
    }

    /// Get server address as string
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    /// Check if running in production
    #[allow(dead_code)]
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// Check if running in development
    #[allow(dead_code)]
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
}
