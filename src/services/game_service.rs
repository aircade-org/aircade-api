use crate::dto::{
    CreateGameRequest, CreateGameResponse, GameResponse, GameSettings, GameStatusResponse,
    StartGameRequest,
};
use crate::entities::{games, players, GameStatus};
use crate::errors::AppError;
use crate::utils::{generate_game_code, is_valid_game_code, normalize_game_code};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};

/// Maximum attempts to generate a unique game code
const MAX_CODE_GENERATION_ATTEMPTS: u32 = 10;

pub struct GameService;

impl GameService {
    /// Create a new game session
    pub async fn create_game(
        db: &DatabaseConnection,
        host_id: i32,
        request: CreateGameRequest,
    ) -> Result<CreateGameResponse, AppError> {
        // Generate unique game code with retry logic
        let code = Self::generate_unique_code(db).await?;

        // Build game settings
        let settings = GameSettings::new(request.max_players, request.game_type);
        let settings_json = serde_json::to_value(&settings)
            .map_err(|e| AppError::Internal(format!("Failed to serialize settings: {e}")))?;

        // Create game record
        let game = games::ActiveModel {
            code: Set(code.clone()),
            host_id: Set(host_id),
            status: Set(GameStatus::Lobby.as_str().to_string()),
            settings: Set(Some(settings_json)),
            created_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        };

        let result = game.insert(db).await?;

        Ok(CreateGameResponse {
            id: result.id,
            code: result.code,
            status: result.status,
            host_id: result.host_id,
            created_at: result.created_at,
        })
    }

    /// Generate a unique game code, with retry logic
    async fn generate_unique_code(db: &DatabaseConnection) -> Result<String, AppError> {
        for _ in 0..MAX_CODE_GENERATION_ATTEMPTS {
            let code = generate_game_code();

            // Check if code already exists
            let existing = games::Entity::find()
                .filter(games::Column::Code.eq(&code))
                .one(db)
                .await?;

            if existing.is_none() {
                return Ok(code);
            }
        }

        Err(AppError::Internal(
            "Failed to generate unique game code after multiple attempts".to_string(),
        ))
    }

    /// Get game by ID
    pub async fn get_game_by_id(
        db: &DatabaseConnection,
        game_id: i32,
    ) -> Result<GameResponse, AppError> {
        let game = games::Entity::find_by_id(game_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::GameNotFound(game_id.to_string()))?;

        #[allow(clippy::cast_possible_truncation)]
        let player_count = players::Entity::find()
            .filter(players::Column::GameId.eq(game_id))
            .count(db)
            .await? as usize;

        let settings: GameSettings = game
            .settings
            .as_ref()
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or_default();

        Ok(GameResponse {
            id: game.id,
            code: game.code,
            status: game.status,
            host_id: game.host_id,
            settings: game.settings,
            created_at: game.created_at,
            player_count,
            max_players: settings.max_players,
        })
    }

    /// Get game by code
    pub async fn get_game_by_code(
        db: &DatabaseConnection,
        code: &str,
    ) -> Result<games::Model, AppError> {
        let normalized_code = normalize_game_code(code);

        if !is_valid_game_code(&normalized_code) {
            return Err(AppError::InvalidGameCode(code.to_string()));
        }

        games::Entity::find()
            .filter(games::Column::Code.eq(&normalized_code))
            .one(db)
            .await?
            .ok_or_else(|| AppError::GameNotFound(code.to_string()))
    }

    /// Start a game (host only)
    pub async fn start_game(
        db: &DatabaseConnection,
        game_id: i32,
        host_id: i32,
        _request: StartGameRequest,
    ) -> Result<GameStatusResponse, AppError> {
        let game = games::Entity::find_by_id(game_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::GameNotFound(game_id.to_string()))?;

        // Verify host
        if game.host_id != host_id {
            return Err(AppError::NotHost {
                action: "start the game".to_string(),
            });
        }

        // Verify game status
        let current_status = GameStatus::from_str(&game.status).unwrap_or(GameStatus::Lobby);

        if !current_status.can_start() {
            return Err(AppError::InvalidGameState {
                current: game.status.clone(),
                required: "lobby".to_string(),
            });
        }

        // Check minimum players (at least 2)
        let player_count = players::Entity::find()
            .filter(players::Column::GameId.eq(game_id))
            .count(db)
            .await?;

        if player_count < 2 {
            return Err(AppError::Validation(
                "At least 2 players required to start the game".to_string(),
            ));
        }

        // Update game status
        let mut game_active: games::ActiveModel = game.into();
        game_active.status = Set(GameStatus::Playing.as_str().to_string());
        game_active.update(db).await?;

        Ok(GameStatusResponse {
            id: game_id,
            status: GameStatus::Playing.as_str().to_string(),
            message: "Game started successfully".to_string(),
        })
    }

    /// End a game (set to finished)
    #[allow(dead_code)]
    pub async fn finish_game(
        db: &DatabaseConnection,
        game_id: i32,
    ) -> Result<GameStatusResponse, AppError> {
        let game = games::Entity::find_by_id(game_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::GameNotFound(game_id.to_string()))?;

        let mut game_active: games::ActiveModel = game.into();
        game_active.status = Set(GameStatus::Finished.as_str().to_string());
        game_active.update(db).await?;

        Ok(GameStatusResponse {
            id: game_id,
            status: GameStatus::Finished.as_str().to_string(),
            message: "Game finished".to_string(),
        })
    }
}
