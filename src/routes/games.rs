#![allow(clippy::significant_drop_tightening)]

use crate::api::AppState;
use crate::dto::{
    CreateGameRequest, CreateGameResponse, GameResponse, GameStatusResponse, JoinGameRequest,
    JoinGameResponse, LeaveGameResponse, PlayerRemovedResponse, PlayersListResponse,
    StartGameRequest,
};
use crate::entities::users;
use crate::errors::AppError;
use crate::services::{GameService, PlayerService};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use sea_orm::{ActiveModelTrait, Set};
use std::sync::Arc;

/// Register all game-related routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Game CRUD
        .route("/api/games", post(create_game))
        .route("/api/games/{id}", get(get_game))
        // Join/Leave
        .route("/api/games/{code}/join", post(join_game))
        .route("/api/games/{id}/leave", post(leave_game))
        // Players
        .route("/api/games/{id}/players", get(get_players))
        .route("/api/games/{id}/players/{player_id}", delete(kick_player))
        // Game lifecycle
        .route("/api/games/{id}/start", post(start_game))
}

/// POST /api/games - Create a new game
async fn create_game(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateGameRequest>,
) -> Result<Json<CreateGameResponse>, AppError> {
    let db_lock = state.db.read().await;
    let db = db_lock
        .as_ref()
        .ok_or_else(|| AppError::Database("Database not connected".to_string()))?;

    // For v0.2.0, we create a temporary user
    // Full user authentication comes in v0.3.0
    let host_id = get_or_create_temp_user(db).await?;

    let response = GameService::create_game(db, host_id, request).await?;

    tracing::info!("Game created: {} (code: {})", response.id, response.code);

    Ok(Json(response))
}

/// GET /api/games/:id - Get game details
async fn get_game(
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<i32>,
) -> Result<Json<GameResponse>, AppError> {
    let db_lock = state.db.read().await;
    let db = db_lock
        .as_ref()
        .ok_or_else(|| AppError::Database("Database not connected".to_string()))?;

    let response = GameService::get_game_by_id(db, game_id).await?;

    Ok(Json(response))
}

/// POST /api/games/:code/join - Join a game by code
async fn join_game(
    State(state): State<Arc<AppState>>,
    Path(game_code): Path<String>,
    Json(request): Json<JoinGameRequest>,
) -> Result<Json<JoinGameResponse>, AppError> {
    let db_lock = state.db.read().await;
    let db = db_lock
        .as_ref()
        .ok_or_else(|| AppError::Database("Database not connected".to_string()))?;

    // For v0.2.0, create/get a temporary user
    let user_id = get_or_create_temp_user(db).await?;

    let response = PlayerService::join_game(db, &game_code, user_id, request).await?;

    tracing::info!(
        "Player {} joined game {} with nickname '{}'",
        response.player_id,
        response.game_id,
        response.nickname
    );

    Ok(Json(response))
}

/// GET /api/games/:id/players - Get all players in a game
async fn get_players(
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<i32>,
) -> Result<Json<PlayersListResponse>, AppError> {
    let db_lock = state.db.read().await;
    let db = db_lock
        .as_ref()
        .ok_or_else(|| AppError::Database("Database not connected".to_string()))?;

    let response = PlayerService::get_players(db, game_id).await?;

    Ok(Json(response))
}

/// POST /api/games/:id/start - Start the game (host only)
async fn start_game(
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<i32>,
    body: Option<Json<StartGameRequest>>,
) -> Result<Json<GameStatusResponse>, AppError> {
    let db_lock = state.db.read().await;
    let db = db_lock
        .as_ref()
        .ok_or_else(|| AppError::Database("Database not connected".to_string()))?;

    let request = body.map(|b| b.0).unwrap_or_default();

    // For v0.2.0, we get the host_id from the game itself
    // In v0.3.0 this will come from JWT authentication
    let game = GameService::get_game_by_id(db, game_id).await?;
    let host_id = game.host_id;

    let response = GameService::start_game(db, game_id, host_id, request).await?;

    tracing::info!("Game {} started", game_id);

    Ok(Json(response))
}

/// DELETE `/api/games/:id/players/:player_id` - Kick a player (host only)
async fn kick_player(
    State(state): State<Arc<AppState>>,
    Path((game_id, player_id)): Path<(i32, i32)>,
) -> Result<Json<PlayerRemovedResponse>, AppError> {
    let db_lock = state.db.read().await;
    let db = db_lock
        .as_ref()
        .ok_or_else(|| AppError::Database("Database not connected".to_string()))?;

    // For v0.2.0, we get the host_id from the game itself
    // In v0.3.0 this will come from JWT authentication
    let game = GameService::get_game_by_id(db, game_id).await?;
    let host_id = game.host_id;

    let response = PlayerService::kick_player(db, game_id, player_id, host_id).await?;

    tracing::info!("Player {} kicked from game {}", player_id, game_id);

    Ok(Json(response))
}

/// POST /api/games/:id/leave - Leave a game
async fn leave_game(
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<i32>,
) -> Result<Json<LeaveGameResponse>, AppError> {
    let db_lock = state.db.read().await;
    let db = db_lock
        .as_ref()
        .ok_or_else(|| AppError::Database("Database not connected".to_string()))?;

    // For v0.2.0, get a temp user - in real usage this would come from auth
    // Note: This is a limitation for v0.2.0 - leave won't work properly without auth
    // This will be fixed in v0.3.0 when JWT authentication is implemented
    let user_id = get_or_create_temp_user(db).await?;

    let response = PlayerService::leave_game(db, game_id, user_id).await?;

    tracing::info!("User {} left game {}", user_id, game_id);

    Ok(Json(response))
}

// ============ Temporary User Handling for v0.2.0 ============
// These functions will be replaced with proper auth in v0.3.0

/// Create a temporary user for testing (v0.2.0 only)
/// In v0.3.0, this will be replaced with JWT authentication
async fn get_or_create_temp_user(db: &sea_orm::DatabaseConnection) -> Result<i32, AppError> {
    // Generate a random username for temp user
    let temp_username = format!("temp_user_{}", chrono::Utc::now().timestamp_millis());

    let now = chrono::Utc::now().naive_utc();
    let user = users::ActiveModel {
        username: Set(temp_username),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let result = user.insert(db).await?;

    Ok(result.id)
}
