//! User-related models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::{Entity, UserRole};

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum AuthMethod {
    Password,
    Oidc,
}

/// OIDC user information from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,           // Subject (unique identifier)
    pub email: String,         // User email
    pub email_verified: bool,  // Email verification status
    pub name: Option<String>,  // Full name
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>, // Profile picture URL
    pub locale: Option<String>,
    pub preferred_username: Option<String>,
}

/// User model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>, // Optional for OIDC users
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub email_verified: bool,
    pub auth_method: AuthMethod,
    pub oidc_provider: Option<String>,
    pub oidc_provider_id: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: String::new(),
            email: String::new(),
            password_hash: None,
            display_name: String::new(),
            avatar_url: None,
            is_active: true,
            email_verified: false,
            auth_method: AuthMethod::Password,
            oidc_provider: None,
            oidc_provider_id: None,
            last_login_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Entity for User {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// User creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// OIDC user creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOidcUser {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub provider: String,
    pub provider_id: String,
}

/// User update request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUser {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: Option<bool>,
}

/// User profile response (without sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            is_active: user.is_active,
            email_verified: user.email_verified,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        }
    }
}

/// User preferences
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPreferences {
    pub user_id: Uuid,
    pub theme: String,
    pub language: String,
    pub latex_engine: String,
    pub auto_save: bool,
    pub line_numbers: bool,
    pub word_wrap: bool,
    pub font_size: i32,
    pub tab_size: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User session for JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub roles: Vec<UserRole>,
}

/// Password reset request
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

/// Password reset confirmation
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordResetConfirm {
    pub token: String,
    pub new_password: String,
}

/// Email verification request
#[derive(Debug, Clone, Deserialize)]
pub struct EmailVerificationRequest {
    pub email: String,
}

/// Login request
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// Refresh token request
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Clone, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
}

/// OIDC login request
#[derive(Debug, Clone, Deserialize)]
pub struct OidcLoginRequest {
    pub provider: String,
}

/// OIDC login URL response
#[derive(Debug, Clone, Serialize)]
pub struct OidcLoginUrlResponse {
    pub auth_url: String,
    pub state: String,
    pub pkce_challenge: Option<String>,
}

/// OIDC callback request
#[derive(Debug, Clone, Deserialize)]
pub struct OidcCallbackRequest {
    pub code: String,
    pub state: String,
    pub provider: String,
}

impl User {
    /// Create a new user with hashed password
    pub async fn create(
        db: &sqlx::PgPool,
        create_user: CreateUser,
    ) -> Result<Self, crate::error::AppError> {
        let password_hash = bcrypt::hash(&create_user.password, bcrypt::DEFAULT_COST)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, display_name, avatar_url, is_active, email_verified, auth_method)
            VALUES ($1, $2, $3, $4, $5, true, false, $6)
            RETURNING *
            "#
        )
        .bind(create_user.username)
        .bind(create_user.email)
        .bind(password_hash)
        .bind(create_user.display_name)
        .bind(create_user.avatar_url)
        .bind(AuthMethod::Password as AuthMethod)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(user)
    }

    /// Create a new OIDC user (no password)
    pub async fn create_oidc(
        db: &sqlx::PgPool,
        create_user: CreateOidcUser,
        email_verified: bool,
    ) -> Result<Self, crate::error::AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, display_name, avatar_url, is_active, email_verified, auth_method, oidc_provider, oidc_provider_id)
            VALUES ($1, $2, $3, $4, $5, true, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(create_user.username)
        .bind(create_user.email)
        .bind(None::<String>)
        .bind(create_user.display_name)
        .bind(create_user.avatar_url)
        .bind(email_verified)
        .bind(AuthMethod::Oidc as AuthMethod)
        .bind(create_user.provider)
        .bind(create_user.provider_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(user)
    }

    /// Find user by ID
    pub async fn find_by_id(
        db: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE id = $1 AND is_active = true
            "#
        )
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(user)
    }

    /// Find user by email
    pub async fn find_by_email(
        db: &sqlx::PgPool,
        email: &str,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE email = $1 AND is_active = true
            "#
        )
        .bind(email)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(user)
    }

    /// Find user by username
    pub async fn find_by_username(
        db: &sqlx::PgPool,
        username: &str,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE username = $1 AND is_active = true
            "#
        )
        .bind(username)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(user)
    }

    /// Find user by OIDC provider and provider ID
    pub async fn find_by_oidc(
        db: &sqlx::PgPool,
        provider: &str,
        provider_id: &str,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE oidc_provider = $1 AND oidc_provider_id = $2 AND is_active = true
            "#
        )
        .bind(provider)
        .bind(provider_id)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(user)
    }

    /// Find or create OIDC user from provider information
    pub async fn find_or_create_oidc(
        db: &sqlx::PgPool,
        user_info: &OidcUserInfo,
        provider: &str,
    ) -> Result<Self, crate::error::AppError> {
        // First, try to find existing user by OIDC info
        if let Some(user) = Self::find_by_oidc(db, provider, &user_info.sub).await? {
            return Ok(user);
        }

        // If not found, try to find by email (for account linking)
        if let Some(mut user) = Self::find_by_email(db, &user_info.email).await? {
            // Link existing account to OIDC provider
            user = sqlx::query_as::<_, User>(
                r#"
                UPDATE users
                SET
                    oidc_provider = $1,
                    oidc_provider_id = $2,
                    auth_method = $3,
                    email_verified = COALESCE($4, email_verified),
                    updated_at = NOW()
                WHERE id = $5
                RETURNING *
                "#
            )
            .bind(provider)
            .bind(user_info.sub.clone())
            .bind(AuthMethod::Oidc as AuthMethod)
            .bind(user_info.email_verified)
            .bind(user.id)
            .fetch_one(db)
            .await
            .map_err(crate::error::AppError::Database)?;
            return Ok(user);
        }

        // Create new user from OIDC information
        let username = user_info.preferred_username
            .as_ref()
            .unwrap_or(&user_info.email)
            .split('@')
            .next()
            .unwrap_or("user")
            .to_string();

        let create_user = CreateOidcUser {
            username: Self::generate_unique_username(db, &username).await?,
            email: user_info.email.clone(),
            display_name: user_info.name.as_ref()
                .unwrap_or(&user_info.email)
                .to_string(),
            avatar_url: user_info.picture.clone(),
            provider: provider.to_string(),
            provider_id: user_info.sub.clone(),
        };

        Self::create_oidc(db, create_user, user_info.email_verified).await
    }

    /// Generate a unique username by appending a number if needed
    async fn generate_unique_username(
        db: &sqlx::PgPool,
        base_username: &str,
    ) -> Result<String, crate::error::AppError> {
        let mut username = base_username.to_string();
        let mut suffix = 1;

        while Self::find_by_username(db, &username).await?.is_some() {
            username = format!("{}{}", base_username, suffix);
            suffix += 1;
        }

        Ok(username)
    }

    /// Verify user password
    pub fn verify_password(&self, password: &str) -> bool {
        match &self.password_hash {
            Some(hash) => bcrypt::verify(password, hash).unwrap_or(false),
            None => false, // OIDC users don't have passwords
        }
    }

    /// Update last login timestamp
    pub async fn update_last_login(
        &self,
        db: &sqlx::PgPool,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query(
            r#"
            UPDATE users
            SET last_login_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(self.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Update user information
    pub async fn update(
        &self,
        db: &sqlx::PgPool,
        update_user: UpdateUser,
    ) -> Result<Self, crate::error::AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET
                display_name = COALESCE($1, display_name),
                avatar_url = COALESCE($2, avatar_url),
                is_active = COALESCE($3, is_active),
                updated_at = NOW()
            WHERE id = $4
            RETURNING *
            "#
        )
        .bind(update_user.display_name)
        .bind(update_user.avatar_url)
        .bind(update_user.is_active)
        .bind(self.id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(user)
    }

    /// Delete user (soft delete)
    pub async fn delete(
        &self,
        db: &sqlx::PgPool,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query(
            r#"
            UPDATE users
            SET is_active = false, updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(self.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Get user preferences
    pub async fn get_preferences(
        &self,
        db: &sqlx::PgPool,
    ) -> Result<UserPreferences, crate::error::AppError> {
        let preferences = sqlx::query_as::<_, UserPreferences>(
            r#"
            SELECT * FROM user_preferences
            WHERE user_id = $1
            "#
        )
        .bind(self.id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(preferences)
    }

    /// Update user preferences
    pub async fn update_preferences(
        &self,
        db: &sqlx::PgPool,
        preferences: &UserPreferences,
    ) -> Result<UserPreferences, crate::error::AppError> {
        let updated = sqlx::query_as::<_, UserPreferences>(
            r#"
            INSERT INTO user_preferences (
                user_id, theme, language, latex_engine, auto_save,
                line_numbers, word_wrap, font_size, tab_size
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (user_id)
            DO UPDATE SET
                theme = EXCLUDED.theme,
                language = EXCLUDED.language,
                latex_engine = EXCLUDED.latex_engine,
                auto_save = EXCLUDED.auto_save,
                line_numbers = EXCLUDED.line_numbers,
                word_wrap = EXCLUDED.word_wrap,
                font_size = EXCLUDED.font_size,
                tab_size = EXCLUDED.tab_size,
                updated_at = NOW()
            RETURNING *
            "#
        )
        .bind(self.id)
        .bind(&preferences.theme)
        .bind(&preferences.language)
        .bind(&preferences.latex_engine)
        .bind(preferences.auto_save)
        .bind(preferences.line_numbers)
        .bind(preferences.word_wrap)
        .bind(preferences.font_size)
        .bind(preferences.tab_size)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(updated)
    }
}

impl UserPreferences {
    /// Get default preferences for a new user
    pub fn default(user_id: Uuid) -> Self {
        Self {
            user_id,
            theme: "dark".to_string(),
            language: "en".to_string(),
            latex_engine: "pdflatex".to_string(),
            auto_save: true,
            line_numbers: true,
            word_wrap: true,
            font_size: 14,
            tab_size: 2,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[tokio::test]
    async fn test_user_creation() {
        // This test would require a test database
        // It's just an example of how to structure tests
        let user_data = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            display_name: "Test User".to_string(),
            avatar_url: None,
        };

        // assert!(User::create(&pool, user_data).await.is_ok());
    }

    #[test]
    fn test_user_password_verification() {
        let password = "password123";
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();

        let user = User {
            id: Uuid::new_v4(),
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            password_hash: hash,
            display_name: "Test".to_string(),
            avatar_url: None,
            is_active: true,
            email_verified: false,
            last_login_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(user.verify_password(password));
        assert!(!user.verify_password("wrong"));
    }
}
