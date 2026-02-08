pub mod jwt;
pub mod middleware;
pub mod oauth;
pub mod password;

use axum::http::HeaderMap;

/// Extract the client IP address from request headers.
///
/// Checks `X-Forwarded-For` first (for reverse proxies like Railway),
/// then falls back to `X-Real-IP`.
#[must_use]
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(std::string::ToString::to_string)
        })
}
