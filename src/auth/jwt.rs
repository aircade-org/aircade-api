use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;

/// JWT claims embedded in both access and refresh tokens.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: user ID as a UUID string.
    pub sub: String,
    /// User role: `"user"`, `"moderator"`, or `"admin"`.
    pub role: String,
    /// Token type: `"access"` or `"refresh"`.
    pub token_type: String,
    /// Expiration time (Unix timestamp).
    pub exp: i64,
    /// Issued-at time (Unix timestamp).
    pub iat: i64,
    /// Unique JWT identifier (used for refresh token tracking in the database).
    pub jti: String,
}

/// A pair of access and refresh tokens returned on sign-in/sign-up.
#[derive(Debug)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    /// The `jti` of the refresh token, used as its identifier in the database.
    pub refresh_jti: Uuid,
    /// Expiration of the refresh token as a `chrono` timestamp.
    pub refresh_expires_at: chrono::DateTime<Utc>,
}

/// Generate a new access + refresh token pair for the given user.
///
/// # Errors
///
/// Returns an error if JWT encoding fails.
pub fn generate_token_pair(
    user_id: Uuid,
    role: &str,
    config: &Config,
) -> anyhow::Result<TokenPair> {
    let now = Utc::now();
    let access_jti = Uuid::new_v4();
    let refresh_jti = Uuid::new_v4();

    #[allow(clippy::cast_possible_wrap)]
    let access_exp = now.timestamp() + config.jwt_access_expiration_secs as i64;
    #[allow(clippy::cast_possible_wrap)]
    let refresh_exp = now.timestamp() + config.jwt_refresh_expiration_secs as i64;

    let access_claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        token_type: "access".to_string(),
        exp: access_exp,
        iat: now.timestamp(),
        jti: access_jti.to_string(),
    };

    let refresh_claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        token_type: "refresh".to_string(),
        exp: refresh_exp,
        iat: now.timestamp(),
        jti: refresh_jti.to_string(),
    };

    let key = EncodingKey::from_secret(config.jwt_secret.as_bytes());

    let access_token = encode(&Header::default(), &access_claims, &key)
        .map_err(|e| anyhow::anyhow!("Failed to encode access token: {e}"))?;

    let refresh_token = encode(&Header::default(), &refresh_claims, &key)
        .map_err(|e| anyhow::anyhow!("Failed to encode refresh token: {e}"))?;

    let refresh_expires_at =
        chrono::DateTime::from_timestamp(refresh_exp, 0).unwrap_or_else(Utc::now);

    Ok(TokenPair {
        access_token,
        refresh_token,
        refresh_jti,
        refresh_expires_at,
    })
}

/// Validate an access token and return its claims.
///
/// # Errors
///
/// Returns an error if the token is invalid, expired, or not an access token.
pub fn validate_access_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    let token_data = decode::<Claims>(token, &key, &validation)
        .map_err(|e| anyhow::anyhow!("Invalid access token: {e}"))?;

    if token_data.claims.token_type != "access" {
        return Err(anyhow::anyhow!("Token is not an access token"));
    }

    Ok(token_data.claims)
}

/// Validate a refresh token and return its claims.
///
/// # Errors
///
/// Returns an error if the token is invalid, expired, or not a refresh token.
pub fn validate_refresh_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    let token_data = decode::<Claims>(token, &key, &validation)
        .map_err(|e| anyhow::anyhow!("Invalid refresh token: {e}"))?;

    if token_data.claims.token_type != "refresh" {
        return Err(anyhow::anyhow!("Token is not a refresh token"));
    }

    Ok(token_data.claims)
}

/// Generate a short-lived JWT for OAuth CSRF state (30 minutes).
///
/// # Errors
///
/// Returns an error if JWT encoding fails.
pub fn generate_oauth_state(secret: &str, redirect_uri: Option<&str>) -> anyhow::Result<String> {
    let now = Utc::now();
    let csrf = Uuid::new_v4().to_string();

    let claims = OAuthStateClaims {
        csrf,
        redirect_uri: redirect_uri.map(String::from),
        exp: now.timestamp() + 1800, // 30 minutes
        iat: now.timestamp(),
    };

    let key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), &claims, &key)
        .map_err(|e| anyhow::anyhow!("Failed to encode OAuth state: {e}"))
}

/// Validate an OAuth CSRF state token.
///
/// # Errors
///
/// Returns an error if the state token is invalid or expired.
pub fn validate_oauth_state(state: &str, secret: &str) -> anyhow::Result<OAuthStateClaims> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    let token_data = decode::<OAuthStateClaims>(state, &key, &validation)
        .map_err(|e| anyhow::anyhow!("Invalid OAuth state: {e}"))?;

    Ok(token_data.claims)
}

/// Claims for OAuth CSRF state tokens.
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthStateClaims {
    pub csrf: String,
    pub redirect_uri: Option<String>,
    pub exp: i64,
    pub iat: i64,
}
