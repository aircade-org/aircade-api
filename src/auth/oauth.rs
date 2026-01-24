use oauth2::basic::{BasicErrorResponseType, BasicTokenType};
use oauth2::{
    AuthUrl, Client, ClientId, ClientSecret, EmptyExtraTokenFields, EndpointNotSet, EndpointSet,
    RedirectUrl, RevocationErrorResponseType, StandardErrorResponse, StandardRevocableToken,
    StandardTokenIntrospectionResponse, StandardTokenResponse, TokenUrl,
};
use serde::Deserialize;

use crate::config::Config;

/// Fully configured `OAuth2` client type (auth URI, token URI, and redirect URI all set).
pub type ConfiguredClient = Client<
    StandardErrorResponse<BasicErrorResponseType>,
    StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardRevocableToken,
    StandardErrorResponse<RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

/// Build an `OAuth2` client for Google.
///
/// # Errors
///
/// Returns an error if the OAuth URLs are malformed.
pub fn google_client(config: &Config) -> anyhow::Result<ConfiguredClient> {
    let client = Client::new(ClientId::new(config.google_client_id.clone()))
        .set_client_secret(ClientSecret::new(config.google_client_secret.clone()))
        .set_auth_uri(AuthUrl::new(
            "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        )?)
        .set_token_uri(TokenUrl::new(
            "https://oauth2.googleapis.com/token".to_string(),
        )?)
        .set_redirect_uri(RedirectUrl::new(config.google_redirect_uri.clone())?);
    Ok(client)
}

/// Build an `OAuth2` client for `GitHub`.
///
/// # Errors
///
/// Returns an error if the OAuth URLs are malformed.
pub fn github_client(config: &Config) -> anyhow::Result<ConfiguredClient> {
    let client = Client::new(ClientId::new(config.github_client_id.clone()))
        .set_client_secret(ClientSecret::new(config.github_client_secret.clone()))
        .set_auth_uri(AuthUrl::new(
            "https://github.com/login/oauth/authorize".to_string(),
        )?)
        .set_token_uri(TokenUrl::new(
            "https://github.com/login/oauth/access_token".to_string(),
        )?)
        .set_redirect_uri(RedirectUrl::new(config.github_redirect_uri.clone())?);
    Ok(client)
}

/// Google user info returned from the userinfo endpoint.
#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// Fetch user info from Google's userinfo endpoint.
///
/// # Errors
///
/// Returns an error if the HTTP request fails or the response is malformed.
pub async fn fetch_google_userinfo(access_token: &str) -> anyhow::Result<GoogleUserInfo> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch Google userinfo: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Google userinfo request failed ({status}): {body}"
        ));
    }

    resp.json::<GoogleUserInfo>()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Google userinfo: {e}"))
}

/// `GitHub` user info returned from the `GitHub` API.
#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// `GitHub` email info from the `/user/emails` endpoint.
#[derive(Debug, Deserialize)]
pub struct GitHubEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

/// Fetch user info from `GitHub`'s API.
///
/// # Errors
///
/// Returns an error if the HTTP request fails or the response is malformed.
pub async fn fetch_github_userinfo(access_token: &str) -> anyhow::Result<GitHubUserInfo> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .bearer_auth(access_token)
        .header("User-Agent", "AirCade-API")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch GitHub userinfo: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "GitHub userinfo request failed ({status}): {body}"
        ));
    }

    resp.json::<GitHubUserInfo>()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse GitHub userinfo: {e}"))
}

/// Fetch the primary verified email from `GitHub`'s `/user/emails` endpoint.
///
/// # Errors
///
/// Returns an error if the HTTP request fails or no primary email is found.
pub async fn fetch_github_primary_email(access_token: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user/emails")
        .bearer_auth(access_token)
        .header("User-Agent", "AirCade-API")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch GitHub emails: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "GitHub emails request failed ({status}): {body}"
        ));
    }

    let emails: Vec<GitHubEmail> = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse GitHub emails: {e}"))?;

    emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email)
        .ok_or_else(|| anyhow::anyhow!("No primary verified email found on GitHub account"))
}
