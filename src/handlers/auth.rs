//! Authentication request handlers

use crate::error::AppError;
use crate::models::auth::{JwtService, PasswordUtils, TokenPair, PasswordResetRequest, EmailVerificationRequest};
use crate::models::user::{CreateUser, User, UserProfile, LoginRequest, LoginResponse};
use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// Application state for auth handlers
#[derive(Clone)]
pub struct AuthState {
    pub db_pool: sqlx::PgPool,
    pub jwt_service: JwtService,
}

/// Register a new user
pub async fn register(
    State(state): State<AuthState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate input
    if payload.username.len() < 3 {
        return Err(AppError::Validation(
            "Username must be at least 3 characters long".to_string(),
        ));
    }

    if !payload.email.contains('@') {
        return Err(AppError::Validation(
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
    };

    let user = User::create(&state.db_pool, create_user).await?;
    let user_profile = UserProfile::from(user.clone());

    // Generate tokens
    let token_pair = state.jwt_service.generate_token_pair(&user, vec![])?;

    // Create email verification request
    let _verification = EmailVerificationRequest::new(
        user.email.clone(),
        user.id,
        24, // 24 hours expiration
    );

    // TODO: Send verification email

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
    State(state): State<AuthState>,
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
    State(state): State<AuthState>,
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
    State(_state): State<AuthState>,
    Json(_payload): Json<LogoutRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Implement token blacklisting or refresh token invalidation
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    })))
}

/// Request password reset
pub async fn forgot_password(
    State(state): State<AuthState>,
    Json(payload): Json<PasswordResetEmailRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Find user by email
    let user = User::find_by_email(&state.db_pool, &payload.email)
        .await?;

    // Always return success to prevent email enumeration
    if let Some(user) = user {
        // Create password reset request
        let reset_request = PasswordResetRequest::new(user.email.clone(), 1); // 1 hour expiration

        // TODO: Save reset request to database
        // TODO: Send password reset email

        tracing::info!("Password reset requested for user: {}", user.email);
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "If an account with that email exists, a password reset link has been sent."
    })))
}

/// Confirm password reset
pub async fn reset_password(
    State(state): State<AuthState>,
    Json(payload): Json<PasswordResetConfirmRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate password strength
    PasswordUtils::validate_password_strength(&payload.new_password)?;

    // TODO: Verify reset token and get user
    // TODO: Update user password
    // TODO: Invalidate reset token

    // For now, this is a placeholder implementation
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Password reset successfully"
    })))
}

/// Verify email address
pub async fn verify_email(
    State(_state): State<AuthState>,
    Json(payload): Json<VerifyEmailRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Verify email verification token
    // TODO: Mark user email as verified

    // For now, this is a placeholder implementation
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Email verified successfully"
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