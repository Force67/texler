//! Error types and handling for the Texler backend

use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;
use validator::ValidationErrors;
use uuid::Uuid;

/// Custom error types for the application
#[derive(Error, Debug)]
pub enum AppError {
    /// Database-related errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Redis-related errors
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Authorization errors
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Server errors
    #[error("Server error: {0}")]
    Server(String),

    /// Storage errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Legacy auth variant (for compatibility)
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationErrors),

    /// Not found errors
    #[error("{entity} not found: {id}")]
    NotFound { entity: String, id: String },

    /// Conflict errors (e.g., duplicate entries)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// LaTeX compilation errors
    #[error("LaTeX compilation error: {0}")]
    Compilation(String),

    /// WebSocket errors
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// JWT errors
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    /// Bcrypt errors
    #[error("Bcrypt error: {0}")]
    Bcrypt(String),

    /// Rate limiting errors
    #[error("Rate limit exceeded")]
    RateLimit,

    /// Bad request errors
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Internal server errors
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Background job errors
    #[error("Job error: {0}")]
    Job(String),
}

/// Request ID for tracking
#[derive(Debug, Clone, Copy)]
pub struct RequestId(pub Uuid);

impl RequestId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AppError {
    /// Get the appropriate HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Authentication(_) | AppError::Auth(_) => StatusCode::UNAUTHORIZED,
            AppError::Authorization(_) => StatusCode::FORBIDDEN,
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Jwt(_) => StatusCode::UNAUTHORIZED,
            AppError::Database(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            AppError::Database(sqlx::Error::Database(
                sqlx::error::DatabaseError::UniqueViolation(_),
            )) => StatusCode::CONFLICT,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::Authentication(_) | AppError::Auth(_) => "AUTHENTICATION_ERROR",
            AppError::Authorization(_) => "AUTHORIZATION_ERROR",
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::RateLimit => "RATE_LIMIT_EXCEEDED",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Jwt(_) => "INVALID_TOKEN",
            AppError::Bcrypt(_) => "BCRYPT_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Redis(_) => "REDIS_ERROR",
            AppError::Compilation(_) => "COMPILATION_ERROR",
            AppError::WebSocket(_) => "WEBSOCKET_ERROR",
            AppError::Io(_) => "IO_ERROR",
            AppError::Json(_) => "JSON_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
            AppError::Config(_) => "CONFIGURATION_ERROR",
            AppError::Job(_) => "JOB_ERROR",
            AppError::Server(_) => "SERVER_ERROR",
            AppError::Storage(_) => "STORAGE_ERROR",
        }
    }

    /// Check if this error is an operational error (expected errors)
    pub fn is_operational(&self) -> bool {
        !matches!(self, AppError::Internal(_))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();

        // Create error response body
        let body = Json(json!({
            "success": false,
            "error": {
                "code": error_code,
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        }));

        // Add validation details if present
        let body = if let AppError::Validation(validation_errors) = &self {
            let details: HashMap<String, Vec<String>> = validation_errors
                .field_errors()
                .iter()
                .map(|(field, errors)| {
                    let messages: Vec<String> = errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect();
                    (field.clone(), messages)
                })
                .collect();

            let mut json_value = serde_json::to_value(&body).unwrap();
            if let Some(error_obj) = json_value.as_object_mut()
                .and_then(|obj| obj.get_mut("error"))
                .and_then(|error| error.as_object_mut())
            {
                error_obj.insert("details".to_string(), serde_json::Value::Object(
                    details
                        .into_iter()
                        .map(|(k, v)| (k, serde_json::Value::Array(v.into_iter().map(serde_json::Value::String).collect())))
                        .collect()
                ));
            }

            (status, Json(serde_json::from_value(json_value).unwrap())).into_response()
        } else {
            (status, body).into_response()
        };

        body
    }
}

/// Result type alias for the application
pub type Result<T> = std::result::Result<T, AppError>;

/// Error that can be converted into an AppError
pub trait IntoAppError<T> {
    fn into_app_error(self) -> Result<T>;
}

impl<T> IntoAppError<T> for Option<T> {
    fn into_app_error(self) -> Result<T> {
        self.ok_or_else(|| AppError::Internal("Expected Some(T), got None".to_string()))
    }
}

/// Convert bcrypt errors to AppError
impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        AppError::Bcrypt(err.to_string())
    }
}

/// Convert validation errors to AppError
impl From<JsonRejection> for AppError {
    fn from(err: JsonRejection) -> Self {
        match err {
            JsonRejection::JsonDataError(err) => {
                AppError::BadRequest(format!("Invalid JSON: {}", err))
            }
            JsonRejection::JsonSyntaxError(err) => {
                AppError::BadRequest(format!("JSON syntax error: {}", err))
            }
            JsonRejection::MissingJsonContentType(_) => {
                AppError::BadRequest("Missing JSON content type".to_string())
            }
            _ => AppError::BadRequest("Invalid request body".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let error = AppError::Auth("Invalid token");
        assert_eq!(error.error_code(), "AUTHENTICATION_ERROR");
        assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_not_found_error() {
        let error = AppError::NotFound {
            entity: "User".to_string(),
            id: "123".to_string(),
        };
        assert_eq!(error.error_code(), "NOT_FOUND");
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_operational_error() {
        let error = AppError::Auth("test");
        assert!(error.is_operational());

        let error = AppError::Internal("internal error");
        assert!(!error.is_operational());
    }
}