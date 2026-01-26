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
// GET /api/v1/users/me
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_me_unauthenticated_returns_401() {
    let app = test_app().await;
    let (status, _body) = common::get(&app, "/api/v1/users/me").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_me_success() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "me@example.com", "meuser", "Password123").await;

    let (status, body) = common::get_with_auth(&app, "/api/v1/users/me", &token).await;
    assert_eq!(status, StatusCode::OK, "get_me failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["email"], "me@example.com");
    assert_eq!(json["username"], "meuser");
    assert_eq!(json["role"], "user");
    assert_eq!(json["subscriptionPlan"], "free");
    assert_eq!(json["accountStatus"], "active");
    // Should include auth providers
    assert!(json["authProviders"].is_array());
    let empty = vec![];
    let providers = json["authProviders"].as_array().unwrap_or(&empty);
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0]["provider"], "email");
}

// ──────────────────────────────────────────────────────────────────────────────
// PATCH /api/v1/users/me
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_me_display_name() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "update@example.com", "updateuser", "Password123").await;

    let (status, body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me",
        &json!({ "displayName": "Cool User" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["displayName"], "Cool User");
}

#[tokio::test]
async fn update_me_bio() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "bio@example.com", "biouser", "Password123").await;

    let (status, body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me",
        &json!({ "bio": "I love games!" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update bio failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["bio"], "I love games!");
}

#[tokio::test]
async fn update_me_display_name_too_long() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "long@example.com", "longuser", "Password123").await;

    let long_name = "a".repeat(101);
    let (status, _body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me",
        &json!({ "displayName": long_name }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_me_bio_too_long() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "longbio@example.com", "longbiouser", "Password123").await;

    let long_bio = "a".repeat(501);
    let (status, _body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me",
        &json!({ "bio": long_bio }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// DELETE /api/v1/users/me/avatar
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_avatar_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "avatar@example.com", "avataruser", "Password123").await;

    let (status, _body) = common::delete_with_auth(&app, "/api/v1/users/me/avatar", &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify avatar is null
    let (status, body) = common::get_with_auth(&app, "/api/v1/users/me", &token).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(json["avatarUrl"].is_null());
}

// ──────────────────────────────────────────────────────────────────────────────
// PATCH /api/v1/users/me/username
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn change_username_success() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "uname@example.com", "oldname", "Password123").await;

    let (status, body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me/username",
        &json!({ "newUsername": "newname" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "username change failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["username"], "newname");

    // Verify via GET /users/me
    let (status, body) = common::get_with_auth(&app, "/api/v1/users/me", &token).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["username"], "newname");
}

#[tokio::test]
async fn change_username_conflict() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "first@example.com", "firstuser", "Password123").await;
    signup_user(&app, "second@example.com", "takenname", "Password123").await;

    let (status, _body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me/username",
        &json!({ "newUsername": "takenname" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn change_username_invalid() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "inv@example.com", "invuser", "Password123").await;

    let (status, _body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me/username",
        &json!({ "newUsername": "ab" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// PATCH /api/v1/users/me/email
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn change_email_success() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "old@example.com", "emailuser", "Password123").await;

    let (status, body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me/email",
        &json!({
            "newEmail": "new@example.com",
            "password": "Password123"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "email change failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["email"], "new@example.com");
    assert_eq!(json["emailVerified"], false);
}

#[tokio::test]
async fn change_email_wrong_password() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "pw@example.com", "pwuser", "Password123").await;

    let (status, _body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me/email",
        &json!({
            "newEmail": "new2@example.com",
            "password": "WrongPassword"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn change_email_no_password_for_email_user() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "nopw@example.com", "nopwuser", "Password123").await;

    let (status, _body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me/email",
        &json!({ "newEmail": "new3@example.com" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn change_email_conflict() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "emaila@example.com", "emaila", "Password123").await;
    signup_user(&app, "emailb@example.com", "emailb", "Password123").await;

    let (status, _body) = common::patch_json_with_auth(
        &app,
        "/api/v1/users/me/email",
        &json!({
            "newEmail": "emailb@example.com",
            "password": "Password123"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

// ──────────────────────────────────────────────────────────────────────────────
// DELETE /api/v1/users/me (deactivate)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn deactivate_account_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "deact@example.com", "deactuser", "Password123").await;

    let (status, _body) = common::delete_json_with_auth(
        &app,
        "/api/v1/users/me",
        &json!({ "password": "Password123" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Subsequent requests should fail (account deactivated + soft-deleted → middleware
    // sees `deleted_at` first and returns 401).
    let (status, _body) = common::get_with_auth(&app, "/api/v1/users/me", &token).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn deactivate_account_wrong_password() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "deact2@example.com", "deact2user", "Password123").await;

    let (status, _body) = common::delete_json_with_auth(
        &app,
        "/api/v1/users/me",
        &json!({ "password": "WrongPassword" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ──────────────────────────────────────────────────────────────────────────────
// GET /api/v1/users/{username} (public profile)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn public_profile_success() {
    let app = test_app().await;
    signup_user(&app, "pub@example.com", "pubuser", "Password123").await;

    let (status, body) = common::get(&app, "/api/v1/users/pubuser").await;
    assert_eq!(status, StatusCode::OK, "public profile failed: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["username"], "pubuser");
    assert!(json["stats"]["gamesPublished"].is_number());
    assert!(json["stats"]["totalPlayCount"].is_number());
    // Private fields should NOT be present
    assert!(json["email"].is_null());
    assert!(json["role"].is_null());
    assert!(json["authProviders"].is_null());
}

#[tokio::test]
async fn public_profile_not_found() {
    let app = test_app().await;
    let (status, _body) = common::get(&app, "/api/v1/users/nonexistent").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn public_profile_deactivated_hidden() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "hidden@example.com", "hiddenuser", "Password123").await;

    // Deactivate
    common::delete_json_with_auth(
        &app,
        "/api/v1/users/me",
        &json!({ "password": "Password123" }),
        &token,
    )
    .await;

    // Public profile should now return 404
    let (status, _body) = common::get(&app, "/api/v1/users/hiddenuser").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
