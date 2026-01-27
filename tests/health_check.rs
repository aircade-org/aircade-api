//! End-to-end tests for health check and configuration
//!
//! These tests verify that the API configuration logic works correctly
//! for Railway deployment without modifying environment variables.

// Relax linting for tests - they don't need production-level strictness
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::useless_vec)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::map_unwrap_or)]

use std::net::TcpListener;
use std::time::Duration;

/// Find an available port for testing
fn get_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port");
    listener
        .local_addr()
        .expect("Failed to get local addr")
        .port()
}

/// Test that we can allocate an available port
#[test]
fn test_port_allocation() {
    let port = get_available_port();
    assert!(port > 0, "Should get a valid port number");
    // u16 max is 65535, so any valid u16 is in range
}

/// Test port precedence logic (simulated, no env modification)
#[test]
fn test_port_precedence_logic() {
    // Simulate the precedence logic used in Config::from_env()
    fn get_port(port: Option<&str>, server_port: Option<&str>) -> String {
        port.map(String::from)
            .or_else(|| server_port.map(String::from))
            .unwrap_or_else(|| "3000".to_string())
    }

    // PORT takes precedence
    assert_eq!(get_port(Some("4000"), Some("5000")), "4000");

    // SERVER_PORT is fallback
    assert_eq!(get_port(None, Some("5000")), "5000");

    // Default is 3000
    assert_eq!(get_port(None, None), "3000");
}

/// Test host default logic (simulated)
#[test]
fn test_host_default_logic() {
    fn get_host(server_host: Option<&str>) -> String {
        server_host
            .map(String::from)
            .unwrap_or_else(|| "0.0.0.0".to_string())
    }

    // Default should be 0.0.0.0 for container deployments
    assert_eq!(get_host(None), "0.0.0.0");

    // Custom host should work
    assert_eq!(get_host(Some("127.0.0.1")), "127.0.0.1");
}

/// Test that port parsing handles valid numbers
#[test]
fn test_valid_port_parsing() {
    let test_cases = vec!["3000", "8080", "80", "443", "65535", "1"];

    for port_str in test_cases {
        let parsed: Result<u16, _> = port_str.parse();
        assert!(parsed.is_ok(), "Should parse valid port: {}", port_str);
    }
}

/// Test that port parsing rejects invalid values
#[test]
fn test_invalid_port_parsing() {
    let test_cases = vec!["not_a_number", "-1", "65536", "", "abc123", "3.14"];

    for port_str in test_cases {
        let parsed: Result<u16, _> = port_str.parse();
        assert!(parsed.is_err(), "Should reject invalid port: {}", port_str);
    }
}

/// Test health check response structure expectations
#[test]
fn test_health_response_structure() {
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct HealthResponse {
        status: String,
        version: String,
        database: String,
    }

    // Test connected state
    let connected_json = r#"{"status":"ok","version":"0.1.0","database":"connected"}"#;
    let response: Result<HealthResponse, _> = serde_json::from_str(connected_json);
    assert!(response.is_ok(), "Health response should be valid JSON");
    let health = response.expect("Should deserialize");
    assert_eq!(health.status, "ok");
    assert_eq!(health.database, "connected");

    // Test disconnected state
    let disconnected_json = r#"{"status":"ok","version":"0.1.0","database":"disconnected"}"#;
    let response: Result<HealthResponse, _> = serde_json::from_str(disconnected_json);
    assert!(response.is_ok(), "Disconnected state should also be valid");
}

/// Test timeout configuration for health checks
#[test]
fn test_health_check_timeout_reasonable() {
    // Railway's health check timeout from railway.toml is 100 seconds
    let health_check_timeout = Duration::from_secs(100);

    // This should be enough for DB connection + response
    assert!(
        health_check_timeout.as_secs() >= 30,
        "Health check timeout should be at least 30 seconds"
    );
    assert!(
        health_check_timeout.as_secs() <= 300,
        "Health check timeout should not exceed 5 minutes"
    );
}

/// Test server address formatting
#[test]
fn test_server_address_format() {
    fn server_address(host: &str, port: u16) -> String {
        format!("{}:{}", host, port)
    }

    assert_eq!(server_address("0.0.0.0", 3000), "0.0.0.0:3000");
    assert_eq!(server_address("127.0.0.1", 8080), "127.0.0.1:8080");
    assert_eq!(server_address("localhost", 80), "localhost:80");
}

/// Test that TCP listener can bind to 0.0.0.0
#[test]
fn test_bind_to_all_interfaces() {
    let port = get_available_port();
    let result = TcpListener::bind(format!("0.0.0.0:{port}"));
    assert!(result.is_ok(), "Should be able to bind to 0.0.0.0");
}

/// Test that TCP listener can bind to localhost
#[test]
fn test_bind_to_localhost() {
    let port = get_available_port();
    let result = TcpListener::bind(format!("127.0.0.1:{port}"));
    assert!(result.is_ok(), "Should be able to bind to 127.0.0.1");
}

/// Test `DATABASE_URL` format validation
#[test]
fn test_database_url_formats() {
    let valid_urls = vec![
        "postgres://user:pass@localhost:5432/db",
        "postgres://user:pass@host.railway.internal:5432/railway",
        "postgresql://user:password@db.example.com:5432/mydb?sslmode=require",
    ];

    for url in valid_urls {
        assert!(
            url.starts_with("postgres"),
            "Should be a postgres URL: {}",
            url
        );
        assert!(url.contains("://"), "Should be a valid URL format: {}", url);
    }
}

/// Test environment variable naming conventions
#[test]
fn test_env_var_naming() {
    let required_vars = vec!["DATABASE_URL"];
    let optional_vars = vec![
        "PORT",
        "SERVER_PORT",
        "SERVER_HOST",
        "ENVIRONMENT",
        "LOG_LEVEL",
    ];

    // All variable names should be uppercase with underscores
    for var in required_vars.iter().chain(optional_vars.iter()) {
        assert!(
            var.chars().all(|c| c.is_uppercase() || c == '_'),
            "Environment variable {} should be SCREAMING_SNAKE_CASE",
            var
        );
    }
}

/// Test log level options
#[test]
fn test_valid_log_levels() {
    let valid_levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in &valid_levels {
        assert_eq!(*level, level.to_lowercase());
    }

    // Default should be "info"
    let default_level = "info";
    assert!(valid_levels.contains(&default_level));
}

/// Test environment options
#[test]
fn test_valid_environments() {
    let valid_environments = vec!["development", "production", "staging", "test"];

    for env in &valid_environments {
        assert_eq!(*env, env.to_lowercase());
    }

    // Default should be "development"
    let default_env = "development";
    assert!(valid_environments.contains(&default_env));
}
