mod common;

use axum::http::StatusCode;
use axum::Router;
use migration::{Migrator, MigratorTrait};
use serde_json::json;

use aircade_api::config::{Config, Environment};
use aircade_api::sessions::SessionManager;
use aircade_api::state::AppState;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

// ─────────────────────────────────────────────────────────────────────────────
// Test Infrastructure
// ─────────────────────────────────────────────────────────────────────────────

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
        session_manager: SessionManager::new(),
    };

    aircade_api::routes::router().with_state(state)
}

/// Sign up a new user and return (`access_token`, `user_id`).
async fn signup_and_get_token(app: &Router, suffix: &str) -> (String, String) {
    let email = format!("creator{suffix}@example.com");
    let username = format!("creator{suffix}");
    let (status, body) = common::post_json(
        app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": email,
            "username": username,
            "password": "SecurePass123!",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "signup failed: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let token = v["token"].as_str().unwrap_or_default().to_string();
    let user_id = v["user"]["id"].as_str().unwrap_or_default().to_string();
    (token, user_id)
}

/// Create a game for a user and return the game ID.
async fn create_game(app: &Router, token: &str, title: &str) -> String {
    let (status, body) =
        common::post_json_with_auth(app, "/api/v1/games", &json!({ "title": title }), token).await;
    assert_eq!(status, StatusCode::CREATED, "create game failed: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    v["id"].as_str().unwrap_or_default().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.1 Create Game
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_game_success() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "cg1").await;

    let (status, body) = common::post_json_with_auth(
        &app,
        "/api/v1/games",
        &json!({
            "title": "Space Race",
            "description": "Race through asteroids!",
            "minPlayers": 2,
            "maxPlayers": 8,
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["title"], "Space Race");
    assert_eq!(v["status"], "draft");
    assert_eq!(v["visibility"], "private");
    assert_eq!(v["minPlayers"], 2);
    assert_eq!(v["maxPlayers"], 8);
    assert!(v["id"].is_string());
}

#[tokio::test]
async fn create_game_unauthenticated() {
    let app = test_app().await;

    let (status, _) = common::post_json(&app, "/api/v1/games", &json!({ "title": "Test" })).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_game_empty_title() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "cg2").await;

    let (status, body) =
        common::post_json_with_auth(&app, "/api/v1/games", &json!({ "title": "" }), &token).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn create_game_invalid_player_counts() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "cg3").await;

    let (status, body) = common::post_json_with_auth(
        &app,
        "/api/v1/games",
        &json!({ "title": "Bad Game", "minPlayers": 5, "maxPlayers": 2 }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.2 Get Game
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_game_private_as_owner() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "gg1").await;
    let game_id = create_game(&app, &token, "My Private Game").await;

    let (status, body) =
        common::get_with_auth(&app, &format!("/api/v1/games/{game_id}"), &token).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["id"], game_id);
    assert_eq!(v["title"], "My Private Game");
    assert!(v["creator"].is_object());
}

#[tokio::test]
async fn get_game_private_as_stranger_is_404() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "gg2").await;
    let game_id = create_game(&app, &token, "Hidden Game").await;

    let (token2, _) = signup_and_get_token(&app, "gg2b").await;
    let (status, _) =
        common::get_with_auth(&app, &format!("/api/v1/games/{game_id}"), &token2).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_game_not_found() {
    let app = test_app().await;
    let fake_id = uuid::Uuid::new_v4();
    let (status, _) = common::get(&app, &format!("/api/v1/games/{fake_id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.3 Update Game
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_game_success() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "ug1").await;
    let game_id = create_game(&app, &token, "Draft Game").await;

    let (status, body) = common::patch_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}"),
        &json!({
            "title": "Updated Title",
            "visibility": "public",
            "gameScreenCode": "function setup() {}",
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["title"], "Updated Title");
    assert_eq!(v["visibility"], "public");
    assert_eq!(v["gameScreenCode"], "function setup() {}");
}

#[tokio::test]
async fn update_game_forbidden_for_non_creator() {
    let app = test_app().await;
    let (token1, _) = signup_and_get_token(&app, "ug2").await;
    let (token2, _) = signup_and_get_token(&app, "ug2b").await;
    let game_id = create_game(&app, &token1, "Creator Game").await;

    let (status, _) = common::patch_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}"),
        &json!({ "title": "Hijacked" }),
        &token2,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.4 Delete Game
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_game_success() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "dg1").await;
    let game_id = create_game(&app, &token, "To Delete").await;

    let (status, _) =
        common::delete_with_auth(&app, &format!("/api/v1/games/{game_id}"), &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Game should no longer be accessible
    let (status, _) =
        common::get_with_auth(&app, &format!("/api/v1/games/{game_id}"), &token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_game_forbidden_for_non_creator() {
    let app = test_app().await;
    let (token1, _) = signup_and_get_token(&app, "dg2").await;
    let (token2, _) = signup_and_get_token(&app, "dg2b").await;
    let game_id = create_game(&app, &token1, "Protected Game").await;

    let (status, _) =
        common::delete_with_auth(&app, &format!("/api/v1/games/{game_id}"), &token2).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.5 Publish Game
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn publish_game_email_not_verified() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "pg1").await;
    let game_id = create_game(&app, &token, "Unpublishable").await;

    // Add some code
    let _ = common::patch_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}"),
        &json!({ "gameScreenCode": "function setup() { createCanvas(400, 400); }" }),
        &token,
    )
    .await;

    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/publish"),
        &json!({}),
        &token,
    )
    .await;

    // Email not verified → should be 422
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["error"]["code"], "EMAIL_NOT_VERIFIED");
}

#[tokio::test]
async fn publish_game_no_code() {
    // We need to manually verify the user for this test
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap_or_default();
    Migrator::up(&db, None).await.unwrap_or_default();

    let state = AppState {
        db: db.clone(),
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
        session_manager: SessionManager::new(),
    };
    let app = aircade_api::routes::router().with_state(state);

    // Sign up
    let (status, body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": "verified@example.com",
            "username": "verifieduser",
            "password": "SecurePass123!",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let token = v["token"].as_str().unwrap_or_default().to_string();
    let user_id: uuid::Uuid = v["user"]["id"]
        .as_str()
        .unwrap_or_default()
        .parse()
        .unwrap_or_default();

    // Manually set email_verified = true
    let Some(user) = aircade_api::entities::user::Entity::find_by_id(user_id)
        .one(&db)
        .await
        .unwrap_or_default()
    else {
        return;
    };
    let mut active: aircade_api::entities::user::ActiveModel = user.into();
    active.email_verified = ActiveValue::Set(true);
    let _ = active.update(&db).await.ok();

    // Create a game (no code)
    let game_id = create_game(&app, &token, "Empty Code Game").await;

    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/publish"),
        &json!({}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["error"]["code"], "INVALID_GAME");
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: create a verified user + published game
// ─────────────────────────────────────────────────────────────────────────────

async fn setup_verified_user_and_published_game(suffix: &str) -> (Router, String, String, String) {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap_or_default();
    Migrator::up(&db, None).await.unwrap_or_default();

    let state = AppState {
        db: db.clone(),
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
        session_manager: SessionManager::new(),
    };
    let app = aircade_api::routes::router().with_state(state);

    // Sign up user
    let (status, body) = common::post_json(
        &app,
        "/api/v1/auth/signup/email",
        &json!({
            "email": format!("pub{suffix}@example.com"),
            "username": format!("pubuser{suffix}"),
            "password": "SecurePass123!",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "signup: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let token = v["token"].as_str().unwrap_or_default().to_string();
    let user_id: uuid::Uuid = v["user"]["id"]
        .as_str()
        .unwrap_or_default()
        .parse()
        .unwrap_or_default();

    // Mark email verified
    let Some(user) = aircade_api::entities::user::Entity::find_by_id(user_id)
        .one(&db)
        .await
        .unwrap_or_default()
    else {
        return (app, String::new(), String::new(), String::new());
    };
    let mut active: aircade_api::entities::user::ActiveModel = user.into();
    active.email_verified = ActiveValue::Set(true);
    let _ = active.update(&db).await.ok();

    // Create game with code
    let game_id = create_game(&app, &token, &format!("Game {suffix}")).await;
    let _ = common::patch_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}"),
        &json!({
            "gameScreenCode": "function setup() { createCanvas(400, 400); }",
            "visibility": "public",
        }),
        &token,
    )
    .await;

    // Publish the game
    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/publish"),
        &json!({ "changelog": "Initial release" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "publish: {body}");

    let username = format!("pubuser{suffix}");
    (app, token, game_id, username)
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.6 Archive / 4.7 Unarchive Game
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn archive_and_unarchive_game() {
    let (app, token, game_id, _) = setup_verified_user_and_published_game("au1").await;

    // Archive
    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/archive"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "archive: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["status"], "archived");

    // Archive again → 422
    let (status, _) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/archive"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Unarchive
    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/unarchive"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "unarchive: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    // Was published before archiving, so should restore to published
    assert_eq!(v["status"], "published");

    // Unarchive when not archived → 422
    let (status, _) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/unarchive"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.8 Fork Game
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fork_game_success() {
    let (app, _token1, game_id, _) = setup_verified_user_and_published_game("fk1").await;

    // A second user forks the game
    let (token2, _) = signup_and_get_token(&app, "fk1b").await;

    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/fork"),
        &json!({}),
        &token2,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["forkedFromId"], game_id);
    assert_eq!(v["status"], "draft");
    assert_eq!(v["visibility"], "private");
}

#[tokio::test]
async fn fork_game_no_published_version() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "fk2").await;
    let game_id = create_game(&app, &token, "Unpublished Game").await;

    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/fork"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.9 / 4.10 Versions
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_and_get_versions() {
    let (app, token, game_id, _) = setup_verified_user_and_published_game("v1").await;

    // List versions
    let (status, body) =
        common::get_with_auth(&app, &format!("/api/v1/games/{game_id}/versions"), &token).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["total"], 1);
    assert_eq!(v["data"][0]["versionNumber"], 1);
    assert_eq!(v["data"][0]["changelog"], "Initial release");

    // Get specific version
    let (status, body) =
        common::get_with_auth(&app, &format!("/api/v1/games/{game_id}/versions/1"), &token).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["versionNumber"], 1);
    assert!(v["gameScreenCode"].is_string());

    // Get non-existent version
    let (status, _) = common::get_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/versions/99"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.15 – 4.17 Tags
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_tags_no_auth() {
    let app = test_app().await;
    let (status, body) = common::get(&app, "/api/v1/tags").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    // Seeded tags exist
    assert!(v["data"].as_array().is_some_and(|a| !a.is_empty()));
}

#[tokio::test]
async fn list_tags_filtered_by_category() {
    let app = test_app().await;
    let (status, body) = common::get(&app, "/api/v1/tags?category=genre").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let data: Vec<serde_json::Value> = v["data"].as_array().cloned().unwrap_or_default();
    assert!(!data.is_empty());
    for tag in &data {
        assert_eq!(tag["category"], "genre");
    }
}

#[tokio::test]
async fn set_and_get_game_tags() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "tg1").await;
    let game_id = create_game(&app, &token, "Tagged Game").await;

    // Get all tags to find valid IDs
    let (_, tags_body) = common::get(&app, "/api/v1/tags?category=genre").await;
    let tags_v: serde_json::Value = serde_json::from_str(&tags_body).unwrap_or_default();
    let tag_ids: Vec<String> = tags_v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .take(2)
                .filter_map(|t| t["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    assert!(tag_ids.len() >= 2, "Need at least 2 seeded tags");

    // Set game tags
    let (status, body) = common::put_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/tags"),
        &json!({ "tagIds": tag_ids }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["tags"].as_array().map_or(0, Vec::len), 2);

    // Get game tags (public)
    let (status, body) = common::get(&app, &format!("/api/v1/games/{game_id}/tags")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["tags"].as_array().map_or(0, Vec::len), 2);

    // Replace with 1 tag
    let (status, body) = common::put_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/tags"),
        &json!({ "tagIds": [tag_ids[0]] }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["tags"].as_array().map_or(0, Vec::len), 1);
}

#[tokio::test]
async fn set_game_tags_invalid_id() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "tg2").await;
    let game_id = create_game(&app, &token, "Game With Bad Tag").await;

    let fake_tag_id = uuid::Uuid::new_v4().to_string();
    let (status, _) = common::put_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/tags"),
        &json!({ "tagIds": [fake_tag_id] }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.18 List My Games
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_my_games() {
    let app = test_app().await;
    let (token, _) = signup_and_get_token(&app, "mg1").await;

    // Create 2 games
    let _ = create_game(&app, &token, "My Game 1").await;
    let _ = create_game(&app, &token, "My Game 2").await;

    let (status, body) = common::get_with_auth(&app, "/api/v1/users/me/games", &token).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["total"], 2);
    assert_eq!(v["data"].as_array().map_or(0, Vec::len), 2);
    // Source code should NOT be in list response
    assert!(v["data"][0]["gameScreenCode"].is_null());
}

#[tokio::test]
async fn list_my_games_filtered_by_status() {
    let (app, token, _, _) = setup_verified_user_and_published_game("mg2").await;

    // We have 1 published game; add a draft
    let _ = create_game(&app, &token, "Draft Game").await;

    // Filter by draft
    let (status, body) =
        common::get_with_auth(&app, "/api/v1/users/me/games?status=draft", &token).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["total"], 1);
    assert_eq!(v["data"][0]["status"], "draft");
}

#[tokio::test]
async fn list_my_games_unauthenticated() {
    let app = test_app().await;
    let (status, _) = common::get(&app, "/api/v1/users/me/games").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4.19 List User's Public Games
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_user_public_games() {
    let (app, _token, _game_id, username) = setup_verified_user_and_published_game("upg1").await;

    let (status, body) = common::get(&app, &format!("/api/v1/users/{username}/games")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["total"], 1);
    // Draft game should not appear
    assert_eq!(v["data"][0]["status"], "published");
}

#[tokio::test]
async fn list_user_public_games_not_found() {
    let app = test_app().await;
    let (status, _) = common::get(&app, "/api/v1/users/nobody/games").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─────────────────────────────────────────────────────────────────────────────
// Get game includes tags in response
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_game_includes_tags() {
    let (app, token, game_id, _) = setup_verified_user_and_published_game("gt1").await;

    // Set a tag first
    let (_, tags_body) = common::get(&app, "/api/v1/tags?category=genre").await;
    let tags_v: serde_json::Value = serde_json::from_str(&tags_body).unwrap_or_default();
    let tag_id = tags_v["data"][0]["id"].as_str().unwrap_or_default();

    let _ = common::put_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}/tags"),
        &json!({ "tagIds": [tag_id] }),
        &token,
    )
    .await;

    // Make game public to access without auth
    let _ = common::patch_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}"),
        &json!({ "visibility": "public" }),
        &token,
    )
    .await;

    let (status, body) = common::get(&app, &format!("/api/v1/games/{game_id}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert!(v["tags"].is_array());
    assert!(v["tags"].as_array().is_some_and(|a| !a.is_empty()));
}

// ─────────────────────────────────────────────────────────────────────────────
// Code visibility rules
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_game_code_hidden_for_non_creator() {
    let (app, token1, game_id, _) = setup_verified_user_and_published_game("cv1").await;

    // Make the game public
    let _ = common::patch_json_with_auth(
        &app,
        &format!("/api/v1/games/{game_id}"),
        &json!({ "visibility": "public" }),
        &token1,
    )
    .await;

    // Another user fetches the game
    let (token2, _) = signup_and_get_token(&app, "cv1b").await;
    let (status, body) =
        common::get_with_auth(&app, &format!("/api/v1/games/{game_id}"), &token2).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    // Code fields should be absent for non-creator
    assert!(v.get("gameScreenCode").is_none() || v["gameScreenCode"].is_null());
}
