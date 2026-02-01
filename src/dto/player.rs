use serde::Serialize;

// ============ Response DTOs ============

/// Player join response
#[derive(Debug, Serialize)]
pub struct JoinGameResponse {
    pub player_id: i32,
    pub game_id: i32,
    pub nickname: String,
    pub color: String,
    pub player_token: String,
    pub joined_at: chrono::NaiveDateTime,
}

/// Player details response
#[derive(Debug, Serialize)]
pub struct PlayerResponse {
    pub id: i32,
    pub game_id: i32,
    pub user_id: i32,
    pub nickname: String,
    pub color: String,
    pub joined_at: chrono::NaiveDateTime,
    pub is_host: bool,
}

/// List of players in a game
#[derive(Debug, Serialize)]
pub struct PlayersListResponse {
    pub players: Vec<PlayerResponse>,
    pub count: usize,
}

/// Player removal response
#[derive(Debug, Serialize)]
pub struct PlayerRemovedResponse {
    pub player_id: i32,
    pub message: String,
}

/// Leave game response
#[derive(Debug, Serialize)]
pub struct LeaveGameResponse {
    pub message: String,
}
