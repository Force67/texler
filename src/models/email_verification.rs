//! Email verification models and functionality

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::Entity;
use crate::error::AppError;

/// Email verification request
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailVerificationRequest {
    pub id: Uuid,
    pub token: String,
    pub email: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
}

impl Entity for EmailVerificationRequest {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.verified_at.unwrap_or(self.created_at)
    }
}

impl EmailVerificationRequest {
    /// Create a new email verification request
    pub async fn create(
        db: &sqlx::PgPool,
        email: String,
        user_id: Uuid,
        expiration_hours: i64,
    ) -> Result<Self, crate::error::AppError> {
        use crate::models::auth::PasswordUtils;

        let verification_request = sqlx::query_as::<_, EmailVerificationRequest>(
            r#"
            INSERT INTO email_verification_requests (token, email, user_id, expires_at, verified, created_at)
            VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour' * $4, false, NOW())
            RETURNING *
            "#
        )
        .bind(PasswordUtils::generate_verification_token())
        .bind(email)
        .bind(user_id)
        .bind(expiration_hours)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(verification_request)
    }

    /// Find a verification request by token
    pub async fn find_by_token(
        db: &sqlx::PgPool,
        token: &str,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let request = sqlx::query_as::<_, EmailVerificationRequest>(
            r#"
            SELECT * FROM email_verification_requests
            WHERE token = $1 AND verified = false AND expires_at > NOW()
            "#
        )
        .bind(token)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(request)
    }

    /// Mark a verification request as verified
    pub async fn mark_as_verified(
        &self,
        db: &sqlx::PgPool,
    ) -> Result<Self, crate::error::AppError> {
        let updated = sqlx::query_as::<_, EmailVerificationRequest>(
            r#"
            UPDATE email_verification_requests
            SET verified = true, verified_at = NOW()
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

    /// Check if verification request is valid
    pub fn is_valid(&self) -> bool {
        !self.verified && Utc::now() < self.expires_at
    }

    /// Clean up expired requests (should be run periodically)
    pub async fn cleanup_expired(db: &sqlx::PgPool) -> Result<u64, crate::error::AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM email_verification_requests
            WHERE (verified = true AND verified_at < NOW() - INTERVAL '24 hours')
            OR (expires_at <= NOW())
            "#
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(result.rows_affected())
    }

    /// Invalidate all existing verification requests for an email
    pub async fn invalidate_for_email(
        db: &sqlx::PgPool,
        email: &str,
    ) -> Result<u64, crate::error::AppError> {
        let result = sqlx::query(
            r#"
            UPDATE email_verification_requests
            SET verified = true, verified_at = NOW()
            WHERE email = $1 AND verified = false
            "#
        )
        .bind(email)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(result.rows_affected())
    }
}

/// Email verification service
pub struct EmailVerificationService;

impl EmailVerificationService {
    /// Create an email verification request
    pub async fn create_verification(
        db: &sqlx::PgPool,
        email: String,
        user_id: Uuid,
    ) -> Result<EmailVerificationRequest, crate::error::AppError> {
        // Invalidate any existing verification requests
        EmailVerificationRequest::invalidate_for_email(db, &email).await?;

        // Create new verification request
        EmailVerificationRequest::create(
            db,
            email,
            user_id,
            24, // 24 hours expiration
        ).await
    }

    /// Confirm an email verification
    pub async fn confirm_verification(
        db: &sqlx::PgPool,
        token: &str,
    ) -> Result<(), crate::error::AppError> {
        use crate::models::user::User;

        // Find valid verification request
        let verification_request = EmailVerificationRequest::find_by_token(db, token).await?
            .ok_or_else(|| AppError::BadRequest("Invalid or expired verification token".to_string()))?;

        // Find user
        let user = User::find_by_id(db, verification_request.user_id).await?
            .ok_or_else(|| AppError::BadRequest("User not found".to_string()))?;

        // Mark user email as verified
        sqlx::query(
            r#"
            UPDATE users
            SET email_verified = true, updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Mark verification request as verified
        verification_request.mark_as_verified(db).await?;

        Ok(())
    }

    /// Resend verification email
    pub async fn resend_verification(
        db: &sqlx::PgPool,
        email: String,
    ) -> Result<Option<EmailVerificationRequest>, crate::error::AppError> {
        use crate::models::user::User;

        // Find user by email
        if let Some(user) = User::find_by_email(db, &email).await? {
            if !user.email_verified {
                // Create new verification request
                let verification_request = Self::create_verification(db, email, user.id).await?;
                Ok(Some(verification_request))
            } else {
                // Email already verified
                Ok(None)
            }
        } else {
            // User doesn't exist
            Ok(None)
        }
    }
}