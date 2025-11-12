//! Token blacklist models and utilities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::Entity;

/// Blacklisted token model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlacklistedToken {
    pub id: Uuid,
    pub jti: String,           // JWT ID
    pub token_type: String,    // "access" or "refresh"
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub blacklisted_at: DateTime<Utc>,
    pub reason: String,        // "logout", "revoke", "admin_action"
}

impl Entity for BlacklistedToken {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.blacklisted_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.blacklisted_at
    }
}

impl BlacklistedToken {
    /// Create a new blacklisted token
    pub async fn create(
        db: &sqlx::PgPool,
        jti: String,
        token_type: String,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
        reason: String,
    ) -> Result<Self, crate::error::AppError> {
        let token = sqlx::query_as::<_, BlacklistedToken>(
            r#"
            INSERT INTO blacklisted_tokens (jti, token_type, user_id, expires_at, blacklisted_at, reason)
            VALUES ($1, $2, $3, $4, NOW(), $5)
            RETURNING *
            "#
        )
        .bind(jti)
        .bind(token_type)
        .bind(user_id)
        .bind(expires_at)
        .bind(reason)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(token)
    }

    /// Check if a token is blacklisted by JTI
    pub async fn is_blacklisted(
        db: &sqlx::PgPool,
        jti: &str,
    ) -> Result<bool, crate::error::AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM blacklisted_tokens
            WHERE jti = $1 AND expires_at > NOW()
            "#
        )
        .bind(jti)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(count > 0)
    }

    /// Check if any tokens are blacklisted for a user
    pub async fn has_blacklisted_tokens(
        db: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<bool, crate::error::AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM blacklisted_tokens
            WHERE user_id = $1 AND expires_at > NOW()
            "#
        )
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(count > 0)
    }

    /// Blacklist all tokens for a user
    pub async fn blacklist_all_for_user(
        db: &sqlx::PgPool,
        user_id: Uuid,
        reason: String,
    ) -> Result<u64, crate::error::AppError> {
        let result = sqlx::query(
            r#"
            INSERT INTO blacklisted_tokens (jti, token_type, user_id, expires_at, blacklisted_at, reason)
            SELECT
                gen_random_uuid()::text as jti,
                'all_tokens' as token_type,
                $1 as user_id,
                NOW() + INTERVAL '7 days' as expires_at,
                NOW() as blacklisted_at,
                $2 as reason
            WHERE NOT EXISTS (
                SELECT 1 FROM blacklisted_tokens
                WHERE user_id = $1 AND token_type = 'all_tokens' AND expires_at > NOW()
            )
            "#
        )
        .bind(user_id)
        .bind(reason)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(result.rows_affected())
    }

    /// Clean up expired tokens (should be run periodically)
    pub async fn cleanup_expired(db: &sqlx::PgPool) -> Result<u64, crate::error::AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM blacklisted_tokens
            WHERE expires_at <= NOW()
            "#
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(result.rows_affected())
    }

    /// Get blacklisted tokens for a user
    pub async fn get_for_user(
        db: &sqlx::PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let tokens = sqlx::query_as::<_, BlacklistedToken>(
            r#"
            SELECT * FROM blacklisted_tokens
            WHERE user_id = $1 AND expires_at > NOW()
            ORDER BY blacklisted_at DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(tokens)
    }
}

/// Token blacklist service
pub struct TokenBlacklistService;

impl TokenBlacklistService {
    /// Check if a token should be rejected
    pub async fn should_reject_token(
        db: &sqlx::PgPool,
        jti: &str,
        user_id: Uuid,
    ) -> Result<bool, crate::error::AppError> {
        // Check if specific token is blacklisted
        if BlacklistedToken::is_blacklisted(db, jti).await? {
            return Ok(true);
        }

        // Check if all user tokens are blacklisted
        if BlacklistedToken::has_blacklisted_tokens(db, user_id).await? {
            return Ok(true);
        }

        Ok(false)
    }

    /// Add token to blacklist
    pub async fn blacklist_token(
        db: &sqlx::PgPool,
        jti: String,
        token_type: String,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
        reason: String,
    ) -> Result<(), crate::error::AppError> {
        BlacklistedToken::create(
            db,
            jti,
            token_type,
            user_id,
            expires_at,
            reason,
        ).await?;

        Ok(())
    }

    /// Blacklist all tokens for user (full logout)
    pub async fn blacklist_all_user_tokens(
        db: &sqlx::PgPool,
        user_id: Uuid,
        reason: String,
    ) -> Result<(), crate::error::AppError> {
        BlacklistedToken::blacklist_all_for_user(db, user_id, reason).await?;
        Ok(())
    }
}