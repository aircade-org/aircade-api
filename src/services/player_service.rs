use crate::dto::{
    GameSettings, JoinGameRequest, JoinGameResponse, LeaveGameResponse, PlayerRemovedResponse,
    PlayerResponse, PlayersListResponse,
};
use crate::entities::{games, players, GameStatus};
use crate::errors::AppError;
use crate::services::GameService;
use crate::utils::get_next_available_color;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub struct PlayerService;

impl PlayerService {
    /// Join a game by code
    pub async fn join_game(
        db: &DatabaseConnection,
        game_code: &str,
        user_id: i32,
        request: JoinGameRequest,
    ) -> Result<JoinGameResponse, AppError> {
        // Validate nickname
        Self::validate_nickname(&request.nickname)?;

        // Get game by code
        let game = GameService::get_game_by_code(db, game_code).await?;

        // Check game status allows joining
        let status = GameStatus::from_str(&game.status).unwrap_or(GameStatus::Finished);
        if !status.can_join() {
            return Err(AppError::InvalidGameState {
                current: game.status.clone(),
                required: "lobby".to_string(),
            });
        }

        // Check if user already in game
        let existing_player = players::Entity::find()
            .filter(players::Column::GameId.eq(game.id))
            .filter(players::Column::UserId.eq(user_id))
            .one(db)
            .await?;

        if existing_player.is_some() {
            return Err(AppError::AlreadyInGame {
                user_id,
                game_code: game.code.clone(),
            });
        }

        // Check nickname uniqueness in game
        let nickname_exists = players::Entity::find()
            .filter(players::Column::GameId.eq(game.id))
            .filter(players::Column::Nickname.eq(&request.nickname))
            .one(db)
            .await?;

        if nickname_exists.is_some() {
            return Err(AppError::NicknameTaken {
                nickname: request.nickname.clone(),
                game_code: game.code.clone(),
            });
        }

        // Check max players
        let current_players = players::Entity::find()
            .filter(players::Column::GameId.eq(game.id))
            .all(db)
            .await?;

        let settings: GameSettings = game
            .settings
            .as_ref()
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or_default();

        if current_players.len() >= settings.max_players as usize {
            return Err(AppError::GameFull {
                game_code: game.code.clone(),
                max_players: settings.max_players,
            });
        }

        // Assign color
        let taken_colors: Vec<String> = current_players.iter().map(|p| p.color.clone()).collect();
        let color = get_next_available_color(&taken_colors).to_string();

        // Generate simple player token (for v0.2.0, full auth comes in v0.3.0)
        let player_token = Self::generate_player_token(game.id, user_id);

        // Create player record
        let now = chrono::Utc::now().naive_utc();
        let player = players::ActiveModel {
            game_id: Set(game.id),
            user_id: Set(user_id),
            nickname: Set(request.nickname.clone()),
            color: Set(color.clone()),
            joined_at: Set(now),
            ..Default::default()
        };

        let result = player.insert(db).await?;

        Ok(JoinGameResponse {
            player_id: result.id,
            game_id: result.game_id,
            nickname: result.nickname,
            color: result.color,
            player_token,
            joined_at: result.joined_at,
        })
    }

    /// Validate nickname format
    fn validate_nickname(nickname: &str) -> Result<(), AppError> {
        let trimmed = nickname.trim();

        if trimmed.len() < 2 {
            return Err(AppError::InvalidNickname(
                "Nickname must be at least 2 characters".to_string(),
            ));
        }

        if trimmed.len() > 20 {
            return Err(AppError::InvalidNickname(
                "Nickname must be at most 20 characters".to_string(),
            ));
        }

        // Only allow alphanumeric, spaces, and basic punctuation
        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '_' || c == '-')
        {
            return Err(AppError::InvalidNickname(
                "Nickname can only contain letters, numbers, spaces, underscores, and hyphens"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Generate a simple player token (placeholder for v0.2.0)
    /// Full JWT implementation comes in v0.3.0
    fn generate_player_token(game_id: i32, user_id: i32) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let timestamp = chrono::Utc::now().timestamp();
        let mut hasher = DefaultHasher::new();
        (game_id, user_id, timestamp).hash(&mut hasher);
        let hash = hasher.finish();

        format!("pt_{game_id}_{user_id}_{hash:x}")
    }

    /// Get all players in a game
    pub async fn get_players(
        db: &DatabaseConnection,
        game_id: i32,
    ) -> Result<PlayersListResponse, AppError> {
        // Verify game exists
        let game = games::Entity::find_by_id(game_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::GameNotFound(game_id.to_string()))?;

        let players_list = players::Entity::find()
            .filter(players::Column::GameId.eq(game_id))
            .order_by_asc(players::Column::JoinedAt)
            .all(db)
            .await?;

        let player_responses: Vec<PlayerResponse> = players_list
            .into_iter()
            .map(|p| PlayerResponse {
                id: p.id,
                game_id: p.game_id,
                user_id: p.user_id,
                nickname: p.nickname,
                color: p.color,
                joined_at: p.joined_at,
                is_host: p.user_id == game.host_id,
            })
            .collect();

        let count = player_responses.len();

        Ok(PlayersListResponse {
            players: player_responses,
            count,
        })
    }

    /// Remove a player from game (kick - host only)
    pub async fn kick_player(
        db: &DatabaseConnection,
        game_id: i32,
        player_id: i32,
        host_id: i32,
    ) -> Result<PlayerRemovedResponse, AppError> {
        // Verify game exists and caller is host
        let game = games::Entity::find_by_id(game_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::GameNotFound(game_id.to_string()))?;

        if game.host_id != host_id {
            return Err(AppError::NotHost {
                action: "kick players".to_string(),
            });
        }

        // Find the player
        let player = players::Entity::find_by_id(player_id)
            .one(db)
            .await?
            .ok_or(AppError::PlayerNotFound { player_id, game_id })?;

        // Verify player belongs to this game
        if player.game_id != game_id {
            return Err(AppError::PlayerNotFound { player_id, game_id });
        }

        // Cannot kick the host
        if player.user_id == game.host_id {
            return Err(AppError::Validation(
                "Cannot kick the host from the game".to_string(),
            ));
        }

        let nickname = player.nickname.clone();

        // Delete the player
        players::Entity::delete_by_id(player_id).exec(db).await?;

        Ok(PlayerRemovedResponse {
            player_id,
            message: format!("Player '{nickname}' has been removed from the game"),
        })
    }

    /// Leave game (player voluntarily leaves)
    pub async fn leave_game(
        db: &DatabaseConnection,
        game_id: i32,
        user_id: i32,
    ) -> Result<LeaveGameResponse, AppError> {
        // Find the player
        let player = players::Entity::find()
            .filter(players::Column::GameId.eq(game_id))
            .filter(players::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or(AppError::PlayerNotFound {
                player_id: 0,
                game_id,
            })?;

        // Check if player is host
        let game = games::Entity::find_by_id(game_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::GameNotFound(game_id.to_string()))?;

        if game.host_id == user_id {
            // Host leaving - end the game for v0.2.0
            // (transfer ownership could be implemented in future version)
            let mut game_active: games::ActiveModel = game.into();
            game_active.status = Set(GameStatus::Finished.as_str().to_string());
            game_active.update(db).await?;

            return Ok(LeaveGameResponse {
                message: "Host left - game has been ended".to_string(),
            });
        }

        // Delete the player record
        players::Entity::delete_by_id(player.id).exec(db).await?;

        Ok(LeaveGameResponse {
            message: "You have left the game".to_string(),
        })
    }
}
