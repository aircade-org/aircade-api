use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

/// API error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error code for client handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Application error types
#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    // === General errors ===
    /// Database operation failed
    Database(String),
    /// Configuration error
    Config(String),
    /// Validation error
    Validation(String),
    /// Resource not found
    NotFound(String),
    /// Internal server error
    Internal(String),

    // === Game-specific errors (v0.2.0) ===
    /// Game not found by ID or code
    GameNotFound(String),
    /// Game is full, cannot join
    GameFull { game_code: String, max_players: u8 },
    /// Invalid game code format
    InvalidGameCode(String),
    /// Game status doesn't allow this operation
    InvalidGameState { current: String, required: String },
    /// Player not found in game
    PlayerNotFound { player_id: i32, game_id: i32 },
    /// Nickname already taken in game
    NicknameTaken { nickname: String, game_code: String },
    /// User already in game
    AlreadyInGame { user_id: i32, game_code: String },
    /// Not authorized (not host for host-only action)
    NotHost { action: String },
    /// Invalid nickname format
    InvalidNickname(String),
    /// Player token invalid or expired
    InvalidPlayerToken,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // General errors
            Self::Database(msg) => write!(f, "Database error: {msg}"),
            Self::Config(msg) => write!(f, "Configuration error: {msg}"),
            Self::Validation(msg) => write!(f, "Validation error: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),

            // Game-specific errors
            Self::GameNotFound(code) => write!(f, "Game not found: {code}"),
            Self::GameFull {
                game_code,
                max_players,
            } => {
                write!(f, "Game '{game_code}' is full (max {max_players} players)")
            }
            Self::InvalidGameCode(code) => write!(f, "Invalid game code: {code}"),
            Self::InvalidGameState { current, required } => {
                write!(
                    f,
                    "Invalid game state: current '{current}', required '{required}'"
                )
            }
            Self::PlayerNotFound { player_id, game_id } => {
                write!(f, "Player {player_id} not found in game {game_id}")
            }
            Self::NicknameTaken {
                nickname,
                game_code,
            } => {
                write!(
                    f,
                    "Nickname '{nickname}' already taken in game '{game_code}'"
                )
            }
            Self::AlreadyInGame { user_id, game_code } => {
                write!(f, "User {user_id} already in game '{game_code}'")
            }
            Self::NotHost { action } => {
                write!(f, "Only the host can perform: {action}")
            }
            Self::InvalidNickname(reason) => write!(f, "Invalid nickname: {reason}"),
            Self::InvalidPlayerToken => write!(f, "Invalid or expired player token"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            // General errors
            Self::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                msg.clone(),
            ),
            Self::Config(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_ERROR",
                msg.clone(),
            ),
            Self::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            Self::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),

            // Game-specific errors
            Self::GameNotFound(code) => (
                StatusCode::NOT_FOUND,
                "GAME_NOT_FOUND",
                format!("Game with code '{code}' not found"),
            ),
            Self::GameFull {
                game_code,
                max_players,
            } => (
                StatusCode::CONFLICT,
                "GAME_FULL",
                format!("Game '{game_code}' is full ({max_players} players max)"),
            ),
            Self::InvalidGameCode(code) => (
                StatusCode::BAD_REQUEST,
                "INVALID_GAME_CODE",
                format!("Invalid game code format: '{code}'"),
            ),
            Self::InvalidGameState { current, required } => (
                StatusCode::CONFLICT,
                "INVALID_GAME_STATE",
                format!("Game is in '{current}' state, but '{required}' is required"),
            ),
            Self::PlayerNotFound { player_id, game_id } => (
                StatusCode::NOT_FOUND,
                "PLAYER_NOT_FOUND",
                format!("Player {player_id} not found in game {game_id}"),
            ),
            Self::NicknameTaken {
                nickname,
                game_code,
            } => (
                StatusCode::CONFLICT,
                "NICKNAME_TAKEN",
                format!("Nickname '{nickname}' is already taken in game '{game_code}'"),
            ),
            Self::AlreadyInGame { user_id, game_code } => (
                StatusCode::CONFLICT,
                "ALREADY_IN_GAME",
                format!("User {user_id} is already in game '{game_code}'"),
            ),
            Self::NotHost { action } => (
                StatusCode::FORBIDDEN,
                "NOT_HOST",
                format!("Only the host can {action}"),
            ),
            Self::InvalidNickname(reason) => (
                StatusCode::BAD_REQUEST,
                "INVALID_NICKNAME",
                format!("Invalid nickname: {reason}"),
            ),
            Self::InvalidPlayerToken => (
                StatusCode::UNAUTHORIZED,
                "INVALID_PLAYER_TOKEN",
                "Invalid or expired player token".to_string(),
            ),
        };

        let error_response = ErrorResponse {
            code: code.to_string(),
            message,
            details: None,
        };

        tracing::error!("API error: {}", self);

        (status, Json(error_response)).into_response()
    }
}

/// Convert `SeaORM` database errors to `AppError`
impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        Self::Database(err.to_string())
    }
}

/// Convert anyhow errors to `AppError`
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}
