use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ============ Request DTOs ============

/// POST /api/games - Create game request
#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    /// Maximum number of players (2-16, default: 8)
    pub max_players: Option<u8>,
    /// Game type identifier
    pub game_type: Option<String>,
    /// Additional game-specific settings (stored as JSON)
    #[allow(dead_code)]
    pub settings: Option<JsonValue>,
}

/// POST /api/games/:code/join - Join game request
#[derive(Debug, Deserialize)]
pub struct JoinGameRequest {
    /// Player's display name (2-20 chars)
    pub nickname: String,
}

/// POST /api/games/:id/start - Start game request (optional body)
#[derive(Debug, Deserialize, Default)]
pub struct StartGameRequest {
    /// Optional override settings for game start
    #[serde(default)]
    #[allow(dead_code)]
    pub settings: Option<JsonValue>,
}

// ============ Response DTOs ============

/// Game creation response
#[derive(Debug, Serialize)]
pub struct CreateGameResponse {
    pub id: i32,
    pub code: String,
    pub status: String,
    pub host_id: i32,
    pub created_at: chrono::NaiveDateTime,
}

/// Game details response
#[derive(Debug, Serialize)]
pub struct GameResponse {
    pub id: i32,
    pub code: String,
    pub status: String,
    pub host_id: i32,
    pub settings: Option<JsonValue>,
    pub created_at: chrono::NaiveDateTime,
    pub player_count: usize,
    pub max_players: u8,
}

/// Game status change response
#[derive(Debug, Serialize)]
pub struct GameStatusResponse {
    pub id: i32,
    pub status: String,
    pub message: String,
}

/// Game settings stored in JSON column
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameSettings {
    pub max_players: u8,
    pub game_type: String,
    #[serde(flatten)]
    pub extra: Option<JsonValue>,
}

impl GameSettings {
    pub fn new(max_players: Option<u8>, game_type: Option<String>) -> Self {
        Self {
            max_players: max_players.unwrap_or(8).clamp(2, 16),
            game_type: game_type.unwrap_or_else(|| "default".to_string()),
            extra: None,
        }
    }
}
