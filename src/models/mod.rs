//! Domain models for the Texler backend

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub mod user;
pub mod project;
pub mod file;
pub mod collaboration;
pub mod compilation;
pub mod auth;
pub mod token_blacklist;
pub mod password_reset;
pub mod email_verification;

/// Common trait for database entities
pub trait Entity {
    fn id(&self) -> Uuid;
    fn created_at(&self) -> DateTime<Utc>;
    fn updated_at(&self) -> DateTime<Utc>;
}

/// Pagination parameters
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

impl PaginationParams {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1)
    }

    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(20).min(100) // Max 100 per page
    }

    pub fn offset(&self) -> u32 {
        self.offset.unwrap_or_else(|| (self.page() - 1) * self.limit())
    }

    pub fn sort_by(&self) -> String {
        self.sort_by.clone().unwrap_or_else(|| "created_at".to_string())
    }

    pub fn sort_order(&self) -> SortOrder {
        self.sort_order.unwrap_or(SortOrder::Desc)
    }
}

/// Sort order enum
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Desc
    }
}

/// Paginated response
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, params: &PaginationParams, total: u64) -> Self {
        let total_pages = (total as f64 / params.limit() as f64).ceil() as u32;

        Self {
            data,
            pagination: PaginationInfo {
                page: params.page(),
                limit: params.limit(),
                total,
                total_pages,
                has_next: params.page() < total_pages,
                has_prev: params.page() > 1,
            },
        }
    }
}

/// Pagination information
#[derive(Debug, Clone, Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Search parameters
#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    pub user_id: Option<Uuid>,
    pub is_public: Option<bool>,
    pub content_type: Option<ContentType>,
}

/// Common response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorInfo>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String, code: Option<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorInfo { message, code }),
            timestamp: Utc::now(),
        }
    }
}

/// Error information
#[derive(Debug, Clone, Serialize)]
pub struct ErrorInfo {
    pub message: String,
    pub code: Option<String>,
}

/// Content type for files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum ContentType {
    #[serde(rename = "latex")]
    #[sqlx(rename = "latex")]
    Latex,
    #[serde(rename = "bibliography")]
    #[sqlx(rename = "bibliography")]
    Bibliography,
    #[serde(rename = "image")]
    #[sqlx(rename = "image")]
    Image,
    #[serde(rename = "other")]
    #[sqlx(rename = "other")]
    Other,
}

impl Default for ContentType {
    fn default() -> Self {
        Self::Latex
    }
}

/// File storage strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum StorageStrategy {
    #[serde(rename = "inline")]
    #[sqlx(rename = "inline")]
    Inline,
    #[serde(rename = "toast")]
    #[sqlx(rename = "toast")]
    Toast,
    #[serde(rename = "external")]
    #[sqlx(rename = "external")]
    External,
}

impl Default for StorageStrategy {
    fn default() -> Self {
        Self::Toast
    }
}

/// User role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum UserRole {
    #[serde(rename = "owner")]
    #[sqlx(rename = "owner")]
    Owner,
    #[serde(rename = "maintainer")]
    #[sqlx(rename = "maintainer")]
    Maintainer,
    #[serde(rename = "collaborator")]
    #[sqlx(rename = "collaborator")]
    Collaborator,
    #[serde(rename = "viewer")]
    #[sqlx(rename = "viewer")]
    Viewer,
}

/// Compilation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum CompilationStatus {
    #[serde(rename = "never")]
    #[sqlx(rename = "never")]
    Never,
    #[serde(rename = "pending")]
    #[sqlx(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    #[sqlx(rename = "running")]
    Running,
    #[serde(rename = "success")]
    #[sqlx(rename = "success")]
    Success,
    #[serde(rename = "error")]
    #[sqlx(rename = "error")]
    Error,
    #[serde(rename = "cancelled")]
    #[sqlx(rename = "cancelled")]
    Cancelled,
}

impl Default for CompilationStatus {
    fn default() -> Self {
        Self::Never
    }
}

/// LaTeX engine type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum LatexEngine {
    #[serde(rename = "pdflatex")]
    #[sqlx(rename = "pdflatex")]
    Pdflatex,
    #[serde(rename = "xelatex")]
    #[sqlx(rename = "xelatex")]
    Xelatex,
    #[serde(rename = "lualatex")]
    #[sqlx(rename = "lualatex")]
    Lualatex,
}

impl Default for LatexEngine {
    fn default() -> Self {
        Self::Pdflatex
    }
}