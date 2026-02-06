use axum::{
    extract::{Multipart, Path, Query, State}, http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json,
    Router,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    entities::{game, game_asset, game_tag, game_version, tag, user},
    error::AppError,
    state::AppState,
};

/// Wraps an optional authenticated user (bearer token is optional for some routes).
struct OptionalAuth(Option<user::Model>);

impl axum::extract::FromRequestParts<AppState> for OptionalAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(AuthUser(u)) => Ok(Self(Some(u))),
            Err(_) => Ok(Self(None)),
        }
    }
}

/// Game management router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_game))
        .route(
            "/{id}",
            get(get_game).patch(update_game).delete(delete_game),
        )
        .route("/{id}/publish", post(publish_game))
        .route("/{id}/archive", post(archive_game))
        .route("/{id}/unarchive", post(unarchive_game))
        .route("/{id}/fork", post(fork_game))
        .route("/{id}/versions", get(list_versions))
        .route("/{id}/versions/{version_number}", get(get_version))
        .route("/{id}/assets", post(upload_asset).get(list_assets))
        .route(
            "/{id}/assets/{asset_id}",
            get(get_asset).delete(delete_asset),
        )
        .route("/{id}/tags", put(set_game_tags).get(get_game_tags))
}

/// Tags router.
pub fn tags_router() -> Router<AppState> {
    Router::new().route("/", get(list_tags))
}

// ============================================================================
// Request / Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateGameRequest {
    title: String,
    description: Option<String>,
    technology: Option<String>,
    min_players: Option<i32>,
    max_players: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateGameRequest {
    title: Option<String>,
    description: Option<String>,
    thumbnail_url: Option<String>,
    min_players: Option<i32>,
    max_players: Option<i32>,
    visibility: Option<String>,
    game_screen_code: Option<String>,
    controller_screen_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishGameRequest {
    changelog: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetTagsRequest {
    tag_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_offset")]
    offset: u64,
    #[serde(default = "default_limit")]
    limit: u64,
}

const fn default_offset() -> u64 {
    0
}

const fn default_limit() -> u64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct MyGamesQuery {
    #[serde(default = "default_offset")]
    offset: u64,
    #[serde(default = "default_limit")]
    limit: u64,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TagCategoryQuery {
    category: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameResponse {
    id: Uuid,
    created_at: String,
    updated_at: String,
    creator_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator: Option<CreatorInfo>,
    title: String,
    description: Option<String>,
    thumbnail_url: Option<String>,
    technology: String,
    min_players: i32,
    max_players: i32,
    status: String,
    visibility: String,
    forked_from_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    game_screen_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    controller_screen_code: Option<String>,
    published_version_id: Option<Uuid>,
    play_count: i64,
    total_play_time: i64,
    avg_rating: f32,
    review_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<TagResponse>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameSummaryResponse {
    id: Uuid,
    created_at: String,
    updated_at: String,
    creator_id: Uuid,
    title: String,
    description: Option<String>,
    thumbnail_url: Option<String>,
    technology: String,
    min_players: i32,
    max_players: i32,
    status: String,
    visibility: String,
    published_version_id: Option<Uuid>,
    play_count: i64,
    avg_rating: f32,
    review_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatorInfo {
    username: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TagResponse {
    id: Uuid,
    name: String,
    slug: String,
    category: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionSummaryResponse {
    id: Uuid,
    created_at: String,
    version_number: i32,
    changelog: Option<String>,
    published_by_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionDetailResponse {
    id: Uuid,
    created_at: String,
    game_id: Uuid,
    version_number: i32,
    game_screen_code: Option<String>,
    controller_screen_code: Option<String>,
    changelog: Option<String>,
    published_by_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetResponse {
    id: Uuid,
    created_at: String,
    game_id: Uuid,
    file_name: String,
    file_type: String,
    file_size: i32,
    storage_url: String,
}

#[derive(Debug, Serialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    total: u64,
    offset: u64,
    limit: u64,
}

// ============================================================================
// Handlers
// ============================================================================

/// `POST /games` — Create a new game.
async fn create_game(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateGameRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title is required".to_string()));
    }

    let min = req.min_players.unwrap_or(1);
    let max = req.max_players.unwrap_or(4);
    if max < min {
        return Err(AppError::BadRequest(
            "maxPlayers must be >= minPlayers".to_string(),
        ));
    }

    let now = chrono::Utc::now();
    let id = Uuid::new_v4();
    let slug = unique_slug(&req.title, id);

    let game = game::ActiveModel {
        id: ActiveValue::Set(id),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
        owner_id: ActiveValue::Set(user.id),
        title: ActiveValue::Set(req.title),
        slug: ActiveValue::Set(slug),
        description: ActiveValue::Set(req.description),
        technology: ActiveValue::Set(req.technology.unwrap_or_else(|| "p5js".to_string())),
        min_players: ActiveValue::Set(min),
        max_players: ActiveValue::Set(max),
        status: ActiveValue::Set("draft".to_string()),
        visibility: ActiveValue::Set("private".to_string()),
        ..Default::default()
    };

    let game = game.insert(&state.db).await?;

    Ok((
        StatusCode::CREATED,
        Json(to_game_response(game, None, None, true)),
    ))
}

/// `GET /games/:id` — Get a game by ID.
async fn get_game(
    State(state): State<AppState>,
    OptionalAuth(opt_user): OptionalAuth,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    let user_id = opt_user.as_ref().map(|u| u.id);
    check_visibility(&game, user_id)?;

    let is_creator = user_id == Some(game.owner_id);

    let creator = load_creator(&state.db, game.owner_id).await?;
    let tags = load_game_tags(&state.db, game.id).await?;

    Ok(Json(to_game_response(
        game,
        Some(creator),
        Some(tags),
        is_creator,
    )))
}

/// `PATCH /games/:id` — Update game metadata or code.
async fn update_game(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateGameRequest>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    // Validate player counts if both provided
    let effective_min = req.min_players.unwrap_or(game.min_players);
    let effective_max = req.max_players.unwrap_or(game.max_players);
    if effective_max < effective_min {
        return Err(AppError::BadRequest(
            "maxPlayers must be >= minPlayers".to_string(),
        ));
    }

    let mut active: game::ActiveModel = game.into();
    active.updated_at = ActiveValue::Set(chrono::Utc::now().into());

    if let Some(title) = req.title {
        if title.trim().is_empty() {
            return Err(AppError::BadRequest("Title cannot be empty".to_string()));
        }
        let id_val = match &active.id {
            ActiveValue::Set(v) | ActiveValue::Unchanged(v) => *v,
            ActiveValue::NotSet => Uuid::new_v4(),
        };
        active.slug = ActiveValue::Set(unique_slug(&title, id_val));
        active.title = ActiveValue::Set(title);
    }
    if let Some(desc) = req.description {
        active.description = ActiveValue::Set(Some(desc));
    }
    if let Some(thumb) = req.thumbnail_url {
        active.thumbnail = ActiveValue::Set(Some(thumb));
    }
    if let Some(min) = req.min_players {
        active.min_players = ActiveValue::Set(min);
    }
    if let Some(max) = req.max_players {
        active.max_players = ActiveValue::Set(max);
    }
    if let Some(vis) = req.visibility {
        active.visibility = ActiveValue::Set(vis);
    }
    if let Some(code) = req.game_screen_code {
        active.game_screen_code = ActiveValue::Set(Some(code));
    }
    if let Some(code) = req.controller_screen_code {
        active.controller_screen_code = ActiveValue::Set(Some(code));
    }

    let game = active.update(&state.db).await?;
    Ok(Json(to_game_response(game, None, None, true)))
}

/// `DELETE /games/:id` — Soft-delete a game.
async fn delete_game(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id && user.role != "moderator" && user.role != "admin" {
        return Err(AppError::Forbidden(
            "You are not authorized to delete this game".to_string(),
        ));
    }

    let now = chrono::Utc::now();

    // Soft-delete the game
    let mut active: game::ActiveModel = game.into();
    active.deleted_at = ActiveValue::Set(Some(now.into()));
    active.update(&state.db).await?;

    // Soft-delete all associated assets
    let assets = game_asset::Entity::find()
        .filter(game_asset::Column::GameId.eq(id))
        .filter(game_asset::Column::DeletedAt.is_null())
        .all(&state.db)
        .await?;

    for asset in assets {
        let mut a: game_asset::ActiveModel = asset.into();
        a.deleted_at = ActiveValue::Set(Some(now.into()));
        a.update(&state.db).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /games/:id/publish` — Publish a game by creating an immutable version snapshot.
#[allow(clippy::items_after_statements)]
async fn publish_game(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PublishGameRequest>,
) -> Result<impl IntoResponse, AppError> {
    if !user.email_verified {
        return Err(AppError::Unprocessable(
            "EMAIL_NOT_VERIFIED".to_string(),
            "Email must be verified to publish games".to_string(),
        ));
    }

    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    if game.title.trim().is_empty() {
        return Err(AppError::Unprocessable(
            "INVALID_GAME".to_string(),
            "Game must have a title".to_string(),
        ));
    }

    let screen_empty = game
        .game_screen_code
        .as_deref()
        .is_none_or(|c| c.trim().is_empty());
    let ctrl_empty = game
        .controller_screen_code
        .as_deref()
        .is_none_or(|c| c.trim().is_empty());
    if screen_empty && ctrl_empty {
        return Err(AppError::Unprocessable(
            "INVALID_GAME".to_string(),
            "Game must have at least one non-empty canvas code".to_string(),
        ));
    }

    // Determine next version number
    let version_count = game_version::Entity::find()
        .filter(game_version::Column::GameId.eq(game.id))
        .count(&state.db)
        .await?;

    #[allow(clippy::cast_possible_truncation)]
    let version_number = (version_count + 1) as i32;

    let version = game_version::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        created_at: ActiveValue::Set(chrono::Utc::now().into()),
        game_id: ActiveValue::Set(game.id),
        version_number: ActiveValue::Set(version_number),
        game_screen_code: ActiveValue::Set(game.game_screen_code.clone()),
        controller_screen_code: ActiveValue::Set(game.controller_screen_code.clone()),
        changelog: ActiveValue::Set(req.changelog),
        published_by_id: ActiveValue::Set(Some(user.id)),
        change_log: ActiveValue::NotSet,
    };

    let version = version.insert(&state.db).await?;

    let mut active: game::ActiveModel = game.into();
    active.status = ActiveValue::Set("published".to_string());
    active.published_version_id = ActiveValue::Set(Some(version.id));
    active.updated_at = ActiveValue::Set(chrono::Utc::now().into());
    let game = active.update(&state.db).await?;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PublishResponse {
        version: VersionSummaryResponse,
        game: PublishGameInfo,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PublishGameInfo {
        id: Uuid,
        status: String,
        published_version_id: Option<Uuid>,
    }

    Ok((
        StatusCode::CREATED,
        Json(PublishResponse {
            version: to_version_summary(version),
            game: PublishGameInfo {
                id: game.id,
                status: game.status,
                published_version_id: game.published_version_id,
            },
        }),
    ))
}

/// `POST /games/:id/archive` — Archive a game.
async fn archive_game(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id && user.role != "moderator" && user.role != "admin" {
        return Err(AppError::Forbidden(
            "You are not authorized to archive this game".to_string(),
        ));
    }

    if game.status == "archived" {
        return Err(AppError::Unprocessable(
            "ALREADY_ARCHIVED".to_string(),
            "Game is already archived".to_string(),
        ));
    }

    let mut active: game::ActiveModel = game.into();
    active.status = ActiveValue::Set("archived".to_string());
    active.updated_at = ActiveValue::Set(chrono::Utc::now().into());
    let game = active.update(&state.db).await?;

    Ok(Json(to_game_response(game, None, None, false)))
}

/// `POST /games/:id/unarchive` — Restore an archived game.
async fn unarchive_game(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    if game.status != "archived" {
        return Err(AppError::Unprocessable(
            "NOT_ARCHIVED".to_string(),
            "Game is not currently archived".to_string(),
        ));
    }

    let new_status = if game.published_version_id.is_some() {
        "published"
    } else {
        "draft"
    };

    let mut active: game::ActiveModel = game.into();
    active.status = ActiveValue::Set(new_status.to_string());
    active.updated_at = ActiveValue::Set(chrono::Utc::now().into());
    let game = active.update(&state.db).await?;

    Ok(Json(to_game_response(game, None, None, false)))
}

/// `POST /games/:id/fork` — Fork a remixable game.
async fn fork_game(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let source = find_active_game(&state.db, id).await?;

    let pub_version_id = source.published_version_id.ok_or_else(|| {
        AppError::Unprocessable(
            "NO_PUBLISHED_VERSION".to_string(),
            "The source game has no published version".to_string(),
        )
    })?;

    let published_version = game_version::Entity::find_by_id(pub_version_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Published version not found".to_string()))?;

    let now = chrono::Utc::now();
    let new_id = Uuid::new_v4();
    let slug = unique_slug(&format!("{} fork", source.title), new_id);

    let forked = game::ActiveModel {
        id: ActiveValue::Set(new_id),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
        owner_id: ActiveValue::Set(user.id),
        title: ActiveValue::Set(format!("{} (Fork)", source.title)),
        slug: ActiveValue::Set(slug),
        description: ActiveValue::Set(source.description.clone()),
        technology: ActiveValue::Set(source.technology.clone()),
        min_players: ActiveValue::Set(source.min_players),
        max_players: ActiveValue::Set(source.max_players),
        status: ActiveValue::Set("draft".to_string()),
        visibility: ActiveValue::Set("private".to_string()),
        game_screen_code: ActiveValue::Set(published_version.game_screen_code),
        controller_screen_code: ActiveValue::Set(published_version.controller_screen_code),
        forked_from_id: ActiveValue::Set(Some(source.id)),
        ..Default::default()
    };

    let forked = forked.insert(&state.db).await?;

    Ok((
        StatusCode::CREATED,
        Json(to_game_response(forked, None, None, true)),
    ))
}

/// `GET /games/:id/versions` — List all published versions (paginated).
async fn list_versions(
    State(state): State<AppState>,
    OptionalAuth(opt_user): OptionalAuth,
    Path(id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    let user_id = opt_user.as_ref().map(|u| u.id);
    check_visibility(&game, user_id)?;

    let total = game_version::Entity::find()
        .filter(game_version::Column::GameId.eq(id))
        .count(&state.db)
        .await?;

    let versions = game_version::Entity::find()
        .filter(game_version::Column::GameId.eq(id))
        .order_by_desc(game_version::Column::VersionNumber)
        .offset(pagination.offset)
        .limit(pagination.limit)
        .all(&state.db)
        .await?;

    Ok(Json(PaginatedResponse {
        data: versions.into_iter().map(to_version_summary).collect(),
        total,
        offset: pagination.offset,
        limit: pagination.limit,
    }))
}

/// `GET /games/:id/versions/:versionNumber` — Get a specific version with full code.
async fn get_version(
    State(state): State<AppState>,
    OptionalAuth(opt_user): OptionalAuth,
    Path((id, version_number)): Path<(Uuid, i32)>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    let user_id = opt_user.as_ref().map(|u| u.id);
    check_visibility(&game, user_id)?;

    let version = game_version::Entity::find()
        .filter(game_version::Column::GameId.eq(id))
        .filter(game_version::Column::VersionNumber.eq(version_number))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Version not found".to_string()))?;

    Ok(Json(VersionDetailResponse {
        id: version.id,
        created_at: version.created_at.to_string(),
        game_id: version.game_id,
        version_number: version.version_number,
        game_screen_code: version.game_screen_code,
        controller_screen_code: version.controller_screen_code,
        changelog: version.changelog,
        published_by_id: version.published_by_id,
    }))
}

/// `POST /games/:id/assets` — Upload a file asset.
#[allow(clippy::items_after_statements)]
async fn upload_asset(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const ALLOWED_TYPES: &[&str] = &[
        "image/png",
        "image/jpeg",
        "image/svg+xml",
        "image/gif",
        "audio/mpeg",
        "audio/wav",
        "audio/ogg",
        "font/ttf",
        "font/woff2",
    ];

    let mut found_file_name = String::new();
    let mut found_data: Vec<u8> = Vec::new();
    let mut found_file_type = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        if field.name() == Some("file") {
            found_file_name = field.file_name().unwrap_or("upload").to_string();
            found_file_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Could not read file: {e}")))?;
            found_data = bytes.to_vec();
        }
    }

    if found_data.is_empty() {
        return Err(AppError::BadRequest("No file provided".to_string()));
    }

    if found_data.len() > MAX_FILE_SIZE {
        return Err(AppError::PayloadTooLarge(
            "File exceeds the 10 MB size limit".to_string(),
        ));
    }

    if !ALLOWED_TYPES.contains(&found_file_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unsupported file type: {found_file_type}"
        )));
    }

    let asset_id = Uuid::new_v4();
    let storage_url = format!("assets/{id}/{found_file_name}");

    let asset = game_asset::ActiveModel {
        id: ActiveValue::Set(asset_id),
        created_at: ActiveValue::Set(chrono::Utc::now().into()),
        game_id: ActiveValue::Set(id),
        file_name: ActiveValue::Set(found_file_name),
        file_type: ActiveValue::Set(found_file_type),
        file_size: ActiveValue::Set(i32::try_from(found_data.len()).unwrap_or(i32::MAX)),
        file_data: ActiveValue::Set(found_data),
        storage_url: ActiveValue::Set(storage_url),
        ..Default::default()
    };

    let asset = asset.insert(&state.db).await?;

    Ok((StatusCode::CREATED, Json(to_asset_response(asset))))
}

/// `GET /games/:id/assets` — List all assets for a game.
async fn list_assets(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    let assets = game_asset::Entity::find()
        .filter(game_asset::Column::GameId.eq(id))
        .filter(game_asset::Column::DeletedAt.is_null())
        .all(&state.db)
        .await?;

    let total = u64::try_from(assets.len()).unwrap_or(0);

    Ok(Json(PaginatedResponse {
        data: assets.into_iter().map(to_asset_response).collect(),
        total,
        offset: 0,
        limit: total,
    }))
}

/// `GET /games/:id/assets/:assetId` — Get a single asset's metadata.
async fn get_asset(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, asset_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    let asset = game_asset::Entity::find_by_id(asset_id)
        .filter(game_asset::Column::GameId.eq(id))
        .filter(game_asset::Column::DeletedAt.is_null())
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Asset not found".to_string()))?;

    Ok(Json(to_asset_response(asset)))
}

/// `DELETE /games/:id/assets/:assetId` — Soft-delete an asset.
async fn delete_asset(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, asset_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    let asset = game_asset::Entity::find_by_id(asset_id)
        .filter(game_asset::Column::GameId.eq(id))
        .filter(game_asset::Column::DeletedAt.is_null())
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Asset not found".to_string()))?;

    let mut a: game_asset::ActiveModel = asset.into();
    a.deleted_at = ActiveValue::Set(Some(chrono::Utc::now().into()));
    a.update(&state.db).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /tags` — List all platform tags (optionally filtered by category).
#[allow(clippy::items_after_statements)]
async fn list_tags(
    State(state): State<AppState>,
    Query(query): Query<TagCategoryQuery>,
) -> Result<impl IntoResponse, AppError> {
    let mut find = tag::Entity::find();
    if let Some(cat) = query.category {
        find = find.filter(tag::Column::Category.eq(cat));
    }

    let tags = find.all(&state.db).await?;

    #[derive(Serialize)]
    struct TagsResponse {
        data: Vec<TagResponse>,
    }

    Ok(Json(TagsResponse {
        data: tags.into_iter().map(to_tag_response).collect(),
    }))
}

/// `PUT /games/:id/tags` — Replace all tags on a game.
#[allow(clippy::items_after_statements)]
async fn set_game_tags(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetTagsRequest>,
) -> Result<impl IntoResponse, AppError> {
    let game = find_active_game(&state.db, id).await?;

    if game.owner_id != user.id {
        return Err(AppError::Forbidden(
            "You are not the creator of this game".to_string(),
        ));
    }

    // Verify all tag IDs exist
    let found_tags = tag::Entity::find()
        .filter(tag::Column::Id.is_in(req.tag_ids.clone()))
        .all(&state.db)
        .await?;

    if found_tags.len() != req.tag_ids.len() {
        return Err(AppError::BadRequest(
            "One or more tag IDs do not exist".to_string(),
        ));
    }

    // Replace: delete existing, then insert new
    game_tag::Entity::delete_many()
        .filter(game_tag::Column::GameId.eq(id))
        .exec(&state.db)
        .await?;

    for tag_id in req.tag_ids {
        game_tag::ActiveModel {
            game_id: ActiveValue::Set(id),
            tag_id: ActiveValue::Set(tag_id),
        }
        .insert(&state.db)
        .await?;
    }

    let tags = load_game_tags(&state.db, id).await?;

    #[derive(Serialize)]
    struct TagsResponse {
        tags: Vec<TagResponse>,
    }

    Ok(Json(TagsResponse { tags }))
}

/// `GET /games/:id/tags` — Get tags assigned to a game.
#[allow(clippy::items_after_statements)]
async fn get_game_tags(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Just confirm the game exists (no visibility restriction per spec)
    let _ = game::Entity::find_by_id(id)
        .filter(game::Column::DeletedAt.is_null())
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;

    let tags = load_game_tags(&state.db, id).await?;

    #[derive(Serialize)]
    struct TagsResponse {
        tags: Vec<TagResponse>,
    }

    Ok(Json(TagsResponse { tags }))
}

/// `GET /users/me/games` — List authenticated user's games.
///
/// # Errors
///
/// Returns [`AppError`] if the database query fails.
pub async fn list_my_games(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(query): Query<MyGamesQuery>,
) -> Result<impl IntoResponse, AppError> {
    let mut find = game::Entity::find()
        .filter(game::Column::OwnerId.eq(user.id))
        .filter(game::Column::DeletedAt.is_null());

    if let Some(status) = query.status {
        find = find.filter(game::Column::Status.eq(status));
    }

    let total = find.clone().count(&state.db).await?;

    let games = find
        .order_by_desc(game::Column::UpdatedAt)
        .offset(query.offset)
        .limit(query.limit)
        .all(&state.db)
        .await?;

    Ok(Json(PaginatedResponse {
        data: games.into_iter().map(to_game_summary).collect(),
        total,
        offset: query.offset,
        limit: query.limit,
    }))
}

/// `GET /users/:username/games` — List a user's public games.
///
/// # Errors
///
/// Returns [`AppError`] if the user is not found or the database query fails.
pub async fn list_user_games(
    State(state): State<AppState>,
    Path(username): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = user::Entity::find()
        .filter(user::Column::Username.eq(&username))
        .filter(user::Column::DeletedAt.is_null())
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let total = game::Entity::find()
        .filter(game::Column::OwnerId.eq(user.id))
        .filter(game::Column::DeletedAt.is_null())
        .filter(game::Column::Status.eq("published"))
        .filter(game::Column::Visibility.eq("public"))
        .count(&state.db)
        .await?;

    let games = game::Entity::find()
        .filter(game::Column::OwnerId.eq(user.id))
        .filter(game::Column::DeletedAt.is_null())
        .filter(game::Column::Status.eq("published"))
        .filter(game::Column::Visibility.eq("public"))
        .order_by_desc(game::Column::UpdatedAt)
        .offset(pagination.offset)
        .limit(pagination.limit)
        .all(&state.db)
        .await?;

    Ok(Json(PaginatedResponse {
        data: games.into_iter().map(to_game_summary).collect(),
        total,
        offset: pagination.offset,
        limit: pagination.limit,
    }))
}

// ============================================================================
// Helpers
// ============================================================================

async fn find_active_game(db: &DatabaseConnection, id: Uuid) -> Result<game::Model, AppError> {
    game::Entity::find_by_id(id)
        .filter(game::Column::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))
}

fn check_visibility(game: &game::Model, user_id: Option<Uuid>) -> Result<(), AppError> {
    if game.visibility == "private" {
        match user_id {
            Some(uid) if uid == game.owner_id => Ok(()),
            _ => Err(AppError::NotFound("Game not found".to_string())),
        }
    } else {
        // public or unlisted
        Ok(())
    }
}

async fn load_creator(db: &DatabaseConnection, user_id: Uuid) -> Result<CreatorInfo, AppError> {
    let u = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Creator not found".to_string()))?;

    Ok(CreatorInfo {
        username: u.username,
        display_name: u.display_name,
        avatar_url: u.avatar_url,
    })
}

async fn load_game_tags(
    db: &DatabaseConnection,
    game_id: Uuid,
) -> Result<Vec<TagResponse>, AppError> {
    let game_tags = game_tag::Entity::find()
        .filter(game_tag::Column::GameId.eq(game_id))
        .all(db)
        .await?;

    if game_tags.is_empty() {
        return Ok(Vec::new());
    }

    let tag_ids: Vec<Uuid> = game_tags.iter().map(|gt| gt.tag_id).collect();
    let tags = tag::Entity::find()
        .filter(tag::Column::Id.is_in(tag_ids))
        .all(db)
        .await?;

    Ok(tags.into_iter().map(to_tag_response).collect())
}

/// Generate a URL-safe slug suffixed with the game ID to guarantee uniqueness.
fn unique_slug(title: &str, id: Uuid) -> String {
    let base: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    format!("{base}-{id}")
}

fn to_game_response(
    game: game::Model,
    creator: Option<CreatorInfo>,
    tags: Option<Vec<TagResponse>>,
    include_code: bool,
) -> GameResponse {
    GameResponse {
        id: game.id,
        created_at: game.created_at.to_string(),
        updated_at: game.updated_at.to_string(),
        creator_id: game.owner_id,
        creator,
        title: game.title,
        description: game.description,
        thumbnail_url: game.thumbnail,
        technology: game.technology,
        min_players: game.min_players,
        max_players: game.max_players,
        status: game.status,
        visibility: game.visibility,
        forked_from_id: game.forked_from_id,
        game_screen_code: if include_code {
            game.game_screen_code
        } else {
            None
        },
        controller_screen_code: if include_code {
            game.controller_screen_code
        } else {
            None
        },
        published_version_id: game.published_version_id,
        play_count: game.play_count,
        total_play_time: game.total_play_time,
        avg_rating: game.avg_rating,
        review_count: game.review_count,
        tags,
    }
}

fn to_game_summary(game: game::Model) -> GameSummaryResponse {
    GameSummaryResponse {
        id: game.id,
        created_at: game.created_at.to_string(),
        updated_at: game.updated_at.to_string(),
        creator_id: game.owner_id,
        title: game.title,
        description: game.description,
        thumbnail_url: game.thumbnail,
        technology: game.technology,
        min_players: game.min_players,
        max_players: game.max_players,
        status: game.status,
        visibility: game.visibility,
        published_version_id: game.published_version_id,
        play_count: game.play_count,
        avg_rating: game.avg_rating,
        review_count: game.review_count,
    }
}

fn to_version_summary(v: game_version::Model) -> VersionSummaryResponse {
    VersionSummaryResponse {
        id: v.id,
        created_at: v.created_at.to_string(),
        version_number: v.version_number,
        changelog: v.changelog,
        published_by_id: v.published_by_id,
    }
}

fn to_asset_response(a: game_asset::Model) -> AssetResponse {
    AssetResponse {
        id: a.id,
        created_at: a.created_at.to_string(),
        game_id: a.game_id,
        file_name: a.file_name,
        file_type: a.file_type,
        file_size: a.file_size,
        storage_url: a.storage_url,
    }
}

fn to_tag_response(t: tag::Model) -> TagResponse {
    TagResponse {
        id: t.id,
        name: t.name,
        slug: t.slug,
        category: t.category,
    }
}
