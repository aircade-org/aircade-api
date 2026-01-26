mod common;

use axum::http::StatusCode;
use axum::Router;
use migration::{Migrator, MigratorTrait};
use serde_json::json;

use aircade_api::config::{Config, Environment};
use aircade_api::state::AppState;

async fn test_app() -> Router {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap_or_default();
    Migrator::up(&db, None).await.unwrap_or_default();

    let state = AppState {
        db,
        config: Config {
            database_url: String::new(),
            server_host: std::net::IpAddr::from([127, 0, 0, 1]),
            server_port: 0,
            environment: Environment::Development,
            log_level: "warn".to_string(),
            jwt_secret: "test-secret-key-for-testing-only-32chars".to_string(),
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
        },
    };

    aircade_api::routes::router().with_state(state)
}

/// Helper: sign up a user and return (`access_token`, `refresh_token`).
async fn signup_user(
    app: &Router,
    email: &str,
    username: &str,
    password: &str,
) -> (String, String) {
    let (status, body) = common::post_json(
        app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": email,
            "username": username,
            "password": password,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "signup failed: {body}");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let token = json["token"].as_str().unwrap_or_default().to_string();
    let refresh = json["refreshToken"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    (token, refresh)
}

// ──────────────────────────────────────────────────────────────────────────────
// Signup tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn signup_email_success() {
    let app = test_app().await;
    let (status, body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": "test@example.com",
            "username": "testuser",
            "password": "SecurePass123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["user"]["email"], "test@example.com");
    assert_eq!(json["user"]["username"], "testuser");
    assert_eq!(json["user"]["emailVerified"], false);
    assert_eq!(json["user"]["role"], "user");
    assert_eq!(json["user"]["subscriptionPlan"], "free");
    assert!(json["token"].is_string());
    assert!(json["refreshToken"].is_string());
}

#[tokio::test]
async fn signup_email_duplicate_email() {
    let app = test_app().await;
    signup_user(&app, "dup@example.com", "user1", "Password123").await;

    let (status, body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": "dup@example.com",
            "username": "user2",
            "password": "Password123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Email")
    );
}

#[tokio::test]
async fn signup_email_duplicate_username() {
    let app = test_app().await;
    signup_user(&app, "a@example.com", "taken_name", "Password123").await;

    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": "b@example.com",
            "username": "taken_name",
            "password": "Password123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn signup_email_weak_password() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": "test@example.com",
            "username": "testuser",
            "password": "short",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn signup_email_invalid_email() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": "notanemail",
            "username": "testuser",
            "password": "Password123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn signup_email_invalid_username() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": "test@example.com",
            "username": "ab",
            "password": "Password123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// Signin tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn signin_email_success() {
    let app = test_app().await;
    signup_user(&app, "login@example.com", "loginuser", "Password123").await;

    let (status, body) = common::post_json(
        &app,
        "/api/v1/auth/signin/email",
        &json!({
            "email": "login@example.com",
            "password": "Password123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["user"]["email"], "login@example.com");
    assert!(json["token"].is_string());
    assert!(json["refreshToken"].is_string());
}

#[tokio::test]
async fn signin_email_wrong_password() {
    let app = test_app().await;
    signup_user(&app, "user@example.com", "testuser", "Password123").await;

    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/signin/email",
        &json!({
            "email": "user@example.com",
            "password": "WrongPassword",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn signin_email_nonexistent() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/signin/email",
        &json!({
            "email": "nobody@example.com",
            "password": "Password123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ──────────────────────────────────────────────────────────────────────────────
// Verify email tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn verify_email_invalid_token() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/verify-email",
        &json!({ "token": "invalid-token-value" }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// Resend verification tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn resend_verification_unauthenticated() {
    let app = test_app().await;
    let (status, _body) =
        common::post_json(&app, "/api/v1/auth/resend-verification", &json!({})).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn resend_verification_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "verify@example.com", "verifyuser", "Password123").await;

    let (status, body) =
        common::post_json_with_auth(&app, "/api/v1/auth/resend-verification", &json!({}), &token)
            .await;

    assert_eq!(status, StatusCode::OK, "resend failed: {body}");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(
        json["message"]
            .as_str()
            .unwrap_or_default()
            .contains("sent")
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Password reset tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn password_reset_request_always_200() {
    let app = test_app().await;

    // Should succeed even for non-existent emails
    let (status, body) = common::post_json(
        &app,
        "/api/v1/auth/password-reset/request",
        &json!({ "email": "nobody@example.com" }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(
        json["message"]
            .as_str()
            .unwrap_or_default()
            .contains("reset link")
    );
}

#[tokio::test]
async fn password_reset_confirm_invalid_token() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/password-reset/confirm",
        &json!({
            "token": "invalid-reset-token",
            "newPassword": "NewPassword123",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// Password change tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn password_change_unauthenticated() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/password/change",
        &json!({
            "currentPassword": "old",
            "newPassword": "new12345",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_change_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "change@example.com", "changeuser", "OldPassword123").await;

    let (status, body) = common::post_json_with_auth(
        &app,
        "/api/v1/auth/password/change",
        &json!({
            "currentPassword": "OldPassword123",
            "newPassword": "NewPassword456",
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "change failed: {body}");

    // Can sign in with new password
    let (status2, _body2) = common::post_json(
        &app,
        "/api/v1/auth/signin/email",
        &json!({
            "email": "change@example.com",
            "password": "NewPassword456",
        }),
    )
    .await;
    assert_eq!(status2, StatusCode::OK);

    // Old password no longer works
    let (status3, _body3) = common::post_json(
        &app,
        "/api/v1/auth/signin/email",
        &json!({
            "email": "change@example.com",
            "password": "OldPassword123",
        }),
    )
    .await;
    assert_eq!(status3, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_change_wrong_current() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "wrong@example.com", "wronguser", "Correct123").await;

    let (status, _body) = common::post_json_with_auth(
        &app,
        "/api/v1/auth/password/change",
        &json!({
            "currentPassword": "WrongPassword",
            "newPassword": "NewPassword456",
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ──────────────────────────────────────────────────────────────────────────────
// Token refresh tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_token_success() {
    let app = test_app().await;
    let (_token, refresh) =
        signup_user(&app, "refresh@example.com", "refreshuser", "Password123").await;

    let (status, body) = common::post_json(
        &app,
        "/api/v1/auth/refresh",
        &json!({ "refreshToken": refresh }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "refresh failed: {body}");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(json["token"].is_string());
    assert!(json["refreshToken"].is_string());
    // New tokens should be different from the old ones
    assert_ne!(json["refreshToken"].as_str().unwrap_or_default(), refresh);
}

#[tokio::test]
async fn refresh_token_invalid() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/refresh",
        &json!({ "refreshToken": "invalid-token" }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_token_revoked_after_signout() {
    let app = test_app().await;
    let (token, refresh) =
        signup_user(&app, "signout@example.com", "signoutuser", "Password123").await;

    // Sign out (revokes refresh token)
    let (status, _body) = common::post_json_with_auth(
        &app,
        "/api/v1/auth/signout",
        &json!({ "refreshToken": &refresh }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Try to use the revoked refresh token
    let (status2, _body2) = common::post_json(
        &app,
        "/api/v1/auth/refresh",
        &json!({ "refreshToken": &refresh }),
    )
    .await;
    assert_eq!(status2, StatusCode::UNAUTHORIZED);
}

// ──────────────────────────────────────────────────────────────────────────────
// Signout tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn signout_success() {
    let app = test_app().await;
    let (token, refresh) = signup_user(&app, "bye@example.com", "byeuser", "Password123").await;

    let (status, _body) = common::post_json_with_auth(
        &app,
        "/api/v1/auth/signout",
        &json!({ "refreshToken": &refresh }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn signout_unauthenticated() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/auth/signout",
        &json!({ "refreshToken": "some-token" }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ──────────────────────────────────────────────────────────────────────────────
// OAuth tests (unconfigured returns 422)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn oauth_google_unconfigured() {
    let app = test_app().await;
    let (status, _body) = common::get(&app, "/api/v1/auth/oauth/google").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn oauth_github_unconfigured() {
    let app = test_app().await;
    let (status, _body) = common::get(&app, "/api/v1/auth/oauth/github").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
