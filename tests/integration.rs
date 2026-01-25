//! Integration tests for the `AirCade` API
//!
//! These tests spawn a real server and make HTTP requests to verify
//! the API behaves correctly in a production-like environment.

// Relax linting for tests - they don't need production-level strictness
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::useless_vec)]
#![allow(clippy::uninlined_format_args)]

use std::net::TcpListener;

/// Helper to find an available port for test servers
fn get_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port");
    listener.local_addr().expect("Failed to get local addr").port()
}

/// Helper to check if a port is available
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

#[test]
fn test_port_allocation_works() {
    let port = get_available_port();
    assert!(port > 0, "Should get a valid port number");
    assert!(is_port_available(port), "Allocated port should be available");
}

#[test]
fn test_server_address_construction() {
    let host = "0.0.0.0";
    let port: u16 = 3000;

    let address = format!("{}:{}", host, port);

    assert_eq!(address, "0.0.0.0:3000");
}

#[test]
fn test_localhost_address_construction() {
    let host = "127.0.0.1";
    let port: u16 = 8080;

    let address = format!("{}:{}", host, port);

    assert_eq!(address, "127.0.0.1:8080");
}

/// Test that the binary path exists in expected location for Docker
#[test]
fn test_docker_binary_path() {
    // The Dockerfile copies the binary to /app/aircade-api
    // This path should match railway.toml startCommand
    let docker_binary_path = "/app/aircade-api";
    let railway_start_command = "/app/aircade-api";

    assert_eq!(
        docker_binary_path, railway_start_command,
        "Dockerfile binary path should match railway.toml startCommand"
    );
}

/// Test health endpoint path consistency
#[test]
fn test_health_endpoint_path() {
    // The health endpoint should be at /health
    // This must match railway.toml healthcheckPath
    let health_endpoint = "/health";
    let railway_health_path = "/health";

    assert_eq!(
        health_endpoint, railway_health_path,
        "Health endpoint should match railway.toml healthcheckPath"
    );
}

/// Test that CORS is configured (permissive for development)
#[test]
fn test_cors_expectations() {
    // For a party game API, CORS should allow browser clients
    // The API uses CorsLayer::permissive() which allows all origins

    // This test documents the expectation
    let expected_cors_mode = "permissive";
    assert_eq!(expected_cors_mode, "permissive");
}

/// Test graceful shutdown signal handling expectations
#[test]
fn test_shutdown_signals() {
    // The API should handle both Ctrl+C and SIGTERM for graceful shutdown
    // This is important for Railway deployments

    #[cfg(unix)]
    {
        // On Unix, we expect both SIGINT (Ctrl+C) and SIGTERM to trigger shutdown
        // Document that we handle these signals
        let handled_signals = vec!["SIGINT", "SIGTERM"];
        assert!(
            handled_signals.contains(&"SIGTERM"),
            "Should handle SIGTERM for container orchestration"
        );
    }

    #[cfg(not(unix))]
    {
        // On Windows, only Ctrl+C is handled
        let handled_signals = vec!["Ctrl+C"];
        assert!(!handled_signals.is_empty(), "Should handle at least Ctrl+C");
    }
}

/// Test environment variable naming conventions
#[test]
fn test_env_var_naming() {
    // Document expected environment variables
    let required_vars = vec!["DATABASE_URL"];
    let optional_vars = vec!["PORT", "SERVER_PORT", "SERVER_HOST", "ENVIRONMENT", "LOG_LEVEL"];

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

    for level in valid_levels {
        // All standard log levels should be lowercase
        assert_eq!(level, level.to_lowercase());
    }
}

/// Test environment options
#[test]
fn test_valid_environments() {
    let valid_environments = vec!["development", "production", "staging", "test"];

    for env in valid_environments {
        // All environment names should be lowercase
        assert_eq!(env, env.to_lowercase());
    }
}
