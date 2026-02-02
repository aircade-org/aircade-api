mod common;

use axum::Router;
use axum::http::StatusCode;
use migration::{Migrator, MigratorTrait};
use serde_json::json;

use aircade_api::config::{Config, Environment};
use aircade_api::sessions::SessionManager;
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
        session_manager: SessionManager::new(),
    };

    aircade_api::routes::router().with_state(state)
}

/// Sign up a user and return (`access_token`, `refresh_token`).
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
    (
        json["token"].as_str().unwrap_or_default().to_string(),
        json["refreshToken"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    )
}

/// Create a session and return the session response JSON.
async fn create_session(app: &Router, token: &str) -> serde_json::Value {
    let (status, body) =
        common::post_json_with_auth(app, "/api/v1/sessions", &json!({ "maxPlayers": 4 }), token)
            .await;
    assert_eq!(status, StatusCode::CREATED, "create session failed: {body}");
    serde_json::from_str(&body).unwrap_or_default()
}

// ──────────────────────────────────────────────────────────────────────────────
// POST /api/v1/sessions — Create Session
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_session_unauthenticated_returns_401() {
    let app = test_app().await;
    let (status, _body) =
        common::post_json(&app, "/api/v1/sessions", &json!({ "maxPlayers": 4 })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_session_success() {
    let app = test_app().await;
    let (token, _refresh) = signup_user(&app, "host@example.com", "hostuser", "Password123").await;

    let session_json = create_session(&app, &token).await;

    assert!(!session_json["id"].as_str().unwrap_or_default().is_empty());
    assert!(
        !session_json["sessionCode"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
    );
    assert_eq!(session_json["status"], "lobby");
    assert_eq!(session_json["maxPlayers"], 4);
    let empty_players: Vec<serde_json::Value> = vec![];
    assert_eq!(
        session_json["players"]
            .as_array()
            .unwrap_or(&empty_players)
            .len(),
        0
    );
}

#[tokio::test]
async fn create_session_default_max_players() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host2@example.com", "host2user", "Password123").await;

    let (status, body) =
        common::post_json_with_auth(&app, "/api/v1/sessions", &json!({}), &token).await;
    assert_eq!(status, StatusCode::CREATED);

    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(json["maxPlayers"], 8); // default
}

// ──────────────────────────────────────────────────────────────────────────────
// GET /api/v1/sessions/{sessionCode}
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_session_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host3@example.com", "host3user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let code = session_json["sessionCode"].as_str().unwrap_or_default();

    let (status, body) = common::get(&app, &format!("/api/v1/sessions/{code}")).await;
    assert_eq!(status, StatusCode::OK);

    let fetched: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(fetched["sessionCode"], code);
    assert_eq!(fetched["status"], "lobby");
}

#[tokio::test]
async fn get_session_case_insensitive() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host4@example.com", "host4user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let code = session_json["sessionCode"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();

    let (status, _body) = common::get(&app, &format!("/api/v1/sessions/{code}")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn get_session_not_found() {
    let app = test_app().await;
    let (status, _body) = common::get(&app, "/api/v1/sessions/ZZZZZ").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ──────────────────────────────────────────────────────────────────────────────
// POST /api/v1/sessions/{sessionCode}/join
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn join_session_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host5@example.com", "host5user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let code = session_json["sessionCode"].as_str().unwrap_or_default();

    let (status, body) = common::post_json(
        &app,
        &format!("/api/v1/sessions/{code}/join"),
        &json!({ "displayName": "Player One" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "join failed: {body}");

    let join_resp: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(join_resp["player"]["displayName"], "Player One");
    assert!(
        !join_resp["player"]["id"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
    );
    assert_eq!(join_resp["session"]["sessionCode"], code);
}

#[tokio::test]
async fn join_session_not_found() {
    let app = test_app().await;
    let (status, _body) = common::post_json(
        &app,
        "/api/v1/sessions/ZZZZZ/join",
        &json!({ "displayName": "Ghost" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn join_session_full() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host6@example.com", "host6user", "Password123").await;

    // Create session with max 1 player
    let (status, body) = common::post_json_with_auth(
        &app,
        "/api/v1/sessions",
        &json!({ "maxPlayers": 1 }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let session_json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let code = session_json["sessionCode"].as_str().unwrap_or_default();

    // First player joins — should succeed
    let (status, _body) = common::post_json(
        &app,
        &format!("/api/v1/sessions/{code}/join"),
        &json!({ "displayName": "Player 1" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Second player joins — should fail
    let (status, _body) = common::post_json(
        &app,
        &format!("/api/v1/sessions/{code}/join"),
        &json!({ "displayName": "Player 2" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn join_session_empty_display_name() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host7@example.com", "host7user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let code = session_json["sessionCode"].as_str().unwrap_or_default();

    let (status, _body) = common::post_json(
        &app,
        &format!("/api/v1/sessions/{code}/join"),
        &json!({ "displayName": "" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// GET /api/v1/sessions/{sessionId}/players
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_players_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host8@example.com", "host8user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();
    let code = session_json["sessionCode"].as_str().unwrap_or_default();

    // Join two players
    common::post_json(
        &app,
        &format!("/api/v1/sessions/{code}/join"),
        &json!({ "displayName": "Alice" }),
    )
    .await;
    common::post_json(
        &app,
        &format!("/api/v1/sessions/{code}/join"),
        &json!({ "displayName": "Bob" }),
    )
    .await;

    let (status, body) = common::get(&app, &format!("/api/v1/sessions/{session_id}/players")).await;
    assert_eq!(status, StatusCode::OK);

    let players: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(players.len(), 2);
}

#[tokio::test]
async fn list_players_session_not_found() {
    let app = test_app().await;
    let fake_id = "00000000-0000-0000-0000-000000000099";
    let (status, _body) = common::get(&app, &format!("/api/v1/sessions/{fake_id}/players")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ──────────────────────────────────────────────────────────────────────────────
// POST /api/v1/sessions/{sessionId}/end
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn end_session_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host9@example.com", "host9user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();

    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/end"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify session is ended
    let code = session_json["sessionCode"].as_str().unwrap_or_default();
    let (status, body) = common::get(&app, &format!("/api/v1/sessions/{code}")).await;
    assert_eq!(status, StatusCode::OK);
    let fetched: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(fetched["status"], "ended");
}

#[tokio::test]
async fn end_session_not_host_returns_403() {
    let app = test_app().await;
    let (host_token, _) =
        signup_user(&app, "host10@example.com", "host10user", "Password123").await;
    let (other_token, _) = signup_user(&app, "other@example.com", "otheruser", "Password123").await;

    let session_json = create_session(&app, &host_token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();

    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/end"),
        &json!({}),
        &other_token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn end_session_already_ended() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host11@example.com", "host11user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();

    // End once
    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/end"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // End again — should fail
    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/end"),
        &json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// POST /api/v1/sessions/{sessionId}/game — Load Game
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn load_game_success() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host12@example.com", "host12user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();

    // Use the seeded Pong game ID
    let pong_game_id = "00000000-0000-0000-0000-000000000010";

    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/game"),
        &json!({ "gameId": pong_game_id }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "load game failed: {body}");

    let resp: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(resp["gameId"], pong_game_id);
    assert_eq!(resp["status"], "playing");
    assert!(
        !resp["gameVersionId"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
    );
}

#[tokio::test]
async fn load_game_not_host_returns_403() {
    let app = test_app().await;
    let (host_token, _) =
        signup_user(&app, "host13@example.com", "host13user", "Password123").await;
    let (other_token, _) =
        signup_user(&app, "other2@example.com", "other2user", "Password123").await;

    let session_json = create_session(&app, &host_token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();
    let pong_game_id = "00000000-0000-0000-0000-000000000010";

    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/game"),
        &json!({ "gameId": pong_game_id }),
        &other_token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn load_game_not_found() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host14@example.com", "host14user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();
    let fake_game_id = "00000000-0000-0000-0000-000000000099";

    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/game"),
        &json!({ "gameId": fake_game_id }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn load_game_on_ended_session() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host15@example.com", "host15user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();

    // End the session
    common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/end"),
        &json!({}),
        &token,
    )
    .await;

    let pong_game_id = "00000000-0000-0000-0000-000000000010";
    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/game"),
        &json!({ "gameId": pong_game_id }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ──────────────────────────────────────────────────────────────────────────────
// Session code format validation
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn session_code_format() {
    let app = test_app().await;
    let (token, _refresh) =
        signup_user(&app, "host16@example.com", "host16user", "Password123").await;

    let session_json = create_session(&app, &token).await;
    let code = session_json["sessionCode"].as_str().unwrap_or_default();

    // Code should be 5 uppercase alphanumeric characters
    assert_eq!(code.len(), 5);
    assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
    assert!(
        code.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    );

    // Should not contain ambiguous characters
    let ambiguous = ['0', 'O', '1', 'I', 'L'];
    assert!(!code.chars().any(|c| ambiguous.contains(&c)));
}

// ──────────────────────────────────────────────────────────────────────────────
// Full flow: create → join → load game
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn full_session_flow() {
    let app = test_app().await;
    let (host_token, _) =
        signup_user(&app, "flow_host@example.com", "flowhost", "Password123").await;

    // 1. Create session
    let session_json = create_session(&app, &host_token).await;
    let session_id = session_json["id"].as_str().unwrap_or_default();
    let code = session_json["sessionCode"].as_str().unwrap_or_default();
    assert_eq!(session_json["status"], "lobby");

    // 2. Player joins
    let (status, body) = common::post_json(
        &app,
        &format!("/api/v1/sessions/{code}/join"),
        &json!({ "displayName": "Pong Player" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let join_resp: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let player_id = join_resp["player"]["id"].as_str().unwrap_or_default();
    assert!(!player_id.is_empty());

    // 3. Session now shows 1 player
    let (status, body) = common::get(&app, &format!("/api/v1/sessions/{code}")).await;
    assert_eq!(status, StatusCode::OK);
    let session_detail: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let empty_players: Vec<serde_json::Value> = vec![];
    assert_eq!(
        session_detail["players"]
            .as_array()
            .unwrap_or(&empty_players)
            .len(),
        1
    );

    // 4. Load Pong game
    let pong_game_id = "00000000-0000-0000-0000-000000000010";
    let (status, body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/game"),
        &json!({ "gameId": pong_game_id }),
        &host_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "load game failed: {body}");
    let load_resp: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(load_resp["status"], "playing");

    // 5. Session shows "playing" status
    let (status, body) = common::get(&app, &format!("/api/v1/sessions/{code}")).await;
    assert_eq!(status, StatusCode::OK);
    let updated_session: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(updated_session["status"], "playing");
    assert_eq!(
        updated_session["gameId"].as_str().unwrap_or_default(),
        pong_game_id
    );

    // 6. End session
    let (status, _body) = common::post_json_with_auth(
        &app,
        &format!("/api/v1/sessions/{session_id}/end"),
        &json!({}),
        &host_token,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 7. Session shows "ended"
    let (status, body) = common::get(&app, &format!("/api/v1/sessions/{code}")).await;
    assert_eq!(status, StatusCode::OK);
    let ended_session: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(ended_session["status"], "ended");
}
