use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::auth::password;
use crate::entities::{auth_provider, user};
use crate::error::AppError;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

/// Build the user route group: `/users/...`
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/me",
            get(get_me).patch(update_me).delete(deactivate_account),
        )
        .route("/me/avatar", post(upload_avatar).delete(delete_avatar))
        .route("/me/username", patch(change_username))
        .route("/me/email", patch(change_email))
        .route("/{username}", get(get_public_profile))
}

// ─────────────────────────────────────────────────────────────────────────────
// DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MeResponse {
    id: Uuid,
    created_at: String,
    updated_at: String,
    email: String,
    username: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    bio: Option<String>,
    email_verified: bool,
    role: String,
    subscription_plan: String,
    subscription_expires_at: Option<String>,
    account_status: String,
    last_login_at: Option<String>,
    auth_providers: Vec<AuthProviderInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthProviderInfo {
    provider: String,
    provider_email: Option<String>,
    linked_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateMeRequest {
    display_name: Option<String>,
    bio: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicProfileResponse {
    id: Uuid,
    username: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    bio: Option<String>,
    created_at: String,
    stats: PublicStats,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicStats {
    games_published: u64,
    total_play_count: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AvatarResponse {
    avatar_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeUsernameRequest {
    new_username: String,
}

#[derive(Serialize)]
struct UsernameResponse {
    username: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeEmailRequest {
    new_email: String,
    password: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangeEmailResponse {
    message: String,
    email: String,
    email_verified: bool,
}

#[derive(Deserialize)]
struct DeactivateRequest {
    password: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build the full `MeResponse` including linked auth providers.
async fn build_me_response(
    db: &sea_orm::DatabaseConnection,
    user_model: &user::Model,
) -> Result<MeResponse, AppError> {
    let providers = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_model.id))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let auth_providers = providers
        .into_iter()
        .map(|p| AuthProviderInfo {
            provider: p.provider,
            provider_email: p.provider_email,
            linked_at: p.created_at.to_rfc3339(),
        })
        .collect();

    Ok(MeResponse {
        id: user_model.id,
        created_at: user_model.created_at.to_rfc3339(),
        updated_at: user_model.updated_at.to_rfc3339(),
        email: user_model.email.clone(),
        username: user_model.username.clone(),
        display_name: user_model.display_name.clone(),
        avatar_url: user_model.avatar_url.clone(),
        bio: user_model.bio.clone(),
        email_verified: user_model.email_verified,
        role: user_model.role.clone(),
        subscription_plan: user_model.subscription_plan.clone(),
        subscription_expires_at: user_model.subscription_expires_at.map(|t| t.to_rfc3339()),
        account_status: user_model.account_status.clone(),
        last_login_at: user_model.last_login_at.map(|t| t.to_rfc3339()),
        auth_providers,
    })
}

/// Validate optional `display_name` length.
fn validate_display_name(name: &str) -> Result<(), String> {
    if name.len() > 100 {
        return Err("Display name must be at most 100 characters.".to_string());
    }
    Ok(())
}

/// Validate optional `bio` length.
fn validate_bio(bio: &str) -> Result<(), String> {
    if bio.len() > 500 {
        return Err("Bio must be at most 500 characters.".to_string());
    }
    Ok(())
}

/// Check if the user has an email auth provider and verify password when required.
async fn verify_account_ownership(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    supplied_password: Option<&str>,
) -> Result<(), AppError> {
    let email_provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_id))
        .filter(auth_provider::Column::Provider.eq("email"))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if let Some(provider) = email_provider {
        // User has email provider — password is required
        let pw = supplied_password.ok_or_else(|| {
            AppError::BadRequest("Password is required for email-linked accounts.".to_string())
        })?;

        let hash = provider.password_hash.as_deref().ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Email provider has no password hash"))
        })?;

        let valid = password::verify_password(pw, hash)?;
        if !valid {
            return Err(AppError::Unauthorized("Password is incorrect.".to_string()));
        }
    }
    // OAuth-only users: no password required
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/users/me`
async fn get_me(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
) -> Result<Json<MeResponse>, AppError> {
    let response = build_me_response(&state.db, &user_model).await?;
    Ok(Json(response))
}

/// `PATCH /api/v1/users/me`
async fn update_me(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    Json(body): Json<UpdateMeRequest>,
) -> Result<Json<MeResponse>, AppError> {
    let mut active: user::ActiveModel = user_model.clone().into();

    if let Some(ref display_name) = body.display_name {
        validate_display_name(display_name).map_err(AppError::BadRequest)?;
        active.display_name = Set(Some(display_name.clone()));
    }

    if let Some(ref bio) = body.bio {
        validate_bio(bio).map_err(AppError::BadRequest)?;
        active.bio = Set(Some(bio.clone()));
    }

    if let Some(ref avatar_url) = body.avatar_url {
        active.avatar_url = Set(Some(avatar_url.clone()));
    }

    let changed = body.display_name.is_some() || body.bio.is_some() || body.avatar_url.is_some();

    let updated_user = if changed {
        let now = Utc::now().fixed_offset();
        active.updated_at = Set(now);
        active
            .update(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?
    } else {
        user_model
    };

    let response = build_me_response(&state.db, &updated_user).await?;
    Ok(Json(response))
}

/// `POST /api/v1/users/me/avatar`
async fn upload_avatar(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    mut multipart: Multipart,
) -> Result<Json<AvatarResponse>, AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart data: {e}")))?
        .ok_or_else(|| AppError::BadRequest("No file field provided.".to_string()))?;

    let file_name = field.file_name().unwrap_or("upload.bin").to_string();

    // Validate file extension
    let extension = file_name.rsplit('.').next().unwrap_or("").to_lowercase();

    if !["png", "jpg", "jpeg", "gif", "svg"].contains(&extension.as_str()) {
        return Err(AppError::BadRequest(
            "Unsupported file type. Allowed: PNG, JPG, GIF, SVG.".to_string(),
        ));
    }

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to read file data: {e}")))?;

    // 5 MB limit
    if data.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest(
            "File exceeds the 5 MB size limit.".to_string(),
        ));
    }

    // Ensure upload directory exists
    let upload_dir = std::path::Path::new(&state.config.upload_dir).join("avatars");
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create upload dir: {e}")))?;

    // Generate unique filename
    let file_id = Uuid::new_v4();
    let stored_name = format!("{file_id}.{extension}");
    let file_path = upload_dir.join(&stored_name);

    tokio::fs::write(&file_path, &data)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to write file: {e}")))?;

    let avatar_url = format!("avatars/{stored_name}");

    // Update user record
    let now = Utc::now().fixed_offset();
    let mut active: user::ActiveModel = user_model.into();
    active.avatar_url = Set(Some(avatar_url.clone()));
    active.updated_at = Set(now);
    active
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(AvatarResponse { avatar_url }))
}

/// `DELETE /api/v1/users/me/avatar`
async fn delete_avatar(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
) -> Result<StatusCode, AppError> {
    // Optionally delete the file from disk
    if let Some(ref url) = user_model.avatar_url {
        let file_path = std::path::Path::new(&state.config.upload_dir).join(url);
        // Best-effort delete: ignore errors if the file doesn't exist
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    let now = Utc::now().fixed_offset();
    let mut active: user::ActiveModel = user_model.into();
    active.avatar_url = Set(None);
    active.updated_at = Set(now);
    active
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// `PATCH /api/v1/users/me/username`
async fn change_username(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    Json(body): Json<ChangeUsernameRequest>,
) -> Result<Json<UsernameResponse>, AppError> {
    let new_username = body.new_username.trim().to_string();

    password::validate_username(&new_username).map_err(AppError::BadRequest)?;

    // Check uniqueness
    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(&new_username))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if let Some(existing_user) = existing
        && existing_user.id != user_model.id
    {
        return Err(AppError::Conflict("Username is already taken.".to_string()));
    }

    let now = Utc::now().fixed_offset();
    let mut active: user::ActiveModel = user_model.into();
    active.username = Set(new_username.clone());
    active.updated_at = Set(now);
    active
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(UsernameResponse {
        username: new_username,
    }))
}

/// `PATCH /api/v1/users/me/email`
async fn change_email(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    Json(body): Json<ChangeEmailRequest>,
) -> Result<Json<ChangeEmailResponse>, AppError> {
    let new_email = body.new_email.trim().to_lowercase();

    password::validate_email(&new_email).map_err(AppError::BadRequest)?;

    // Check uniqueness
    let existing = user::Entity::find()
        .filter(user::Column::Email.eq(&new_email))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if let Some(existing_user) = existing
        && existing_user.id != user_model.id
    {
        return Err(AppError::Conflict("Email is already in use.".to_string()));
    }

    // Verify ownership (password check for email-linked accounts)
    let user_id = user_model.id;
    verify_account_ownership(&state.db, user_id, body.password.as_deref()).await?;

    // Update email and reset verification
    let now = Utc::now().fixed_offset();
    let verification_token = Uuid::new_v4().to_string();
    let token_expires_at = Utc::now() + chrono::Duration::hours(24);

    let mut active: user::ActiveModel = user_model.into();
    active.email = Set(new_email.clone());
    active.email_verified = Set(false);
    active.updated_at = Set(now);
    active
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Update verification token on email provider (if exists)
    let email_provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_id))
        .filter(auth_provider::Column::Provider.eq("email"))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if let Some(provider) = email_provider {
        let mut active_provider: auth_provider::ActiveModel = provider.into();
        active_provider.provider_id = Set(new_email.clone());
        active_provider.provider_email = Set(Some(new_email.clone()));
        active_provider.verification_token = Set(Some(verification_token.clone()));
        active_provider.token_expires_at = Set(Some(token_expires_at.fixed_offset()));
        active_provider
            .update(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
    }

    tracing::info!(
        email = %new_email,
        token = %verification_token,
        "Email changed, verification email pending (email sending not yet implemented)"
    );

    Ok(Json(ChangeEmailResponse {
        message: "Email updated. A verification email has been sent to the new address."
            .to_string(),
        email: new_email,
        email_verified: false,
    }))
}

/// `DELETE /api/v1/users/me`
async fn deactivate_account(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    Json(body): Json<DeactivateRequest>,
) -> Result<StatusCode, AppError> {
    // Verify ownership
    verify_account_ownership(&state.db, user_model.id, body.password.as_deref()).await?;

    let now = Utc::now().fixed_offset();
    let mut active: user::ActiveModel = user_model.into();
    active.account_status = Set("deactivated".to_string());
    active.deleted_at = Set(Some(now));
    active.updated_at = Set(now);
    active
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/users/{username}`
async fn get_public_profile(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Response, AppError> {
    let user_model = user::Entity::find()
        .filter(user::Column::Username.eq(&username))
        .filter(user::Column::DeletedAt.is_null())
        .filter(user::Column::AccountStatus.eq("active"))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    // Game stats are stubbed until the Game entity is implemented (M0.4.0)
    let profile_stats = PublicStats {
        games_published: 0,
        total_play_count: 0,
    };

    let response = PublicProfileResponse {
        id: user_model.id,
        username: user_model.username,
        display_name: user_model.display_name,
        avatar_url: user_model.avatar_url,
        bio: user_model.bio,
        created_at: user_model.created_at.to_rfc3339(),
        stats: profile_stats,
    };

    Ok(Json(response).into_response())
}
