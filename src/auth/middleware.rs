use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sea_orm::EntityTrait;

use crate::auth::jwt;
use crate::entities::user;
use crate::error::AppError;
use crate::state::AppState;

/// Authenticated user extracted from the `Authorization: Bearer <token>` header.
///
/// Use as an extractor in handler parameters to require authentication:
/// ```ignore
/// async fn handler(AuthUser(user): AuthUser) -> impl IntoResponse { ... }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser(pub user::Model);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing authorization header.".to_string()))?;

        let token = header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("Invalid authorization header format.".to_string())
        })?;

        let claims = jwt::validate_access_token(token, &state.config.jwt_secret)
            .map_err(|_| AppError::Unauthorized("Invalid or expired token.".to_string()))?;

        let user_id: uuid::Uuid = claims
            .sub
            .parse()
            .map_err(|_| AppError::Unauthorized("Invalid token subject.".to_string()))?;

        let user_model = user::Entity::find_by_id(user_id)
            .one(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .ok_or_else(|| AppError::Unauthorized("User not found.".to_string()))?;

        // Reject soft-deleted accounts
        if user_model.deleted_at.is_some() {
            return Err(AppError::Unauthorized("User not found.".to_string()));
        }

        // Reject suspended accounts
        if user_model.account_status == "suspended" {
            let reason = user_model
                .suspension_reason
                .as_deref()
                .unwrap_or("No reason provided");
            return Err(AppError::Forbidden(format!(
                "Account is suspended: {reason}"
            )));
        }

        // Reject deactivated accounts
        if user_model.account_status == "deactivated" {
            return Err(AppError::Forbidden("Account is deactivated.".to_string()));
        }

        Ok(Self(user_model))
    }
}

/// Requires the authenticated user to have at least `"moderator"` or `"admin"` role.
#[derive(Debug, Clone)]
pub struct ModeratorUser(pub user::Model);

impl FromRequestParts<AppState> for ModeratorUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user_model) = AuthUser::from_request_parts(parts, state).await?;

        if user_model.role != "moderator" && user_model.role != "admin" {
            return Err(AppError::Forbidden(
                "Moderator or admin role required.".to_string(),
            ));
        }

        Ok(Self(user_model))
    }
}

/// Requires the authenticated user to have the `"admin"` role.
#[derive(Debug, Clone)]
pub struct AdminUser(pub user::Model);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user_model) = AuthUser::from_request_parts(parts, state).await?;

        if user_model.role != "admin" {
            return Err(AppError::Forbidden("Admin role required.".to_string()));
        }

        Ok(Self(user_model))
    }
}
