use std::net::{IpAddr, SocketAddr};

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_host: IpAddr,
    pub server_port: u16,
    pub environment: Environment,
    pub log_level: String,
    pub jwt_secret: String,
    pub jwt_access_expiration_secs: u64,
    pub jwt_refresh_expiration_secs: u64,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub github_redirect_uri: String,
    pub frontend_url: String,
    pub upload_dir: String,
}

/// Deployment environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required: `DATABASE_URL`
    /// Optional with defaults: `SERVER_HOST`, `SERVER_PORT`, `ENVIRONMENT`, `LOG_LEVEL`
    ///
    /// On Railway, `PORT` overrides `SERVER_PORT` and host defaults to `0.0.0.0`.
    ///
    /// # Errors
    ///
    /// Returns an error if `DATABASE_URL` is not set, or if `SERVER_HOST` / `SERVER_PORT`
    /// contain invalid values.
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;

        let environment = match std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .as_str()
        {
            "production" => Environment::Production,
            "staging" => Environment::Staging,
            _ => Environment::Development,
        };

        // Railway provides PORT; fall back to SERVER_PORT, then 3000
        let server_port = std::env::var("PORT")
            .or_else(|_| std::env::var("SERVER_PORT"))
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("SERVER_PORT / PORT must be a valid u16"))?;

        // In production, default to 0.0.0.0 so Railway can route traffic
        let default_host = if environment == Environment::Production {
            "0.0.0.0"
        } else {
            "127.0.0.1"
        };

        let server_host = std::env::var("SERVER_HOST")
            .unwrap_or_else(|_| default_host.to_string())
            .parse::<IpAddr>()
            .map_err(|_| anyhow::anyhow!("SERVER_HOST must be a valid IP address"))?;

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let jwt_secret =
            std::env::var("JWT_SECRET").map_err(|_| anyhow::anyhow!("JWT_SECRET must be set"))?;

        let jwt_access_expiration_secs = std::env::var("JWT_ACCESS_EXPIRATION")
            .unwrap_or_else(|_| "900".to_string())
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("JWT_ACCESS_EXPIRATION must be a valid u64"))?;

        let jwt_refresh_expiration_secs = std::env::var("JWT_REFRESH_EXPIRATION")
            .unwrap_or_else(|_| "604800".to_string())
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("JWT_REFRESH_EXPIRATION must be a valid u64"))?;

        let google_client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| String::new());
        let google_client_secret =
            std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| String::new());
        let google_redirect_uri =
            std::env::var("GOOGLE_REDIRECT_URI").unwrap_or_else(|_| String::new());
        let github_client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| String::new());
        let github_client_secret =
            std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_else(|_| String::new());
        let github_redirect_uri =
            std::env::var("GITHUB_REDIRECT_URI").unwrap_or_else(|_| String::new());
        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3001".to_string());
        let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string());

        Ok(Self {
            database_url,
            server_host,
            server_port,
            environment,
            log_level,
            jwt_secret,
            jwt_access_expiration_secs,
            jwt_refresh_expiration_secs,
            google_client_id,
            google_client_secret,
            google_redirect_uri,
            github_client_id,
            github_client_secret,
            github_redirect_uri,
            frontend_url,
            upload_dir,
        })
    }

    /// Build the socket address for the server to bind to.
    #[must_use]
    pub const fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.server_host, self.server_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_addr() {
        let config = Config {
            database_url: String::new(),
            server_host: IpAddr::from([127, 0, 0, 1]),
            server_port: 3000,
            environment: Environment::Development,
            log_level: "info".to_string(),
            jwt_secret: "test-secret".to_string(),
            jwt_access_expiration_secs: 900,
            jwt_refresh_expiration_secs: 604_800,
            google_client_id: String::new(),
            google_client_secret: String::new(),
            google_redirect_uri: String::new(),
            github_client_id: String::new(),
            github_client_secret: String::new(),
            github_redirect_uri: String::new(),
            frontend_url: "http://localhost:3001".to_string(),
            upload_dir: "test_uploads".to_string(),
        };
        let addr = config.socket_addr();
        assert_eq!(addr.port(), 3000);
    }
}
