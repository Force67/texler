//! Password reset models and functionality

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::Entity;
use crate::error::AppError;

/// Password reset request
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordResetRequest {
    pub id: Uuid,
    pub token: String,
    pub email: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl Entity for PasswordResetRequest {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.used_at.unwrap_or(self.created_at)
    }
}

impl PasswordResetRequest {
    /// Create a new password reset request
    pub async fn create(
        db: &sqlx::PgPool,
        email: String,
        user_id: Uuid,
        expiration_hours: i64,
    ) -> Result<Self, crate::error::AppError> {
        use crate::models::auth::PasswordUtils;

        let reset_request = sqlx::query_as::<_, PasswordResetRequest>(
            r#"
            INSERT INTO password_reset_requests (token, email, user_id, expires_at, used, created_at)
            VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour' * $4, false, NOW())
            RETURNING *
            "#
        )
        .bind(PasswordUtils::generate_reset_token())
        .bind(email)
        .bind(user_id)
        .bind(expiration_hours)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(reset_request)
    }

    /// Find a reset request by token
    pub async fn find_by_token(
        db: &sqlx::PgPool,
        token: &str,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let request = sqlx::query_as::<_, PasswordResetRequest>(
            r#"
            SELECT * FROM password_reset_requests
            WHERE token = $1 AND used = false AND expires_at > NOW()
            "#
        )
        .bind(token)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(request)
    }

    /// Mark a reset request as used
    pub async fn mark_as_used(
        &self,
        db: &sqlx::PgPool,
    ) -> Result<Self, crate::error::AppError> {
        let updated = sqlx::query_as::<_, PasswordResetRequest>(
            r#"
            UPDATE password_reset_requests
            SET used = true, used_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(self.id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(updated)
    }

    /// Check if reset request is valid
    pub fn is_valid(&self) -> bool {
        !self.used && Utc::now() < self.expires_at
    }

    /// Clean up expired requests (should be run periodically)
    pub async fn cleanup_expired(db: &sqlx::PgPool) -> Result<u64, crate::error::AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM password_reset_requests
            WHERE (used = true AND used_at < NOW() - INTERVAL '24 hours')
            OR (expires_at <= NOW())
            "#
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(result.rows_affected())
    }

    /// Invalidate all existing reset requests for an email
    pub async fn invalidate_for_email(
        db: &sqlx::PgPool,
        email: &str,
    ) -> Result<u64, crate::error::AppError> {
        let result = sqlx::query(
            r#"
            UPDATE password_reset_requests
            SET used = true, used_at = NOW()
            WHERE email = $1 AND used = false
            "#
        )
        .bind(email)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(result.rows_affected())
    }
}

/// Password reset service
pub struct PasswordResetService;

impl PasswordResetService {
    /// Request a password reset
    pub async fn request_reset(
        db: &sqlx::PgPool,
        email: String,
    ) -> Result<Option<PasswordResetRequest>, crate::error::AppError> {
        use crate::models::user::User;

        // Find user by email
        if let Some(user) = User::find_by_email(db, &email).await? {
            // Invalidate any existing reset requests
            PasswordResetRequest::invalidate_for_email(db, &email).await?;

            // Create new reset request
            let reset_request = PasswordResetRequest::create(
                db,
                email,
                user.id,
                1, // 1 hour expiration
            ).await?;

            Ok(Some(reset_request))
        } else {
            // User doesn't exist - return None to prevent email enumeration
            Ok(None)
        }
    }

    /// Confirm a password reset
    pub async fn confirm_reset(
        db: &sqlx::PgPool,
        token: &str,
        new_password: String,
    ) -> Result<(), crate::error::AppError> {
        use crate::models::user::User;
        use crate::models::auth::PasswordUtils;

        // Validate password strength
        PasswordUtils::validate_password_strength(&new_password)?;

        // Find valid reset request
        let reset_request = PasswordResetRequest::find_by_token(db, token).await?
            .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".to_string()))?;

        // Find user
        let user = User::find_by_id(db, reset_request.user_id).await?
            .ok_or_else(|| AppError::BadRequest("User not found".to_string()))?;

        // Hash new password
        let password_hash = PasswordUtils::hash_password(&new_password)?;

        // Update user password
        sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $1, updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(password_hash)
        .bind(user.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Mark reset request as used
        reset_request.mark_as_used(db).await?;

        Ok(())
    }
}