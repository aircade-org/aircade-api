use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::auth::{extract_client_ip, jwt, oauth, password};
use crate::entities::{auth_provider, refresh_token, user};
use crate::error::AppError;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

/// Build the auth route group: `/auth/...`
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/signup/email", post(signup_email))
        .route("/signin/email", post(signin_email))
        .route("/verify-email", post(verify_email))
        .route("/resend-verification", post(resend_verification))
        .route("/password-reset/request", post(password_reset_request))
        .route("/password-reset/confirm", post(password_reset_confirm))
        .route("/password/change", post(password_change))
        .route("/oauth/google", get(oauth_google_initiate))
        .route("/oauth/google/callback", get(oauth_google_callback))
        .route("/oauth/github", get(oauth_github_initiate))
        .route("/oauth/github/callback", get(oauth_github_callback))
        .route(
            "/link/{provider}",
            post(link_provider).delete(unlink_provider),
        )
        .route("/refresh", post(refresh_token_handler))
        .route("/signout", post(signout))
}

// ─────────────────────────────────────────────────────────────────────────────
// DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SignupEmailRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SigninEmailRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: String,
    pub refresh_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub email_verified: bool,
    pub role: String,
    pub subscription_plan: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Deserialize)]
pub struct PasswordResetRequestBody {
    pub email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetConfirmBody {
    pub token: String,
    pub new_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChangeBody {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequestBody {
    pub refresh_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub token: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignoutRequestBody {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct LinkProviderRequest {
    pub code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkProviderResponse {
    pub provider: String,
    pub provider_email: Option<String>,
    pub linked_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthInitiateQuery {
    pub redirect_uri: Option<String>,
}

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn user_response(u: &user::Model) -> UserResponse {
    UserResponse {
        id: u.id,
        email: u.email.clone(),
        username: u.username.clone(),
        display_name: u.display_name.clone(),
        avatar_url: u.avatar_url.clone(),
        bio: u.bio.clone(),
        email_verified: u.email_verified,
        role: u.role.clone(),
        subscription_plan: u.subscription_plan.clone(),
        created_at: u.created_at.to_rfc3339(),
    }
}

/// Store a new refresh token record in the database.
async fn store_refresh_token(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    token_pair: &jwt::TokenPair,
) -> Result<(), AppError> {
    let now = Utc::now().fixed_offset();

    let record = refresh_token::ActiveModel {
        id: Set(token_pair.refresh_jti),
        user_id: Set(user_id),
        token_hash: Set(token_pair.refresh_jti.to_string()),
        expires_at: Set(token_pair.refresh_expires_at.fixed_offset()),
        revoked_at: Set(None),
        created_at: Set(now),
    };

    record
        .insert(db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(())
}

/// Generate a random verification/reset token.
fn generate_verification_token() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a unique username from a display name by adding a random suffix.
fn generate_username_from_name(name: &str) -> String {
    let base: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(40)
        .collect();
    let base = if base.len() < 3 {
        "user".to_string()
    } else {
        base
    };
    let suffix = rand::random::<u16>() % 10000;
    format!("{base}_{suffix}")
}

/// Parameters for creating or signing in an OAuth user.
struct OAuthUserParams {
    provider_name: String,
    provider_id: String,
    email: String,
    email_verified: bool,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

/// Shared logic: find or create a user for an OAuth callback, update login info, generate tokens.
async fn oauth_find_or_create_user(
    state: &AppState,
    headers: &HeaderMap,
    params: OAuthUserParams,
) -> Result<user::Model, AppError> {
    let existing_provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::Provider.eq(&params.provider_name))
        .filter(auth_provider::Column::ProviderId.eq(&params.provider_id))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if let Some(provider) = existing_provider {
        return oauth_signin_existing(state, headers, provider.user_id).await;
    }

    oauth_create_new_user(state, headers, &params).await
}

/// Sign in an existing OAuth user: check status, update login info.
async fn oauth_signin_existing(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
) -> Result<user::Model, AppError> {
    let user_model = user::Entity::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User not found for provider")))?;

    if user_model.account_status == "suspended" {
        return Err(AppError::Forbidden("Account is suspended.".to_string()));
    }
    if user_model.account_status == "deactivated" {
        return Err(AppError::Forbidden("Account is deactivated.".to_string()));
    }

    let client_ip = extract_client_ip(headers);
    let now = Utc::now().fixed_offset();
    let mut active_user: user::ActiveModel = user_model.into();
    active_user.last_login_at = Set(Some(now));
    active_user.last_login_ip = Set(client_ip);
    active_user.updated_at = Set(now);
    active_user
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))
}

/// Create a new user via OAuth: check email conflicts, insert user + provider.
async fn oauth_create_new_user(
    state: &AppState,
    headers: &HeaderMap,
    params: &OAuthUserParams,
) -> Result<user::Model, AppError> {
    let existing_user = user::Entity::find()
        .filter(user::Column::Email.eq(&params.email))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if existing_user.is_some() {
        return Err(AppError::Conflict(
            "Email is already registered via a different provider.".to_string(),
        ));
    }

    let now = Utc::now().fixed_offset();
    let user_id = Uuid::new_v4();
    let username = generate_username_from_name(params.display_name.as_deref().unwrap_or("user"));

    let txn = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let new_user = user::ActiveModel {
        id: Set(user_id),
        email: Set(params.email.clone()),
        username: Set(username),
        display_name: Set(params.display_name.clone()),
        avatar_url: Set(params.avatar_url.clone()),
        bio: Set(None),
        email_verified: Set(params.email_verified),
        role: Set("user".to_string()),
        subscription_plan: Set("free".to_string()),
        subscription_expires_at: Set(None),
        account_status: Set("active".to_string()),
        suspension_reason: Set(None),
        last_login_at: Set(Some(now)),
        last_login_ip: Set(extract_client_ip(headers)),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };
    let user_model = new_user
        .insert(&txn)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let new_provider = auth_provider::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        provider: Set(params.provider_name.clone()),
        provider_id: Set(params.provider_id.clone()),
        password_hash: Set(None),
        provider_email: Set(Some(params.email.clone())),
        verification_token: Set(None),
        token_expires_at: Set(None),
        created_at: Set(now),
    };
    new_provider
        .insert(&txn)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    txn.commit()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(user_model)
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/auth/signup/email`
async fn signup_email(
    State(state): State<AppState>,
    Json(body): Json<SignupEmailRequest>,
) -> Result<Response, AppError> {
    let email = body.email.trim().to_lowercase();
    let username = body.username.trim().to_string();

    // Validate input
    password::validate_email(&email).map_err(AppError::BadRequest)?;
    password::validate_username(&username).map_err(AppError::BadRequest)?;
    password::validate_password(&body.password).map_err(AppError::BadRequest)?;

    // Check for existing user with same email
    let existing_email = user::Entity::find()
        .filter(user::Column::Email.eq(&email))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if existing_email.is_some() {
        return Err(AppError::Conflict("Email already registered.".to_string()));
    }

    // Check for existing user with same username (case-insensitive)
    let existing_username = user::Entity::find()
        .filter(user::Column::Username.eq(&username))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if existing_username.is_some() {
        return Err(AppError::Conflict("Username already taken.".to_string()));
    }

    // Hash password
    let password_hash = password::hash_password(&body.password)?;

    // Generate verification token
    let verification_token = generate_verification_token();
    let token_expires_at = Utc::now() + chrono::Duration::hours(24);

    let now = Utc::now().fixed_offset();
    let user_id = Uuid::new_v4();
    let auth_provider_id = Uuid::new_v4();

    // Create user and auth provider in a transaction
    let txn = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let new_user = user::ActiveModel {
        id: Set(user_id),
        email: Set(email.clone()),
        username: Set(username),
        display_name: Set(None),
        avatar_url: Set(None),
        bio: Set(None),
        email_verified: Set(false),
        role: Set("user".to_string()),
        subscription_plan: Set("free".to_string()),
        subscription_expires_at: Set(None),
        account_status: Set("active".to_string()),
        suspension_reason: Set(None),
        last_login_at: Set(None),
        last_login_ip: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };
    let user_model = new_user
        .insert(&txn)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let new_provider = auth_provider::ActiveModel {
        id: Set(auth_provider_id),
        user_id: Set(user_id),
        provider: Set("email".to_string()),
        provider_id: Set(email.clone()),
        password_hash: Set(Some(password_hash)),
        provider_email: Set(Some(email.clone())),
        verification_token: Set(Some(verification_token.clone())),
        token_expires_at: Set(Some(token_expires_at.fixed_offset())),
        created_at: Set(now),
    };
    new_provider
        .insert(&txn)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    txn.commit()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Log verification token (stub for email sending)
    tracing::info!(
        email = %email,
        token = %verification_token,
        "Email verification token generated (email sending not yet implemented)"
    );

    // Generate tokens
    let token_pair = jwt::generate_token_pair(user_id, &user_model.role, &state.config)?;
    store_refresh_token(&state.db, user_id, &token_pair).await?;

    let response = AuthResponse {
        user: user_response(&user_model),
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
    };

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// `POST /api/v1/auth/signin/email`
async fn signin_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SigninEmailRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = body.email.trim().to_lowercase();

    // Find user by email
    let user_model = user::Entity::find()
        .filter(user::Column::Email.eq(&email))
        .filter(user::Column::DeletedAt.is_null())
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password.".to_string()))?;

    // Check account status
    if user_model.account_status == "suspended" {
        return Err(AppError::Forbidden("Account is suspended.".to_string()));
    }
    if user_model.account_status == "deactivated" {
        return Err(AppError::Forbidden("Account is deactivated.".to_string()));
    }

    // Find email auth provider
    let provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_model.id))
        .filter(auth_provider::Column::Provider.eq("email"))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password.".to_string()))?;

    // Verify password
    let hash = provider
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password.".to_string()))?;
    let valid = password::verify_password(&body.password, hash)?;
    if !valid {
        return Err(AppError::Unauthorized(
            "Invalid email or password.".to_string(),
        ));
    }

    // Update last login info
    let client_ip = extract_client_ip(&headers);
    let now = Utc::now().fixed_offset();
    let mut active_user: user::ActiveModel = user_model.clone().into();
    active_user.last_login_at = Set(Some(now));
    active_user.last_login_ip = Set(client_ip);
    active_user.updated_at = Set(now);
    let user_model = active_user
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Generate tokens
    let token_pair = jwt::generate_token_pair(user_model.id, &user_model.role, &state.config)?;
    store_refresh_token(&state.db, user_model.id, &token_pair).await?;

    Ok(Json(AuthResponse {
        user: user_response(&user_model),
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
    }))
}

/// `POST /api/v1/auth/verify-email`
async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<VerifyEmailRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    // Find auth provider by verification token
    let provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::VerificationToken.eq(&body.token))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::BadRequest("Invalid verification token.".to_string()))?;

    // Check token expiry
    if let Some(expires_at) = provider.token_expires_at
        && expires_at < Utc::now().fixed_offset()
    {
        return Err(AppError::BadRequest(
            "Verification token has expired.".to_string(),
        ));
    }

    // Set email verified on user
    let user_model = user::Entity::find_by_id(provider.user_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    let now = Utc::now().fixed_offset();
    let mut active_user: user::ActiveModel = user_model.into();
    active_user.email_verified = Set(true);
    active_user.updated_at = Set(now);
    active_user
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Clear verification token
    let mut active_provider: auth_provider::ActiveModel = provider.into();
    active_provider.verification_token = Set(None);
    active_provider.token_expires_at = Set(None);
    active_provider
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(MessageResponse {
        message: "Email verified successfully.".to_string(),
    }))
}

/// `POST /api/v1/auth/resend-verification`
async fn resend_verification(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
) -> Result<Json<MessageResponse>, AppError> {
    if user_model.email_verified {
        return Err(AppError::UnprocessableEntity(
            "Email is already verified.".to_string(),
        ));
    }

    // Find email auth provider
    let provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_model.id))
        .filter(auth_provider::Column::Provider.eq("email"))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("No email auth provider linked.".to_string()))?;

    // Generate new token
    let verification_token = generate_verification_token();
    let token_expires_at = Utc::now() + chrono::Duration::hours(24);

    let mut active_provider: auth_provider::ActiveModel = provider.into();
    active_provider.verification_token = Set(Some(verification_token.clone()));
    active_provider.token_expires_at = Set(Some(token_expires_at.fixed_offset()));
    active_provider
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    tracing::info!(
        email = %user_model.email,
        token = %verification_token,
        "Verification email resent (email sending not yet implemented)"
    );

    Ok(Json(MessageResponse {
        message: "Verification email sent.".to_string(),
    }))
}

/// `POST /api/v1/auth/password-reset/request`
async fn password_reset_request(
    State(state): State<AppState>,
    Json(body): Json<PasswordResetRequestBody>,
) -> Result<Json<MessageResponse>, AppError> {
    let email = body.email.trim().to_lowercase();
    let constant_message = "If an account with that email exists, a reset link has been sent.";

    // Always return success to prevent email enumeration
    let user_opt = user::Entity::find()
        .filter(user::Column::Email.eq(&email))
        .filter(user::Column::DeletedAt.is_null())
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if let Some(user_model) = user_opt {
        let provider_opt = auth_provider::Entity::find()
            .filter(auth_provider::Column::UserId.eq(user_model.id))
            .filter(auth_provider::Column::Provider.eq("email"))
            .one(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        if let Some(provider) = provider_opt {
            let reset_token = generate_verification_token();
            let token_expires_at = Utc::now() + chrono::Duration::hours(1);

            let mut active_provider: auth_provider::ActiveModel = provider.into();
            active_provider.verification_token = Set(Some(reset_token.clone()));
            active_provider.token_expires_at = Set(Some(token_expires_at.fixed_offset()));
            active_provider
                .update(&state.db)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;

            tracing::info!(
                email = %email,
                token = %reset_token,
                "Password reset token generated (email sending not yet implemented)"
            );
        }
    }

    Ok(Json(MessageResponse {
        message: constant_message.to_string(),
    }))
}

/// `POST /api/v1/auth/password-reset/confirm`
async fn password_reset_confirm(
    State(state): State<AppState>,
    Json(body): Json<PasswordResetConfirmBody>,
) -> Result<Json<MessageResponse>, AppError> {
    // Find auth provider by token
    let provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::VerificationToken.eq(&body.token))
        .filter(auth_provider::Column::Provider.eq("email"))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token.".to_string()))?;

    // Check expiry
    if let Some(expires_at) = provider.token_expires_at {
        if expires_at < Utc::now().fixed_offset() {
            return Err(AppError::BadRequest("Reset token has expired.".to_string()));
        }
    } else {
        return Err(AppError::BadRequest(
            "Invalid or expired reset token.".to_string(),
        ));
    }

    // Validate new password
    password::validate_password(&body.new_password).map_err(AppError::BadRequest)?;

    // Hash and update
    let new_hash = password::hash_password(&body.new_password)?;
    let mut active_provider: auth_provider::ActiveModel = provider.into();
    active_provider.password_hash = Set(Some(new_hash));
    active_provider.verification_token = Set(None);
    active_provider.token_expires_at = Set(None);
    active_provider
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(MessageResponse {
        message: "Password has been reset.".to_string(),
    }))
}

/// `POST /api/v1/auth/password/change`
async fn password_change(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    Json(body): Json<PasswordChangeBody>,
) -> Result<Json<MessageResponse>, AppError> {
    // Find email auth provider
    let provider = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_model.id))
        .filter(auth_provider::Column::Provider.eq("email"))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound("No email auth provider linked.".to_string()))?;

    // Verify current password
    let hash = provider
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::Unauthorized("Current password is incorrect.".to_string()))?;
    let valid = password::verify_password(&body.current_password, hash)?;
    if !valid {
        return Err(AppError::Unauthorized(
            "Current password is incorrect.".to_string(),
        ));
    }

    // Validate and hash new password
    password::validate_password(&body.new_password).map_err(AppError::BadRequest)?;
    let new_hash = password::hash_password(&body.new_password)?;

    let mut active_provider: auth_provider::ActiveModel = provider.into();
    active_provider.password_hash = Set(Some(new_hash));
    active_provider
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(MessageResponse {
        message: "Password changed.".to_string(),
    }))
}

/// `GET /api/v1/auth/oauth/google`
async fn oauth_google_initiate(
    State(state): State<AppState>,
    Query(query): Query<OAuthInitiateQuery>,
) -> Result<Response, AppError> {
    if state.config.google_client_id.is_empty() {
        return Err(AppError::UnprocessableEntity(
            "Google OAuth is not configured.".to_string(),
        ));
    }

    let client = oauth::google_client(&state.config)?;
    let state_token =
        jwt::generate_oauth_state(&state.config.jwt_secret, query.redirect_uri.as_deref())?;

    let (auth_url, _csrf) = client
        .authorize_url(|| CsrfToken::new(state_token))
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    Ok(Redirect::to(auth_url.as_str()).into_response())
}

/// `GET /api/v1/auth/oauth/google/callback`
async fn oauth_google_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, AppError> {
    let state_claims = jwt::validate_oauth_state(&query.state, &state.config.jwt_secret)
        .map_err(|_| AppError::BadRequest("Invalid or expired OAuth state.".to_string()))?;

    let client = oauth::google_client(&state.config)?;
    let token_result = client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&reqwest::Client::new())
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to exchange authorization code: {e}")))?;

    let access_token = token_result.access_token().secret().clone();
    let google_user = oauth::fetch_google_userinfo(&access_token).await?;

    let user_model = oauth_find_or_create_user(
        &state,
        &headers,
        OAuthUserParams {
            provider_name: "google".to_string(),
            provider_id: google_user.sub,
            email: google_user.email,
            email_verified: google_user.email_verified.unwrap_or(true),
            display_name: google_user.name,
            avatar_url: google_user.picture,
        },
    )
    .await?;

    let token_pair = jwt::generate_token_pair(user_model.id, &user_model.role, &state.config)?;
    store_refresh_token(&state.db, user_model.id, &token_pair).await?;

    let auth_response = AuthResponse {
        user: user_response(&user_model),
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
    };

    // If redirect_uri was provided, redirect to frontend with auth data
    if let Some(redirect_uri) = state_claims.redirect_uri {
        let user_json =
            serde_json::to_string(&auth_response.user).unwrap_or_else(|_| "{}".to_string());
        let redirect_url = format!(
            "{}?provider=google&token={}&refreshToken={}&user={}",
            redirect_uri,
            urlencoding::encode(&auth_response.token),
            urlencoding::encode(&auth_response.refresh_token),
            urlencoding::encode(&user_json)
        );
        return Ok(Redirect::to(&redirect_url).into_response());
    }

    // Fallback: return JSON for API clients
    Ok(Json(auth_response).into_response())
}

/// `GET /api/v1/auth/oauth/github`
async fn oauth_github_initiate(
    State(state): State<AppState>,
    Query(query): Query<OAuthInitiateQuery>,
) -> Result<Response, AppError> {
    if state.config.github_client_id.is_empty() {
        return Err(AppError::UnprocessableEntity(
            "GitHub OAuth is not configured.".to_string(),
        ));
    }

    let client = oauth::github_client(&state.config)?;
    let state_token =
        jwt::generate_oauth_state(&state.config.jwt_secret, query.redirect_uri.as_deref())?;

    let (auth_url, _csrf) = client
        .authorize_url(|| CsrfToken::new(state_token))
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    Ok(Redirect::to(auth_url.as_str()).into_response())
}

/// `GET /api/v1/auth/oauth/github/callback`
async fn oauth_github_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, AppError> {
    let state_claims = jwt::validate_oauth_state(&query.state, &state.config.jwt_secret)
        .map_err(|_| AppError::BadRequest("Invalid or expired OAuth state.".to_string()))?;

    let client = oauth::github_client(&state.config)?;
    let token_result = client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&reqwest::Client::new())
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to exchange authorization code: {e}")))?;

    let access_token = token_result.access_token().secret().clone();
    let github_user = oauth::fetch_github_userinfo(&access_token).await?;
    let github_id = github_user.id.to_string();

    let email = if let Some(ref email) = github_user.email {
        email.clone()
    } else {
        oauth::fetch_github_primary_email(&access_token).await?
    };

    let user_model = oauth_find_or_create_user(
        &state,
        &headers,
        OAuthUserParams {
            provider_name: "github".to_string(),
            provider_id: github_id,
            email,
            email_verified: true,
            display_name: github_user.name,
            avatar_url: github_user.avatar_url,
        },
    )
    .await?;

    let token_pair = jwt::generate_token_pair(user_model.id, &user_model.role, &state.config)?;
    store_refresh_token(&state.db, user_model.id, &token_pair).await?;

    let auth_response = AuthResponse {
        user: user_response(&user_model),
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
    };

    // If redirect_uri was provided, redirect to frontend with auth data
    if let Some(redirect_uri) = state_claims.redirect_uri {
        let user_json =
            serde_json::to_string(&auth_response.user).unwrap_or_else(|_| "{}".to_string());
        let redirect_url = format!(
            "{}?provider=github&token={}&refreshToken={}&user={}",
            redirect_uri,
            urlencoding::encode(&auth_response.token),
            urlencoding::encode(&auth_response.refresh_token),
            urlencoding::encode(&user_json)
        );
        return Ok(Redirect::to(&redirect_url).into_response());
    }

    // Fallback: return JSON for API clients
    Ok(Json(auth_response).into_response())
}

/// `POST /api/v1/auth/link/{provider}`
async fn link_provider(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    Path(provider): Path<String>,
    Json(body): Json<LinkProviderRequest>,
) -> Result<Response, AppError> {
    if provider != "google" && provider != "github" {
        return Err(AppError::BadRequest(format!(
            "Unsupported provider: {provider}"
        )));
    }

    // Exchange code and get user info based on provider
    let (provider_id, provider_email) = match provider.as_str() {
        "google" => {
            let client = oauth::google_client(&state.config)?;
            let token_result = client
                .exchange_code(AuthorizationCode::new(body.code))
                .request_async(&reqwest::Client::new())
                .await
                .map_err(|e| AppError::BadRequest(format!("Invalid authorization code: {e}")))?;
            let access_token = token_result.access_token().secret().clone();
            let info = oauth::fetch_google_userinfo(&access_token).await?;
            (info.sub, Some(info.email))
        }
        "github" => {
            let client = oauth::github_client(&state.config)?;
            let token_result = client
                .exchange_code(AuthorizationCode::new(body.code))
                .request_async(&reqwest::Client::new())
                .await
                .map_err(|e| AppError::BadRequest(format!("Invalid authorization code: {e}")))?;
            let access_token = token_result.access_token().secret().clone();
            let info = oauth::fetch_github_userinfo(&access_token).await?;
            let email = if let Some(ref email) = info.email {
                Some(email.clone())
            } else {
                oauth::fetch_github_primary_email(&access_token).await.ok()
            };
            (info.id.to_string(), email)
        }
        _ => return Err(AppError::BadRequest("Unsupported provider.".to_string())),
    };

    // Check if provider_id is already linked to another user
    let existing = auth_provider::Entity::find()
        .filter(auth_provider::Column::ProviderId.eq(&provider_id))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if let Some(existing) = existing {
        if existing.user_id == user_model.id {
            return Err(AppError::Conflict(
                "This provider is already linked to your account.".to_string(),
            ));
        }
        return Err(AppError::Conflict(
            "This provider account is linked to a different user.".to_string(),
        ));
    }

    // Check if user already has this provider type
    let existing_type = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_model.id))
        .filter(auth_provider::Column::Provider.eq(&provider))
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if existing_type.is_some() {
        return Err(AppError::Conflict(format!(
            "{provider} is already linked to your account."
        )));
    }

    let now = Utc::now().fixed_offset();
    let new_provider = auth_provider::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_model.id),
        provider: Set(provider.clone()),
        provider_id: Set(provider_id),
        password_hash: Set(None),
        provider_email: Set(provider_email.clone()),
        verification_token: Set(None),
        token_expires_at: Set(None),
        created_at: Set(now),
    };
    new_provider
        .insert(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(LinkProviderResponse {
            provider,
            provider_email,
            linked_at: now.to_rfc3339(),
        }),
    )
        .into_response())
}

/// `DELETE /api/v1/auth/link/{provider}`
async fn unlink_provider(
    State(state): State<AppState>,
    AuthUser(user_model): AuthUser,
    Path(provider): Path<String>,
) -> Result<StatusCode, AppError> {
    // Count auth providers for user
    let providers = auth_provider::Entity::find()
        .filter(auth_provider::Column::UserId.eq(user_model.id))
        .all(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if providers.len() <= 1 {
        return Err(AppError::UnprocessableEntity(
            "Cannot unlink the last remaining auth provider.".to_string(),
        ));
    }

    // Find and delete the specific provider
    let target = providers
        .into_iter()
        .find(|p| p.provider == provider)
        .ok_or_else(|| {
            AppError::NotFound("This provider is not linked to your account.".to_string())
        })?;

    let active: auth_provider::ActiveModel = target.into();
    active
        .delete(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/auth/refresh`
async fn refresh_token_handler(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequestBody>,
) -> Result<Json<RefreshResponse>, AppError> {
    // Validate refresh token JWT
    let claims = jwt::validate_refresh_token(&body.refresh_token, &state.config.jwt_secret)
        .map_err(|_| AppError::Unauthorized("Invalid or expired refresh token.".to_string()))?;

    // Look up refresh token record in DB
    let jti: Uuid = claims
        .jti
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid refresh token.".to_string()))?;

    let token_record = refresh_token::Entity::find_by_id(jti)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::Unauthorized("Refresh token not found.".to_string()))?;

    // Check if revoked
    if token_record.revoked_at.is_some() {
        return Err(AppError::Unauthorized(
            "Refresh token has been revoked.".to_string(),
        ));
    }

    // Revoke the old token
    let now = Utc::now().fixed_offset();
    let mut active_token: refresh_token::ActiveModel = token_record.clone().into();
    active_token.revoked_at = Set(Some(now));
    active_token
        .update(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Look up user
    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid token subject.".to_string()))?;
    let user_model = user::Entity::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::Unauthorized("User not found.".to_string()))?;

    // Check account status
    if user_model.account_status == "suspended" {
        return Err(AppError::Forbidden("Account is suspended.".to_string()));
    }
    if user_model.account_status == "deactivated" {
        return Err(AppError::Forbidden("Account is deactivated.".to_string()));
    }

    // Generate new token pair
    let token_pair = jwt::generate_token_pair(user_model.id, &user_model.role, &state.config)?;
    store_refresh_token(&state.db, user_model.id, &token_pair).await?;

    Ok(Json(RefreshResponse {
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
    }))
}

/// `POST /api/v1/auth/signout`
async fn signout(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Json(body): Json<SignoutRequestBody>,
) -> Result<StatusCode, AppError> {
    // Try to decode the refresh token to get the jti
    if let Ok(claims) = jwt::validate_refresh_token(&body.refresh_token, &state.config.jwt_secret)
        && let Ok(jti) = claims.jti.parse::<Uuid>()
    {
        let token_record = refresh_token::Entity::find_by_id(jti)
            .one(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        if let Some(record) = token_record {
            let now = Utc::now().fixed_offset();
            let mut active_token: refresh_token::ActiveModel = record.into();
            active_token.revoked_at = Set(Some(now));
            active_token
                .update(&state.db)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
