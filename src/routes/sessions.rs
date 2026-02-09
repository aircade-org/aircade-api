use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::entities::{game, game_version, player, session};
use crate::error::AppError;
use crate::sessions::ClientRole;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

/// Build the session route group: `/sessions/...`
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_session))
        .route("/{session_code}", get(get_session))
        .route("/{session_code}/join", post(join_session))
        .route("/{session_id}/players", get(list_players))
        .route("/{session_id}/end", post(end_session))
        .route("/{session_id}/game", post(load_game))
        .route("/{session_id}/ws", get(ws_upgrade))
}

// ─────────────────────────────────────────────────────────────────────────────
// DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionRequest {
    max_players: Option<i32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    id: Uuid,
    created_at: String,
    updated_at: String,
    ended_at: Option<String>,
    host_id: Uuid,
    game_id: Option<Uuid>,
    game_version_id: Option<Uuid>,
    session_code: String,
    status: String,
    max_players: i32,
    players: Vec<PlayerResponse>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PlayerResponse {
    id: Uuid,
    created_at: String,
    display_name: String,
    avatar_url: Option<String>,
    connection_status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinSessionRequest {
    display_name: String,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JoinResponse {
    player: PlayerResponse,
    session: SessionSummary,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionSummary {
    id: Uuid,
    session_code: String,
    status: String,
    host_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadGameRequest {
    game_id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadGameResponse {
    session_id: Uuid,
    game_id: Uuid,
    game_version_id: Uuid,
    status: String,
}

#[derive(Deserialize)]
struct WsQueryParams {
    role: String,
    #[serde(rename = "playerId")]
    player_id: Option<Uuid>,
    token: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Session code generation
// ─────────────────────────────────────────────────────────────────────────────

/// Characters used for session codes — excludes ambiguous chars (0/O, 1/I/L).
const SESSION_CODE_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const SESSION_CODE_LENGTH: usize = 5;

/// Generate a random session code string (not yet validated for uniqueness).
fn random_session_code() -> String {
    let mut rng = rand::thread_rng();
    (0..SESSION_CODE_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..SESSION_CODE_CHARS.len());
            char::from(SESSION_CODE_CHARS[idx])
        })
        .collect()
}

/// Generate a unique session code, retrying if collisions occur.
///
/// # Errors
///
/// Returns an error if a unique code cannot be generated after 20 attempts.
async fn generate_session_code(db: &sea_orm::DatabaseConnection) -> Result<String, AppError> {
    for _ in 0..20 {
        let code = random_session_code();

        // Check uniqueness among active (non-ended) sessions
        let existing = session::Entity::find()
            .filter(session::Column::SessionCode.eq(&code))
            .filter(session::Column::Status.ne("ended"))
            .one(db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        if existing.is_none() {
            return Ok(code);
        }
    }

    Err(AppError::Internal(anyhow::anyhow!(
        "Failed to generate unique session code after 20 attempts"
    )))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build a `SessionResponse` from a session model and its players.
fn build_session_response(sess: &session::Model, players: Vec<player::Model>) -> SessionResponse {
    SessionResponse {
        id: sess.id,
        created_at: sess.created_at.to_rfc3339(),
        updated_at: sess.updated_at.to_rfc3339(),
        ended_at: sess.ended_at.map(|t| t.to_rfc3339()),
        host_id: sess.host_id,
        game_id: sess.game_id,
        game_version_id: sess.game_version_id,
        session_code: sess.session_code.clone(),
        status: sess.status.clone(),
        max_players: sess.max_players,
        players: players.into_iter().map(build_player_response).collect(),
    }
}

/// Build a `PlayerResponse` from a player model.
fn build_player_response(p: player::Model) -> PlayerResponse {
    PlayerResponse {
        id: p.id,
        created_at: p.created_at.to_rfc3339(),
        display_name: p.display_name,
        avatar_url: p.avatar_url,
        connection_status: p.connection_status,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/sessions` — Create a new session in lobby status.
async fn create_session(
    State(state): State<AppState>,
    AuthUser(host): AuthUser,
    Json(body): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionResponse>), AppError> {
    let session_code = generate_session_code(&state.db).await?;
    let now = Utc::now().fixed_offset();
    let max_players = body.max_players.unwrap_or(8).clamp(1, 32);

    let sess = session::ActiveModel {
        id: Set(Uuid::new_v4()),
        created_at: Set(now),
        updated_at: Set(now),
        ended_at: Set(None),
        host_id: Set(host.id),
        game_id: Set(None),
        game_version_id: Set(None),
        session_code: Set(session_code),
        status: Set("lobby".to_string()),
        max_players: Set(max_players),
    };

    let inserted = sess
        .insert(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let response = build_session_response(&inserted, vec![]);
    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /api/v1/sessions/{sessionCode}` — Get session details by code.
async fn get_session(
    State(state): State<AppState>,
    Path(session_code): Path<String>,
) -> Result<Json<SessionResponse>, AppError> {
    let code_upper = session_code.to_uppercase();

    let sess = session::Entity::find()
        .filter(session::Column::SessionCode.eq(&code_upper))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found.".to_string()))?;

    let players = player::Entity::find()
        .filter(player::Column::SessionId.eq(sess.id))
        .filter(player::Column::LeftAt.is_null())
        .all(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(build_session_response(&sess, players)))
}

/// `POST /api/v1/sessions/{sessionCode}/join` — Join a session by code.
async fn join_session(
    State(state): State<AppState>,
    Path(session_code): Path<String>,
    Json(body): Json<JoinSessionRequest>,
) -> Result<(StatusCode, Json<JoinResponse>), AppError> {
    let code_upper = session_code.to_uppercase();

    let sess = session::Entity::find()
        .filter(session::Column::SessionCode.eq(&code_upper))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found.".to_string()))?;

    // Validate session is joinable
    if sess.status == "ended" {
        return Err(AppError::BadRequest("Session has ended.".to_string()));
    }

    // Count active players
    let active_players = player::Entity::find()
        .filter(player::Column::SessionId.eq(sess.id))
        .filter(player::Column::LeftAt.is_null())
        .all(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let max = usize::try_from(sess.max_players).unwrap_or(8);
    if active_players.len() >= max {
        return Err(AppError::BadRequest("Session is full.".to_string()));
    }

    // Validate display name
    let display_name = body.display_name.trim().to_string();
    if display_name.is_empty() || display_name.len() > 100 {
        return Err(AppError::BadRequest(
            "Display name must be between 1 and 100 characters.".to_string(),
        ));
    }

    let now = Utc::now().fixed_offset();
    let player_model = player::ActiveModel {
        id: Set(Uuid::new_v4()),
        created_at: Set(now),
        session_id: Set(sess.id),
        user_id: Set(None), // Anonymous guest
        display_name: Set(display_name),
        avatar_url: Set(body.avatar_url),
        connection_status: Set("connected".to_string()),
        left_at: Set(None),
    };

    let inserted_player = player_model
        .insert(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Broadcast player_joined to all connected clients
    let joined_msg = serde_json::json!({
        "type": "player_joined",
        "payload": {
            "player": {
                "id": inserted_player.id,
                "displayName": inserted_player.display_name,
                "avatarUrl": inserted_player.avatar_url,
            }
        }
    });
    state
        .session_manager
        .broadcast(sess.id, &joined_msg.to_string());

    let player_resp = build_player_response(inserted_player);

    Ok((
        StatusCode::CREATED,
        Json(JoinResponse {
            player: player_resp,
            session: SessionSummary {
                id: sess.id,
                session_code: sess.session_code,
                status: sess.status,
                host_id: sess.host_id,
            },
        }),
    ))
}

/// `GET /api/v1/sessions/{sessionId}/players` — List all players in a session.
async fn list_players(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<PlayerResponse>>, AppError> {
    // Verify session exists
    session::Entity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found.".to_string()))?;

    let players = player::Entity::find()
        .filter(player::Column::SessionId.eq(session_id))
        .all(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(
        players.into_iter().map(build_player_response).collect(),
    ))
}

/// `POST /api/v1/sessions/{sessionId}/end` — End a session (host only).
async fn end_session(
    State(state): State<AppState>,
    AuthUser(host): AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let sess = session::Entity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found.".to_string()))?;

    if sess.host_id != host.id {
        return Err(AppError::Forbidden(
            "Only the session host can end the session.".to_string(),
        ));
    }

    if sess.status == "ended" {
        return Err(AppError::BadRequest(
            "Session is already ended.".to_string(),
        ));
    }

    let now = Utc::now().fixed_offset();
    let mut active: session::ActiveModel = sess.into();
    active.status = Set("ended".to_string());
    active.ended_at = Set(Some(now));
    active.updated_at = Set(now);
    active
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Broadcast session_status_change and close all connections
    let status_msg = serde_json::json!({
        "type": "session_status_change",
        "payload": {
            "status": "ended",
            "previousStatus": "lobby"
        }
    });
    state
        .session_manager
        .broadcast(session_id, &status_msg.to_string());
    state.session_manager.remove_session(session_id);

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/sessions/{sessionId}/game` — Load a game into the session.
async fn load_game(
    State(state): State<AppState>,
    AuthUser(host): AuthUser,
    Path(session_id): Path<Uuid>,
    Json(body): Json<LoadGameRequest>,
) -> Result<Json<LoadGameResponse>, AppError> {
    let sess = session::Entity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found.".to_string()))?;

    if sess.host_id != host.id {
        return Err(AppError::Forbidden(
            "Only the session host can load a game.".to_string(),
        ));
    }

    if sess.status == "ended" {
        return Err(AppError::BadRequest("Session has ended.".to_string()));
    }

    // Validate game exists and is published
    let found_game = game::Entity::find_by_id(body.game_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("Game not found.".to_string()))?;

    if found_game.status != "published" {
        return Err(AppError::BadRequest("Game is not published.".to_string()));
    }

    // Get the published version
    let version = if let Some(ver_id) = found_game.published_version_id {
        game_version::Entity::find_by_id(ver_id)
            .one(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?
    } else {
        // Fall back to latest version
        game_version::Entity::find()
            .filter(game_version::Column::GameId.eq(found_game.id))
            .one(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?
    };

    let version =
        version.ok_or_else(|| AppError::NotFound("No game version found.".to_string()))?;

    let previous_status = sess.status.clone();

    // Update session with game info and transition to playing
    let now = Utc::now().fixed_offset();
    let mut active: session::ActiveModel = sess.into();
    active.game_id = Set(Some(found_game.id));
    active.game_version_id = Set(Some(version.id));
    active.status = Set("playing".to_string());
    active.updated_at = Set(now);
    active
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Send game_loaded to host with gameScreenCode
    let host_msg = serde_json::json!({
        "type": "game_loaded",
        "payload": {
            "gameId": found_game.id,
            "gameVersionId": version.id,
            "gameScreenCode": version.game_screen_code,
        }
    });
    state
        .session_manager
        .send_to_host(session_id, &host_msg.to_string());

    // Send game_loaded to all players with controllerScreenCode
    let player_msg = serde_json::json!({
        "type": "game_loaded",
        "payload": {
            "gameId": found_game.id,
            "gameVersionId": version.id,
            "controllerScreenCode": version.controller_screen_code,
        }
    });
    state
        .session_manager
        .broadcast_to_players(session_id, &player_msg.to_string());

    // Broadcast status change
    let status_msg = serde_json::json!({
        "type": "session_status_change",
        "payload": {
            "status": "playing",
            "previousStatus": previous_status,
        }
    });
    state
        .session_manager
        .broadcast(session_id, &status_msg.to_string());

    Ok(Json(LoadGameResponse {
        session_id,
        game_id: found_game.id,
        game_version_id: version.id,
        status: "playing".to_string(),
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/sessions/{sessionId}/ws` — Upgrade to `WebSocket`.
async fn ws_upgrade(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<WsQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    // Validate session exists and is not ended
    let sess = session::Entity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found.".to_string()))?;

    if sess.status == "ended" {
        return Err(AppError::BadRequest("Session has ended.".to_string()));
    }

    let role = match params.role.as_str() {
        "host" => {
            // Validate host identity via token
            if let Some(token) = &params.token {
                let claims =
                    crate::auth::jwt::validate_access_token(token, &state.config.jwt_secret)
                        .map_err(|_| {
                            AppError::Unauthorized("Invalid or expired token.".to_string())
                        })?;
                let user_id: Uuid = claims
                    .sub
                    .parse()
                    .map_err(|_| AppError::Unauthorized("Invalid token subject.".to_string()))?;
                if user_id != sess.host_id {
                    return Err(AppError::Forbidden(
                        "Only the session host can connect as host.".to_string(),
                    ));
                }
            } else {
                return Err(AppError::Unauthorized(
                    "Token required for host connection.".to_string(),
                ));
            }
            ClientRole::Host
        }
        "player" => {
            let player_id = params.player_id.ok_or_else(|| {
                AppError::BadRequest("playerId is required for player connections.".to_string())
            })?;

            // Validate player exists in this session
            let found_player = player::Entity::find_by_id(player_id)
                .one(&state.db)
                .await
                .map_err(|e| AppError::Internal(e.into()))?
                .ok_or_else(|| AppError::NotFound("Player not found.".to_string()))?;

            if found_player.session_id != session_id {
                return Err(AppError::BadRequest(
                    "Player does not belong to this session.".to_string(),
                ));
            }

            // Update connection status
            let mut active_player: player::ActiveModel = found_player.into();
            active_player.connection_status = Set("connected".to_string());
            active_player.left_at = Set(None);
            active_player
                .update(&state.db)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;

            ClientRole::Player(player_id)
        }
        _ => {
            return Err(AppError::BadRequest(
                "Invalid role. Must be 'host' or 'player'.".to_string(),
            ));
        }
    };

    let ws_state = state.clone();

    Ok(ws.on_upgrade(move |socket| handle_ws_connection(ws_state, session_id, role, socket)))
}

/// Handle a single `WebSocket` connection for message relay.
async fn handle_ws_connection(
    state: AppState,
    session_id: Uuid,
    role: ClientRole,
    socket: WebSocket,
) {
    let (mut ws_sink, mut ws_stream) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Register this connection
    state.session_manager.register(session_id, role.clone(), tx);

    // Send connected message
    let connected_msg = match &role {
        ClientRole::Host => serde_json::json!({
            "type": "connected",
            "payload": {
                "sessionId": session_id,
                "role": "host",
            }
        }),
        ClientRole::Player(pid) => serde_json::json!({
            "type": "connected",
            "payload": {
                "sessionId": session_id,
                "role": "player",
                "playerId": pid,
            }
        }),
    };
    let _ = ws_sink
        .send(Message::Text(connected_msg.to_string().into()))
        .await;

    // Spawn task to forward outbound messages to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Process inbound messages
    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => {
                handle_ws_message(&state, session_id, &role, &text);
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup on disconnect
    send_task.abort();
    state.session_manager.unregister(session_id, &role);

    // Update player connection status in database
    if let ClientRole::Player(player_id) = &role {
        if let Ok(Some(p)) = player::Entity::find_by_id(*player_id).one(&state.db).await {
            let now = Utc::now().fixed_offset();
            let mut active_player: player::ActiveModel = p.into();
            active_player.connection_status = Set("disconnected".to_string());
            active_player.left_at = Set(Some(now));
            let _ = active_player.update(&state.db).await;
        }

        // Broadcast player_left
        let left_msg = serde_json::json!({
            "type": "player_left",
            "payload": {
                "playerId": player_id,
                "reason": "disconnected"
            }
        });
        state
            .session_manager
            .broadcast(session_id, &left_msg.to_string());
    }
}

/// Route an inbound `WebSocket` message based on its type.
fn handle_ws_message(state: &AppState, session_id: Uuid, role: &ClientRole, text: &str) {
    let parsed: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    let msg_type = parsed
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    match (msg_type, role) {
        // Player sends input → relay to host with playerId attached
        ("player_input", ClientRole::Player(player_id)) => {
            let relay_msg = serde_json::json!({
                "type": "player_input_event",
                "payload": {
                    "playerId": player_id,
                    "inputType": parsed["payload"]["inputType"],
                    "data": parsed["payload"]["data"],
                }
            });
            state
                .session_manager
                .send_to_host(session_id, &relay_msg.to_string());
        }
        // Host broadcasts game state → relay to all players
        ("game_state_update", ClientRole::Host) => {
            let relay_msg = serde_json::json!({
                "type": "game_state",
                "payload": parsed["payload"],
            });
            state
                .session_manager
                .broadcast_to_players(session_id, &relay_msg.to_string());
        }
        _ => {
            // Unknown message types are silently ignored
        }
    }
}
