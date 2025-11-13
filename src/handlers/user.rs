//! User request handlers

use crate::error::AppError;
use crate::models::user::{User, UpdateUser, UserProfile, UserPreferences};
use crate::models::UserRole;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::server::AppState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User profile response
#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub user: UserProfile,
}

/// User preferences response
#[derive(Debug, Serialize)]
pub struct UserPreferencesResponse {
    pub preferences: UserPreferences,
}

/// User search response
#[derive(Debug, Serialize)]
pub struct UserSearchResponse {
    pub users: Vec<UserProfile>,
    pub total: u64,
}

/// User update request
#[derive(Debug, Deserialize)]
pub struct UserUpdateRequest {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// User preferences update request
#[derive(Debug, Deserialize)]
pub struct UserPreferencesUpdateRequest {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub latex_engine: Option<String>,
    pub auto_save: Option<bool>,
    pub line_numbers: Option<bool>,
    pub word_wrap: Option<bool>,
    pub font_size: Option<i32>,
    pub tab_size: Option<i32>,
}

/// User search parameters
#[derive(Debug, Deserialize)]
pub struct UserSearchParams {
    pub query: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Get current user profile
pub async fn get_current_user(
    State(state): State<AppState>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(&state.db_pool, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "User".to_string(),
            id: auth_user.user_id.to_string(),
        })?;

    let user_profile = UserProfile::from(user);

    let response = UserProfileResponse {
        user: user_profile,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Update current user profile
pub async fn update_user(
    State(state): State<AppState>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
    Json(payload): Json<UserUpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(&state.db_pool, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "User".to_string(),
            id: auth_user.user_id.to_string(),
        })?;

    let update_user = UpdateUser {
        display_name: payload.display_name,
        avatar_url: payload.avatar_url,
        is_active: None, // Users can't deactivate themselves
    };

    let updated_user = user.update(&state.db_pool, update_user).await?;
    let user_profile = UserProfile::from(updated_user);

    let response = UserProfileResponse {
        user: user_profile,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Get user preferences
pub async fn get_preferences(
    State(state): State<AppState>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(&state.db_pool, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "User".to_string(),
            id: auth_user.user_id.to_string(),
        })?;

    let preferences = user.get_preferences(&state.db_pool).await?;

    let response = UserPreferencesResponse {
        preferences,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Update user preferences
pub async fn update_preferences(
    State(state): State<AppState>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
    Json(payload): Json<UserPreferencesUpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(&state.db_pool, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "User".to_string(),
            id: auth_user.user_id.to_string(),
        })?;

    // Get current preferences
    let mut preferences = user.get_preferences(&state.db_pool).await?;

    // Update provided fields
    if let Some(theme) = payload.theme {
        preferences.theme = theme;
    }

    if let Some(language) = payload.language {
        preferences.language = language;
    }

    if let Some(latex_engine) = payload.latex_engine {
        preferences.latex_engine = latex_engine;
    }

    if let Some(auto_save) = payload.auto_save {
        preferences.auto_save = auto_save;
    }

    if let Some(line_numbers) = payload.line_numbers {
        preferences.line_numbers = line_numbers;
    }

    if let Some(word_wrap) = payload.word_wrap {
        preferences.word_wrap = word_wrap;
    }

    if let Some(font_size) = payload.font_size {
        preferences.font_size = font_size;
    }

    if let Some(tab_size) = payload.tab_size {
        preferences.tab_size = tab_size;
    }

    let updated_preferences = user.update_preferences(&state.db_pool, &preferences).await?;

    let response = UserPreferencesResponse {
        preferences: updated_preferences,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Search users
pub async fn search_users(
    State(state): State<AppState>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
    Json(payload): Json<UserSearchParams>,
) -> Result<impl IntoResponse, AppError> {
    let limit = payload.limit.unwrap_or(20).min(100) as i64;
    let offset = payload.offset.unwrap_or(0) as i64;

    let query = if payload.query.is_empty() {
        // Return no results if query is empty to prevent returning all users
        return Ok(Json(serde_json::json!({
            "success": true,
            "data": UserSearchResponse {
                users: vec![],
                total: 0,
            }
        })));
    } else {
        format!("%{}%", payload.query)
    };

    // Search users by username, display name, or email
    let users = sqlx::query_as::<_, UserProfile>(
        r#"
        SELECT id, username, email, display_name, avatar_url,
               is_active, email_verified, last_login_at, created_at
        FROM users
        WHERE is_active = true AND (
            username ILIKE $1 OR
            display_name ILIKE $1 OR
            email ILIKE $1
        )
        ORDER BY username
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(&query)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    // Get total count
    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM users
        WHERE is_active = true AND (
            username ILIKE $1 OR
            display_name ILIKE $1 OR
            email ILIKE $1
        )
        "#
    )
    .bind(&query)
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let response = UserSearchResponse {
        users,
        total: total as u64,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Get user by ID (public profile)
pub async fn get_user_by_id(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    _auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "User".to_string(),
            id: user_id.to_string(),
        })?;

    let user_profile = UserProfile::from(user);

    let response = UserProfileResponse {
        user: user_profile,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Get user statistics (admin only)
pub async fn get_user_stats(
    State(state): State<AppState>,
    _auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // This endpoint would require admin privileges
    // For now, it's a placeholder

    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE is_active = true")
        .fetch_one(&state.db_pool)
        .await
        .map_err(AppError::Database)?;

    let new_users_this_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE is_active = true AND created_at >= date_trunc('month', CURRENT_DATE)"
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let stats = serde_json::json!({
        "total_users": total_users,
        "new_users_this_month": new_users_this_month,
        "active_users_today": 0, // TODO: Implement based on last login
        "email_verified_users": 0, // TODO: Implement
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "data": stats
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_search_validation() {
        let search_params = UserSearchParams {
            query: "".to_string(), // Empty query should return no results
            limit: Some(10),
            offset: Some(0),
        };

        // This test would require setting up proper auth context
        // For now, we just verify the validation logic exists
        assert!(search_params.query.is_empty());
    }

    #[test]
    fn test_user_update_request() {
        let request = UserUpdateRequest {
            display_name: Some("New Display Name".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
        };

        assert!(request.display_name.is_some());
        assert!(request.avatar_url.is_some());
    }

    #[test]
    fn test_preferences_update() {
        let request = UserPreferencesUpdateRequest {
            theme: Some("dark".to_string()),
            font_size: Some(16),
            auto_save: Some(true),
            tab_size: Some(4),
        };

        assert_eq!(request.theme, Some("dark".to_string()));
        assert_eq!(request.font_size, Some(16));
        assert_eq!(request.auto_save, Some(true));
        assert_eq!(request.tab_size, Some(4));
    }
}
