//! Authentication models and utilities

use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::UserRole;
use crate::error::AppError;
use crate::models::user::User;

/// JWT token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub username: String,
    pub email: String,
    pub roles: Vec<UserRole>,
    pub iat: i64, // Issued at
    pub exp: i64, // Expiration
    pub iss: String, // Issuer
    pub jti: String, // JWT ID for blacklisting
}

impl Claims {
    /// Create new claims for a user
    pub fn new(user: &User, roles: Vec<UserRole>, expiration: i64, issuer: String) -> Self {
        let now = Utc::now();
        Self {
            sub: user.id.to_string(),
            username: user.username.clone(),
            email: user.email.clone(),
            roles,
            iat: now.timestamp(),
            exp: now.timestamp() + expiration,
            iss: issuer,
            jti: PasswordUtils::generate_reset_token(), // Use as unique JWT ID
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// Authentication token pair
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// JWT token service
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: String,
    access_expiration: i64,
    refresh_expiration: i64,
}

impl JwtService {
    /// Create new JWT service
    pub fn new(
        secret: &str,
        issuer: String,
        access_expiration: i64,
        refresh_expiration: i64,
    ) -> Result<Self, AppError> {
        if secret.len() < 32 {
            return Err(AppError::Config(
                "JWT secret must be at least 32 characters long".to_string(),
            ));
        }

        let mut validation = Validation::default();
        validation.validate_exp = true;
        // Note: validate_iat is not available in current version
        validation.set_issuer(&[&issuer]);

        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
            issuer,
            access_expiration,
            refresh_expiration,
        })
    }

    /// Generate access token
    pub fn generate_access_token(&self, user: &User, roles: Vec<UserRole>) -> Result<String, AppError> {
        let claims = Claims::new(user, roles, self.access_expiration, self.issuer.clone());
        self.encode_token(&claims)
    }

    /// Generate refresh token
    pub fn generate_refresh_token(&self, user: &User) -> Result<String, AppError> {
        let claims = Claims::new(user, vec![], self.refresh_expiration, self.issuer.clone());
        self.encode_token(&claims)
    }

    /// Generate token pair
    pub fn generate_token_pair(&self, user: &User, roles: Vec<UserRole>) -> Result<TokenPair, AppError> {
        let access_token = self.generate_access_token(user, roles.clone())?;
        let refresh_token = self.generate_refresh_token(user)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            expires_in: self.access_expiration as u64,
        })
    }

    /// Verify and decode token (without database check - use verify_token_with_db for full validation)
    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| AppError::Authentication(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }

    /// Verify and decode token with blacklist check
    pub async fn verify_token_with_db(&self, token: &str, db: &sqlx::PgPool) -> Result<Claims, AppError> {
        let claims = self.verify_token(token)?;

        // Check if token is blacklisted
        use crate::models::token_blacklist::TokenBlacklistService;
        use uuid::Uuid;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

        if TokenBlacklistService::should_reject_token(db, &claims.jti, user_id).await? {
            return Err(AppError::Authentication("Token has been revoked".to_string()));
        }

        Ok(claims)
    }

    /// Refresh access token using refresh token
    pub fn refresh_access_token(
        &self,
        refresh_token: &str,
        user: &User,
        roles: Vec<UserRole>,
    ) -> Result<TokenPair, AppError> {
        // Verify refresh token
        let claims = self.verify_token(refresh_token)?;

        // Check if token belongs to the same user
        if claims.sub != user.id.to_string() {
            return Err(AppError::Authentication(
                "Refresh token does not belong to this user".to_string(),
            ));
        }

        // Check if refresh token is still valid
        if claims.is_expired() {
            return Err(AppError::Authentication(
                "Refresh token has expired".to_string(),
            ));
        }

        // Generate new token pair
        self.generate_token_pair(user, roles)
    }

    /// Encode token
    fn encode_token(&self, claims: &Claims) -> Result<String, AppError> {
        encode(&Header::default(), claims, &self.encoding_key)
            .map_err(|e| AppError::Authentication(format!("Failed to encode token: {}", e)))
    }
}

/// Password utilities
pub struct PasswordUtils;

impl PasswordUtils {
    /// Hash password with bcrypt
    pub fn hash_password(password: &str) -> Result<String, AppError> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::Authentication(format!("Failed to hash password: {}", e)))
    }

    /// Verify password against hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
        bcrypt::verify(password, hash)
            .map_err(|e| AppError::Authentication(format!("Failed to verify password: {}", e)))
    }

    /// Generate password reset token
    pub fn generate_reset_token() -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    }

    /// Generate email verification token
    pub fn generate_verification_token() -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    }

    /// Validate password strength
    pub fn validate_password_strength(password: &str) -> Result<(), AppError> {
        if password.len() < 8 {
            return Err(AppError::BadRequest(
                "Password must be at least 8 characters long".to_string(),
            ));
        }

        if password.len() > 128 {
            return Err(AppError::BadRequest(
                "Password must be less than 128 characters long".to_string(),
            ));
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

        if !has_uppercase {
            return Err(AppError::BadRequest(
                "Password must contain at least one uppercase letter".to_string(),
            ));
        }

        if !has_lowercase {
            return Err(AppError::BadRequest(
                "Password must contain at least one lowercase letter".to_string(),
            ));
        }

        if !has_digit {
            return Err(AppError::BadRequest(
                "Password must contain at least one digit".to_string(),
            ));
        }

        if !has_special {
            return Err(AppError::BadRequest(
                "Password must contain at least one special character".to_string(),
            ));
        }

        Ok(())
    }
}

/// Authentication context for requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub roles: Vec<UserRole>,
    pub token_issued_at: DateTime<Utc>,
    pub token_expires_at: DateTime<Utc>,
}

impl From<Claims> for AuthContext {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: Uuid::parse_str(&claims.sub).unwrap_or_else(|_| Uuid::new_v4()),
            username: claims.username,
            email: claims.email,
            roles: claims.roles,
            token_issued_at: DateTime::from_timestamp(claims.iat, 0).unwrap_or_else(|| Utc::now()),
            token_expires_at: DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(|| Utc::now()),
        }
    }
}

impl AuthContext {
    /// Check if user has specific role
    pub fn has_role(&self, role: UserRole) -> bool {
        self.roles.contains(&role)
    }

    /// Check if user is owner or maintainer
    pub fn is_owner_or_maintainer(&self) -> bool {
        self.has_role(UserRole::Owner) || self.has_role(UserRole::Maintainer)
    }

    /// Check if user can write
    pub fn can_write(&self) -> bool {
        self.has_role(UserRole::Owner) || self.has_role(UserRole::Maintainer) || self.has_role(UserRole::Collaborator)
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.token_expires_at
    }
}

/// Password reset request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub token: String,
    pub email: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

impl PasswordResetRequest {
    /// Create new password reset request
    pub fn new(email: String, expiration_hours: i64) -> Self {
        Self {
            token: PasswordUtils::generate_reset_token(),
            email,
            expires_at: Utc::now() + Duration::hours(expiration_hours),
            used: false,
            created_at: Utc::now(),
        }
    }

    /// Check if reset request is valid
    pub fn is_valid(&self) -> bool {
        !self.used && Utc::now() < self.expires_at
    }
}

/// Email verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerificationRequest {
    pub token: String,
    pub email: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}

impl EmailVerificationRequest {
    /// Create new email verification request
    pub fn new(email: String, user_id: Uuid, expiration_hours: i64) -> Self {
        Self {
            token: PasswordUtils::generate_verification_token(),
            email,
            user_id,
            expires_at: Utc::now() + Duration::hours(expiration_hours),
            verified: false,
            created_at: Utc::now(),
        }
    }

    /// Check if verification request is valid
    pub fn is_valid(&self) -> bool {
        !self.verified && Utc::now() < self.expires_at
    }
}

/// Authentication middleware state
pub struct AuthState {
    pub jwt_service: JwtService,
}

impl AuthState {
    pub fn new(jwt_service: JwtService) -> Self {
        Self { jwt_service }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::User;

    #[test]
    fn test_password_validation() {
        assert!(PasswordUtils::validate_password_strength("").is_err());
        assert!(PasswordUtils::validate_password_strength("weak").is_err());
        assert!(PasswordUtils::validate_password_strength("weakpass").is_err());
        assert!(PasswordUtils::validate_password_strength("Weakpass").is_err());
        assert!(PasswordUtils::validate_password_strength("Weakpass1").is_err());
        assert!(PasswordUtils::validate_password_strength("Weakpass1!").is_ok());
    }

    #[test]
    fn test_jwt_service_creation() {
        assert!(JwtService::new("short", "test".to_string(), 3600, 86400).is_err());
        assert!(JwtService::new("this_is_a_very_long_secret_key_32_chars", "test".to_string(), 3600, 86400).is_ok());
    }

    #[test]
    fn test_password_reset_request() {
        let reset_req = PasswordResetRequest::new("test@example.com".to_string(), 24);
        assert!(reset_req.is_valid());
        assert_eq!(reset_req.token.len(), 32);
    }

    #[test]
    fn test_email_verification_request() {
        let verify_req = EmailVerificationRequest::new(
            "test@example.com".to_string(),
            Uuid::new_v4(),
            24,
        );
        assert!(verify_req.is_valid());
        assert_eq!(verify_req.token.len(), 64);
    }
}
