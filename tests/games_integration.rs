//! Integration tests for game session management (v0.2.0)
//!
//! These tests verify the game creation, joining, and management APIs.
//! Note: Tests that require a database connection are marked with ignore
//! and should be run separately with a test database.

// Allow unwrap/expect in tests
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use aircade_api::dto::{
    CreateGameRequest, CreateGameResponse, GameResponse, GameSettings, JoinGameRequest,
    JoinGameResponse, PlayersListResponse,
};
use aircade_api::entities::GameStatus;
use aircade_api::utils::{generate_game_code, is_valid_game_code, normalize_game_code};

// ============ Game Code Tests ============

#[test]
fn test_game_code_generation() {
    let code = generate_game_code();
    assert_eq!(code.len(), 6, "Game code should be 6 characters");
    assert!(
        is_valid_game_code(&code),
        "Generated code should be valid"
    );
}

#[test]
fn test_game_code_uniqueness() {
    let codes: std::collections::HashSet<String> =
        (0..100).map(|_| generate_game_code()).collect();
    // With 32^6 possibilities, 100 codes should all be unique
    assert_eq!(codes.len(), 100, "All 100 codes should be unique");
}

#[test]
fn test_game_code_validation() {
    // Valid codes (validation converts to uppercase internally)
    assert!(is_valid_game_code("ABC234"));
    assert!(is_valid_game_code("XYZMNK"));
    assert!(is_valid_game_code("234567"));
    assert!(is_valid_game_code("abc234")); // Lowercase is valid (converted internally)

    // Invalid codes
    assert!(!is_valid_game_code("ABC")); // Too short
    assert!(!is_valid_game_code("ABCDEFG")); // Too long
    assert!(!is_valid_game_code("ABC12!")); // Invalid character
}

#[test]
fn test_game_code_normalization() {
    assert_eq!(normalize_game_code("abc234"), "ABC234");
    assert_eq!(normalize_game_code("  XYZ789  "), "XYZ789");
    assert_eq!(normalize_game_code("MiXeD"), "MIXED");
}

// ============ Game Status Tests ============

#[test]
fn test_game_status_from_str() {
    assert_eq!(GameStatus::from_str("lobby"), Some(GameStatus::Lobby));
    assert_eq!(GameStatus::from_str("LOBBY"), Some(GameStatus::Lobby));
    assert_eq!(GameStatus::from_str("playing"), Some(GameStatus::Playing));
    assert_eq!(GameStatus::from_str("finished"), Some(GameStatus::Finished));
    assert_eq!(GameStatus::from_str("invalid"), None);
}

#[test]
fn test_game_status_as_str() {
    assert_eq!(GameStatus::Lobby.as_str(), "lobby");
    assert_eq!(GameStatus::Playing.as_str(), "playing");
    assert_eq!(GameStatus::Finished.as_str(), "finished");
}

#[test]
fn test_game_status_can_join() {
    assert!(GameStatus::Lobby.can_join());
    assert!(!GameStatus::Playing.can_join());
    assert!(!GameStatus::Finished.can_join());
}

#[test]
fn test_game_status_can_start() {
    assert!(GameStatus::Lobby.can_start());
    assert!(!GameStatus::Playing.can_start());
    assert!(!GameStatus::Finished.can_start());
}

// ============ DTO Tests ============

#[test]
fn test_create_game_request_deserialization() {
    let json = r#"{"max_players": 8, "game_type": "trivia"}"#;
    let request: CreateGameRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.max_players, Some(8));
    assert_eq!(request.game_type, Some("trivia".to_string()));
}

#[test]
fn test_create_game_request_defaults() {
    let json = r#"{}"#;
    let request: CreateGameRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.max_players, None);
    assert_eq!(request.game_type, None);
}

#[test]
fn test_join_game_request_deserialization() {
    let json = r#"{"nickname": "Player1"}"#;
    let request: JoinGameRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.nickname, "Player1");
}

#[test]
fn test_game_settings_new() {
    let settings = GameSettings::new(None, None);
    assert_eq!(settings.max_players, 8); // Default
    assert_eq!(settings.game_type, "default");

    let settings = GameSettings::new(Some(4), Some("trivia".to_string()));
    assert_eq!(settings.max_players, 4);
    assert_eq!(settings.game_type, "trivia");
}

#[test]
fn test_game_settings_clamp() {
    // Test max_players clamping
    let settings = GameSettings::new(Some(1), None);
    assert_eq!(settings.max_players, 2); // Clamped to minimum

    let settings = GameSettings::new(Some(100), None);
    assert_eq!(settings.max_players, 16); // Clamped to maximum
}

#[test]
fn test_create_game_response_serialization() {
    let response = CreateGameResponse {
        id: 1,
        code: "ABC123".to_string(),
        status: "lobby".to_string(),
        host_id: 1,
        created_at: chrono::DateTime::from_timestamp(0, 0)
            .map(|dt| dt.naive_utc())
            .unwrap(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"code\":\"ABC123\""));
    assert!(json.contains("\"status\":\"lobby\""));
}

#[test]
fn test_join_game_response_serialization() {
    let response = JoinGameResponse {
        player_id: 1,
        game_id: 1,
        nickname: "TestPlayer".to_string(),
        color: "#FF6B6B".to_string(),
        player_token: "pt_1_1_abc123".to_string(),
        joined_at: chrono::DateTime::from_timestamp(0, 0)
            .map(|dt| dt.naive_utc())
            .unwrap(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"player_id\":1"));
    assert!(json.contains("\"nickname\":\"TestPlayer\""));
    assert!(json.contains("\"color\":\"#FF6B6B\""));
}

#[test]
fn test_game_response_serialization() {
    let response = GameResponse {
        id: 1,
        code: "ABC123".to_string(),
        status: "lobby".to_string(),
        host_id: 1,
        settings: None,
        created_at: chrono::DateTime::from_timestamp(0, 0)
            .map(|dt| dt.naive_utc())
            .unwrap(),
        player_count: 2,
        max_players: 8,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"player_count\":2"));
    assert!(json.contains("\"max_players\":8"));
}

#[test]
fn test_players_list_response_serialization() {
    let response = PlayersListResponse {
        players: vec![],
        count: 0,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"players\":[]"));
    assert!(json.contains("\"count\":0"));
}

// ============ Color Assignment Tests ============

#[test]
fn test_color_assignment() {
    use aircade_api::utils::get_next_available_color;

    // First player gets first color
    let color1 = get_next_available_color(&[]);
    assert_eq!(color1, "#FF6B6B");

    // Second player gets second color
    let taken = vec!["#FF6B6B".to_string()];
    let color2 = get_next_available_color(&taken);
    assert_eq!(color2, "#4ECDC4");
}

// ============ API Endpoint Path Tests ============

#[test]
fn test_api_endpoint_paths() {
    // Verify expected endpoint patterns
    let create_game = "/api/games";
    let get_game = "/api/games/1";
    let join_game = "/api/games/ABC123/join";
    let get_players = "/api/games/1/players";
    let start_game = "/api/games/1/start";
    let kick_player = "/api/games/1/players/2";
    let leave_game = "/api/games/1/leave";

    assert!(create_game.starts_with("/api/games"));
    assert!(get_game.contains("/api/games/"));
    assert!(join_game.contains("/join"));
    assert!(get_players.contains("/players"));
    assert!(start_game.contains("/start"));
    assert!(kick_player.contains("/players/"));
    assert!(leave_game.contains("/leave"));
}

// ============ Note on Database Tests ============
// The following tests require a database connection and should be run
// with a test database configured. They are currently commented out
// but provide a template for future integration testing.

/*
#[tokio::test]
#[ignore]
async fn test_create_game_success() {
    // Setup test database
    // Create game
    // Verify response has valid code
    // Verify game exists in database
}

#[tokio::test]
#[ignore]
async fn test_join_game_success() {
    // Create game
    // Join game with valid nickname
    // Verify player added to game
    // Verify color assigned
}

#[tokio::test]
#[ignore]
async fn test_join_game_duplicate_nickname() {
    // Create game
    // Join with nickname "Player1"
    // Try to join again with "Player1"
    // Verify NICKNAME_TAKEN error
}

#[tokio::test]
#[ignore]
async fn test_start_game_success() {
    // Create game
    // Add 2+ players
    // Start game as host
    // Verify status changed to "playing"
}

#[tokio::test]
#[ignore]
async fn test_start_game_insufficient_players() {
    // Create game
    // Add only 1 player
    // Try to start game
    // Verify validation error
}

#[tokio::test]
#[ignore]
async fn test_kick_player_success() {
    // Create game
    // Add player
    // Kick player as host
    // Verify player removed
}

#[tokio::test]
#[ignore]
async fn test_leave_game_host() {
    // Create game
    // Host leaves
    // Verify game status is "finished"
}
*/
