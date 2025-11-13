//! Authentication request handlers

use crate::error::AppError;
use crate::server::AppState;
use crate::models::auth::PasswordUtils;
use crate::models::user::{CreateUser, User, UserProfile, LoginRequest, LoginResponse, OidcLoginRequest, OidcCallbackRequest};
use axum::{
    extract::{State, Json, Query},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// User registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
}

/// Registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserProfile,
    pub message: String,
}

/// Password reset email request
#[derive(Debug, Deserialize)]
pub struct PasswordResetEmailRequest {
    pub email: String,
}

/// Password reset confirm request
#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

/// Email verification request
#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Logout request
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

/// Register a new user
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate input
    if payload.username.len() < 3 {
        return Err(AppError::BadRequest(
            "Username must be at least 3 characters long".to_string(),
        ));
    }

    if !payload.email.contains('@') {
        return Err(AppError::BadRequest(
            "Invalid email address".to_string(),
        ));
    }

    // Validate password strength
    PasswordUtils::validate_password_strength(&payload.password)?;

    // Check if username already exists
    if let Some(_) = User::find_by_username(&state.db_pool, &payload.username).await? {
        return Err(AppError::Conflict(
            "Username already exists".to_string(),
        ));
    }

    // Check if email already exists
    if let Some(_) = User::find_by_email(&state.db_pool, &payload.email).await? {
        return Err(AppError::Conflict(
            "Email already exists".to_string(),
        ));
    }

    // Create user
    let create_user = CreateUser {
        username: payload.username.clone(),
        email: payload.email.clone(),
        password: payload.password,
        display_name: payload.display_name,
        avatar_url: None,
    };

    let user = User::create(&state.db_pool, create_user).await?;
    let user_profile = UserProfile::from(user.clone());

    // Generate tokens
    let token_pair = state.jwt_service.generate_token_pair(&user, vec![])?;

    // Create email verification request
    let _verification = crate::models::email_verification::EmailVerificationService::create_verification(
        &state.db_pool,
        user.email.clone(),
        user.id,
    ).await?;

    // TODO: Send verification email with verification.token

    let response = RegisterResponse {
        user: user_profile,
        message: "User registered successfully. Please check your email for verification.".to_string(),
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": response,
            "tokens": token_pair
        })),
    ))
}

/// Login user
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Find user by email
    let user = User::find_by_email(&state.db_pool, &payload.email)
        .await?
        .ok_or_else(|| AppError::Authentication("Invalid credentials".to_string()))?;

    // Verify password
    if !user.verify_password(&payload.password) {
        return Err(AppError::Authentication("Invalid credentials".to_string()));
    }

    // Update last login
    user.update_last_login(&state.db_pool).await?;

    // Generate tokens
    let token_pair = state.jwt_service.generate_token_pair(&user, vec![])?;

    let response = LoginResponse {
        user: UserProfile::from(user),
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Refresh access token
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Verify refresh token
    let claims = state.jwt_service.verify_token(&payload.refresh_token)?;

    // Find user
    let user = User::find_by_id(&state.db_pool, Uuid::parse_str(&claims.sub).unwrap())
        .await?
        .ok_or_else(|| AppError::Authentication("User not found".to_string()))?;

    // Generate new token pair
    let token_pair = state.jwt_service.generate_token_pair(&user, vec![])?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "access_token": token_pair.access_token,
            "refresh_token": token_pair.refresh_token,
            "expires_in": token_pair.expires_in
        }
    })))
}

/// Logout user
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<impl IntoResponse, AppError> {
    use crate::models::token_blacklist::TokenBlacklistService;

    // Verify the refresh token
    let claims = state.jwt_service.verify_token(&payload.refresh_token)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    // Blacklist the refresh token
    let expires_at = chrono::DateTime::from_timestamp(claims.exp, 0)
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(24));

    TokenBlacklistService::blacklist_token(
        &state.db_pool,
        claims.jti,
        "refresh".to_string(),
        user_id,
        expires_at,
        "logout".to_string(),
    ).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    })))
}

/// Request password reset
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<PasswordResetEmailRequest>,
) -> Result<impl IntoResponse, AppError> {
    use crate::models::password_reset::PasswordResetService;

    // Create password reset request (returns None if user doesn't exist)
    let reset_request = PasswordResetService::request_reset(&state.db_pool, payload.email.clone()).await?;

    if let Some(reset_req) = reset_request {
        // TODO: Send password reset email with reset_req.token
        tracing::info!("Password reset requested for user: {}", reset_req.email);
        tracing::debug!("Reset token: {}", reset_req.token);
    }

    // Always return success to prevent email enumeration
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "If an account with that email exists, a password reset link has been sent."
    })))
}

/// Confirm password reset
pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<PasswordResetConfirmRequest>,
) -> Result<impl IntoResponse, AppError> {
    use crate::models::password_reset::PasswordResetService;

    // Confirm reset and update password
    PasswordResetService::confirm_reset(&state.db_pool, &payload.token, payload.new_password.clone()).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Password reset successfully"
    })))
}

/// Verify email address
pub async fn verify_email(
    State(state): State<AppState>,
    Json(payload): Json<VerifyEmailRequest>,
) -> Result<impl IntoResponse, AppError> {
    use crate::models::email_verification::EmailVerificationService;

    // Confirm email verification
    EmailVerificationService::confirm_verification(&state.db_pool, &payload.token).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Email verified successfully"
    })))
}

/// Get OIDC providers
pub async fn get_oidc_providers(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if !state.config.oidc.enabled {
        return Err(AppError::NotFound {
            entity: "OIDC".to_string(),
            id: "disabled".to_string(),
        });
    }

    let providers: Vec<_> = state.config.oidc.providers.iter()
        .map(|p| serde_json::json!({
            "name": p.name,
            "display_name": p.display_name,
        }))
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "enabled": true,
            "providers": providers
        }
    })))
}

/// Complete OIDC login flow using authware
pub async fn oidc_login(
    State(state): State<AppState>,
    Json(payload): Json<OidcLoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    if !state.config.oidc.enabled {
        return Err(AppError::NotFound {
            entity: "OIDC".to_string(),
            id: "disabled".to_string(),
        });
    }

    let _oidc_client = state.oidc_clients.get(&payload.provider)
        .ok_or_else(|| AppError::NotFound {
            entity: "OIDC Client".to_string(),
            id: payload.provider.clone(),
        })?;

    // TODO: Implement proper OIDC flow with authware
    // For now, return a placeholder implementation
    Ok(Json(serde_json::json!({
        "success": false,
        "message": "OIDC implementation pending authware crate integration"
    })))
}

/// OIDC callback handler using authware
pub async fn oidc_callback(
    State(_state): State<AppState>,
    _params: Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Implement proper OIDC callback handling with authware
    Ok(Json(serde_json::json!({
        "success": false,
        "message": "OIDC implementation pending authware crate integration"
    })))
}

/// OIDC callback handler (POST version for mobile apps)
pub async fn oidc_callback_post(
    State(_state): State<AppState>,
    _payload: Json<OidcCallbackRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Implement proper OIDC callback handling with authware
    Ok(Json(serde_json::json!({
        "success": false,
        "message": "OIDC implementation pending authware crate integration"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use sqlx::PgPool;

    // Note: These tests would require a test database setup
    // For now, they serve as examples of how to structure the tests

    #[tokio::test]
    async fn test_register_validation() {
        let config = Config::load().unwrap();
        let jwt_service = JwtService::new(
            &config.jwt.secret,
            config.jwt.issuer.clone(),
            config.jwt.expiration as i64,
            config.jwt.refresh_expiration as i64,
        ).unwrap();

        let state = AuthState {
            db_pool: PgPool::connect("postgresql://test").await.unwrap(),
            jwt_service,
        };

        // Test invalid username
        let request = RegisterRequest {
            username: "ab".to_string(), // Too short
            email: "test@example.com".to_string(),
            password: "ValidPass1!".to_string(),
            display_name: "Test User".to_string(),
        };

        let result = register(State(state.clone()), Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_login_validation() {
        let config = Config::load().unwrap();
        let jwt_service = JwtService::new(
            &config.jwt.secret,
            config.jwt.issuer.clone(),
            config.jwt.expiration as i64,
            config.jwt.refresh_expiration as i64,
        ).unwrap();

        let state = AuthState {
            db_pool: PgPool::connect("postgresql://test").await.unwrap(),
            jwt_service,
        };

        // Test invalid email
        let request = LoginRequest {
            email: "invalid-email".to_string(),
            password: "password".to_string(),
        };

        let result = login(State(state), Json(request)).await;
        assert!(result.is_err());
    }
}
