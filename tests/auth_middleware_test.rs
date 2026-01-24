mod common;

use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use migration::{Migrator, MigratorTrait};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use serde_json::json;
use uuid::Uuid;

use aircade_api::auth::jwt;
use aircade_api::auth::middleware::{AdminUser, AuthUser, ModeratorUser};
use aircade_api::auth::password;
use aircade_api::config::{Config, Environment};
use aircade_api::entities::{auth_provider, user};
use aircade_api::state::AppState;

fn test_config() -> Config {
    Config {
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
    }
}

async fn test_app_with_middleware_routes() -> (Router, AppState) {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap_or_default();
    Migrator::up(&db, None).await.unwrap_or_default();

    let state = AppState {
        db,
        config: test_config(),
    };

    // Create test routes that exercise the middleware extractors
    let app = Router::new()
        .route(
            "/test/user",
            get(|AuthUser(u): AuthUser| async move {
                Json(json!({ "id": u.id.to_string(), "role": u.role }))
            }),
        )
        .route(
            "/test/moderator",
            get(|ModeratorUser(u): ModeratorUser| async move {
                Json(json!({ "id": u.id.to_string(), "role": u.role }))
            }),
        )
        .route(
            "/test/admin",
            get(|AdminUser(u): AdminUser| async move {
                Json(json!({ "id": u.id.to_string(), "role": u.role }))
            }),
        )
        .with_state(state.clone());

    (app, state)
}

async fn create_user(
    state: &AppState,
    role: &str,
    account_status: &str,
) -> anyhow::Result<(user::Model, String)> {
    let now = Utc::now().fixed_offset();
    let user_id = Uuid::new_v4();

    let new_user = user::ActiveModel {
        id: Set(user_id),
        email: Set(format!("{user_id}@test.com")),
        username: Set(format!("user_{}", &user_id.to_string()[..8])),
        display_name: Set(None),
        avatar_url: Set(None),
        bio: Set(None),
        email_verified: Set(false),
        role: Set(role.to_string()),
        subscription_plan: Set("free".to_string()),
        subscription_expires_at: Set(None),
        account_status: Set(account_status.to_string()),
        suspension_reason: Set(if account_status == "suspended" {
            Some("Test suspension".to_string())
        } else {
            None
        }),
        last_login_at: Set(None),
        last_login_ip: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };
    let user_model = new_user.insert(&state.db).await?;

    // Create email auth provider
    let pw_hash = password::hash_password("TestPassword123")?;
    let provider = auth_provider::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        provider: Set("email".to_string()),
        provider_id: Set(format!("{user_id}@test.com")),
        password_hash: Set(Some(pw_hash)),
        provider_email: Set(Some(format!("{user_id}@test.com"))),
        verification_token: Set(None),
        token_expires_at: Set(None),
        created_at: Set(now),
    };
    provider.insert(&state.db).await?;

    let token_pair = jwt::generate_token_pair(user_id, role, &state.config)?;

    Ok((user_model, token_pair.access_token))
}

// ──────────────────────────────────────────────────────────────────────────────
// Basic auth middleware tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn middleware_missing_header_returns_401() {
    let (app, _state) = test_app_with_middleware_routes().await;
    let (status, _body) = common::get(&app, "/test/user").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn middleware_invalid_token_returns_401() {
    let (app, _state) = test_app_with_middleware_routes().await;
    let (status, _body) = common::get_with_auth(&app, "/test/user", "invalid-jwt-token").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn middleware_valid_token_passes() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (user_model, token) = create_user(&state, "user", "active").await?;

    let (status, body) = common::get_with_auth(&app, "/test/user", &token).await;
    assert_eq!(status, StatusCode::OK, "middleware reject: {body}");

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["id"], user_model.id.to_string());
    assert_eq!(json["role"], "user");
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Account status tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn middleware_suspended_user_returns_403() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (_user, token) = create_user(&state, "user", "suspended").await?;

    let (status, body) = common::get_with_auth(&app, "/test/user", &token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("suspended")
    );
    Ok(())
}

#[tokio::test]
async fn middleware_deactivated_user_returns_403() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (_user, token) = create_user(&state, "user", "deactivated").await?;

    let (status, body) = common::get_with_auth(&app, "/test/user", &token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("deactivated")
    );
    Ok(())
}

#[tokio::test]
async fn middleware_deleted_user_returns_401() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (user_model, token) = create_user(&state, "user", "active").await?;

    // Soft-delete the user
    let now = Utc::now().fixed_offset();
    let mut active: user::ActiveModel = user_model.into();
    active.deleted_at = Set(Some(now));
    active.update(&state.db).await?;

    let (status, _body) = common::get_with_auth(&app, "/test/user", &token).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Role enforcement tests
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn role_user_cannot_access_moderator_route() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (_user, token) = create_user(&state, "user", "active").await?;

    let (status, _body) = common::get_with_auth(&app, "/test/moderator", &token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    Ok(())
}

#[tokio::test]
async fn role_user_cannot_access_admin_route() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (_user, token) = create_user(&state, "user", "active").await?;

    let (status, _body) = common::get_with_auth(&app, "/test/admin", &token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    Ok(())
}

#[tokio::test]
async fn role_moderator_can_access_moderator_route() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (_user, token) = create_user(&state, "moderator", "active").await?;

    let (status, _body) = common::get_with_auth(&app, "/test/moderator", &token).await;
    assert_eq!(status, StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn role_moderator_cannot_access_admin_route() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (_user, token) = create_user(&state, "moderator", "active").await?;

    let (status, _body) = common::get_with_auth(&app, "/test/admin", &token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    Ok(())
}

#[tokio::test]
async fn role_admin_can_access_all_routes() -> anyhow::Result<()> {
    let (app, state) = test_app_with_middleware_routes().await;
    let (_user, token) = create_user(&state, "admin", "active").await?;

    let (status1, _body1) = common::get_with_auth(&app, "/test/user", &token).await;
    assert_eq!(status1, StatusCode::OK);

    let (status2, _body2) = common::get_with_auth(&app, "/test/moderator", &token).await;
    assert_eq!(status2, StatusCode::OK);

    let (status3, _body3) = common::get_with_auth(&app, "/test/admin", &token).await;
    assert_eq!(status3, StatusCode::OK);
    Ok(())
}
