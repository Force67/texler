//! File-related models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::path::PathBuf;
use uuid::Uuid;

use super::{ContentType, Entity, StorageStrategy};

/// File model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct File {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub path: String,
    pub content_type: ContentType,
    pub storage_strategy: StorageStrategy,
    pub content_hash: Option<String>,
    pub size: i64,
    pub line_count: i32,
    pub word_count: i32,
    pub latex_metadata: Option<FileMetadata>,
    pub version: i32,
    pub checksum: Option<String>,
    pub is_main: bool,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub last_modified_by: Option<Uuid>,
    pub last_modified: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for File {
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

/// File metadata for LaTeX files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub citations: Vec<String>,
    pub references: Vec<String>,
    pub labels: Vec<String>,
    pub includes: Vec<String>,
    pub sections: Vec<SectionInfo>,
    pub figures: Vec<FigureInfo>,
    pub tables: Vec<TableInfo>,
    pub equations: Vec<EquationInfo>,
}

/// Section information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionInfo {
    pub title: String,
    pub level: i32,
    pub line_number: i32,
    pub label: Option<String>,
}

/// Figure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FigureInfo {
    pub caption: String,
    pub label: Option<String>,
    pub line_number: i32,
}

/// Table information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub caption: String,
    pub label: Option<String>,
    pub line_number: i32,
}

/// Equation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationInfo {
    pub label: Option<String>,
    pub line_number: i32,
}

/// File creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFile {
    pub name: String,
    pub path: String,
    pub content: Option<String>,
    pub content_type: Option<ContentType>,
}

/// File update request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFile {
    pub name: Option<String>,
    pub path: Option<String>,
    pub content: Option<String>,
    pub content_type: Option<ContentType>,
    pub is_main: Option<bool>,
}

/// File version for version history
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FileVersion {
    pub id: Uuid,
    pub file_id: Uuid,
    pub version: i32,
    pub content_hash: String,
    pub changes: Option<String>, // JSON diff
    pub change_summary: String,
    pub author_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// File with additional data
#[derive(Debug, Clone, Serialize)]
pub struct FileWithDetails {
    #[serde(flatten)]
    pub file: File,
    pub modified_by: Option<super::UserProfile>,
    pub versions: Vec<FileVersion>,
    pub url: Option<String>,
}

/// File search result
#[derive(Debug, Clone, Serialize)]
pub struct FileSearchResult {
    #[serde(flatten)]
    pub file: File,
    pub highlights: Vec<SearchHighlight>,
    pub relevance_score: f64,
}

/// Search highlight
#[derive(Debug, Clone, Serialize)]
pub struct SearchHighlight {
    pub file_path: String,
    pub line_number: i32,
    pub snippet: String,
    pub offset: i32,
    pub length: i32,
}

/// File tree structure
#[derive(Debug, Clone, Serialize)]
pub struct FileNode {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: i64,
    pub modified_at: DateTime<Utc>,
    pub children: Vec<FileNode>,
    pub level: i32,
}

impl File {
    /// Create a new file
    pub async fn create(
        db: &sqlx::PgPool,
        project_id: Uuid,
        create_file: CreateFile,
        created_by: Uuid,
    ) -> Result<Self, crate::error::AppError> {
        let content = create_file.content.unwrap_or_default();
        let content_type = create_file.content_type.unwrap_or_default();
        let content_hash = Some(calculate_content_hash(&content));
        let size = content.len() as i64;
        let line_count = content.lines().count() as i32;
        let word_count = content.split_whitespace().count() as i32;
        let latex_metadata = extract_latex_metadata(&content, content_type);

        let file = sqlx::query_as!(
            File,
            r#"
            INSERT INTO files (
                project_id, name, path, content_type, storage_strategy,
                content_hash, size, line_count, word_count, latex_metadata,
                version, checksum, is_main, is_deleted, created_by, last_modified,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, $6, false, false, $11, NOW(), NOW(), NOW())
            RETURNING *
            "#,
            project_id,
            create_file.name,
            create_file.path,
            content_type as ContentType,
            StorageStrategy::default(),
            content_hash,
            size,
            line_count,
            word_count,
            serde_json::to_value(latex_metadata).ok(),
            calculate_content_hash(&content),
            content_hash,
            create_file.path == "main.tex",
            created_by
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Log file creation
        super::models::project::ProjectActivity::log(
            db,
            project_id,
            created_by,
            "file_created",
            "file",
            Some(file.id),
            None,
        )
        .await?;

        Ok(file)
    }

    /// Find file by ID with access control
    pub async fn find_by_id(
        db: &sqlx::PgPool,
        file_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let file = sqlx::query_as!(
            File,
            r#"
            SELECT f.* FROM files f
            JOIN projects p ON f.project_id = p.id
            WHERE f.id = $1 AND f.is_deleted = false AND (
                p.owner_id = $2 OR
                p.id IN (
                    SELECT project_id FROM project_collaborators
                    WHERE user_id = $2
                ) OR
                p.is_public = true
            )
            "#,
            file_id,
            user_id
        )
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(file)
    }

    /// Find file by path in project
    pub async fn find_by_path(
        db: &sqlx::PgPool,
        project_id: Uuid,
        path: &str,
        user_id: Uuid,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let file = sqlx::query_as!(
            File,
            r#"
            SELECT f.* FROM files f
            JOIN projects p ON f.project_id = p.id
            WHERE f.project_id = $1 AND f.path = $2 AND f.is_deleted = false AND (
                p.owner_id = $3 OR
                p.id IN (
                    SELECT project_id FROM project_collaborators
                    WHERE user_id = $3
                ) OR
                p.is_public = true
            )
            "#,
            project_id,
            path,
            user_id
        )
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(file)
    }

    /// List files in project
    pub async fn list_for_project(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
        params: &super::PaginationParams,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let files = sqlx::query_as!(
            File,
            r#"
            SELECT f.* FROM files f
            JOIN projects p ON f.project_id = p.id
            WHERE f.project_id = $1 AND f.is_deleted = false AND (
                p.owner_id = $2 OR
                p.id IN (
                    SELECT project_id FROM project_collaborators
                    WHERE user_id = $2
                ) OR
                p.is_public = true
            )
            ORDER BY f.path
            LIMIT $3 OFFSET $4
            "#,
            project_id,
            user_id,
            params.limit() as i64,
            params.offset() as i64
        )
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(files)
    }

    /// Update file content
    pub async fn update_content(
        &self,
        db: &sqlx::PgPool,
        content: String,
        modified_by: Uuid,
    ) -> Result<Self, crate::error::AppError> {
        let content_hash = Some(calculate_content_hash(&content));
        let size = content.len() as i64;
        let line_count = content.lines().count() as i32;
        let word_count = content.split_whitespace().count() as i32;
        let latex_metadata = extract_latex_metadata(&content, self.content_type);

        let file = sqlx::query_as!(
            File,
            r#"
            UPDATE files SET
                content_hash = $1,
                size = $2,
                line_count = $3,
                word_count = $4,
                latex_metadata = $5,
                version = version + 1,
                checksum = $1,
                last_modified_by = $6,
                last_modified = NOW(),
                updated_at = NOW()
            WHERE id = $7
            RETURNING *
            "#,
            content_hash,
            size,
            line_count,
            word_count,
            serde_json::to_value(latex_metadata).ok(),
            modified_by,
            self.id
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(file)
    }

    /// Soft delete file
    pub async fn soft_delete(
        &self,
        db: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE files SET is_deleted = true, deleted_at = NOW() WHERE id = $1",
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Log file deletion
        super::models::project::ProjectActivity::log(
            db,
            self.project_id,
            user_id,
            "file_deleted",
            "file",
            Some(self.id),
            None,
        )
        .await?;

        Ok(())
    }

    /// Restore soft-deleted file
    pub async fn restore(
        &self,
        db: &sqlx::PgPool,
    ) -> Result<Self, crate::error::AppError> {
        let file = sqlx::query_as!(
            File,
            "UPDATE files SET is_deleted = false, deleted_at = NULL WHERE id = $1 RETURNING *",
            self.id
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(file)
    }

    /// Get file with full details
    pub async fn get_with_details(
        db: &sqlx::PgPool,
        file_id: Uuid,
        user_id: Uuid,
    ) -> Result<FileWithDetails, crate::error::AppError> {
        // Get basic file info with access control
        let file = Self::find_by_id(db, file_id, user_id)?
            .ok_or_else(|| crate::error::AppError::NotFound {
                entity: "File",
                id: file_id.to_string(),
            })?;

        // Get modified by user info
        let modified_by = if let Some(user_id) = file.last_modified_by {
            Some(
                sqlx::query_as!(
                    super::UserProfile,
                    r#"
                    SELECT id, username, email, display_name, avatar_url,
                           is_active, email_verified, last_login_at, created_at
                    FROM users
                    WHERE id = $1
                    "#,
                    user_id
                )
                .fetch_one(db)
                .await
                .map_err(crate::error::AppError::Database)?,
            )
        } else {
            None
        };

        // Get file versions
        let versions = sqlx::query_as!(
            FileVersion,
            "SELECT * FROM file_versions WHERE file_id = $1 ORDER BY created_at DESC",
            file_id
        )
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(FileWithDetails {
            file,
            modified_by,
            versions,
            url: None, // TODO: Implement URL generation for stored files
        })
    }

    /// Build file tree structure
    pub async fn build_tree(files: &[Self]) -> Vec<FileNode> {
        let mut tree: Vec<FileNode> = Vec::new();

        // Sort files by path
        let mut sorted_files = files.to_vec();
        sorted_files.sort_by(|a, b| a.path.cmp(&b.path));

        for file in sorted_files {
            let path_parts: Vec<&str> = file.path.split('/').collect();
            let mut current_level = &mut tree;

            // Navigate/create directory structure
            for (i, part) in path_parts.iter().enumerate() {
                let is_last = i == path_parts.len() - 1;

                if is_last {
                    // This is the file
                    let node = FileNode {
                        id: file.id,
                        name: file.name.clone(),
                        path: file.path.clone(),
                        is_directory: false,
                        size: file.size,
                        modified_at: file.last_modified,
                        children: Vec::new(),
                        level: i as i32,
                    };
                    current_level.push(node);
                } else {
                    // This is a directory component
                    let dir_name = part.to_string();
                    let mut found_dir = false;

                    // Check if directory already exists
                    for node in current_level.iter_mut() {
                        if node.is_directory && node.name == dir_name {
                            current_level = &mut node.children;
                            found_dir = true;
                            break;
                        }
                    }

                    if !found_dir {
                        // Create new directory node
                        let node = FileNode {
                            id: Uuid::new_v4(), // Temporary ID for directories
                            name: dir_name.clone(),
                            path: path_parts[..i + 1].join("/"),
                            is_directory: true,
                            size: 0,
                            modified_at: Utc::now(),
                            children: Vec::new(),
                            level: i as i32,
                        };
                        current_level.push(node);
                        current_level = &mut node.children;
                    }
                }
            }
        }

        tree
    }
}

impl FileVersion {
    /// Create new version
    pub async fn create(
        db: &sqlx::PgPool,
        file_id: Uuid,
        version: i32,
        content: &str,
        author_id: Uuid,
        message: &str,
    ) -> Result<Self, crate::error::AppError> {
        let content_hash = calculate_content_hash(content);
        let changes = None; // TODO: Calculate diff from previous version

        let file_version = sqlx::query_as!(
            FileVersion,
            r#"
            INSERT INTO file_versions (file_id, version, content_hash, changes, change_summary, author_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            file_id,
            version,
            content_hash,
            changes,
            message,
            author_id
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(file_version)
    }

    /// Get version history for file
    pub async fn get_history(
        db: &sqlx::PgPool,
        file_id: Uuid,
        limit: u32,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let versions = sqlx::query_as!(
            FileVersion,
            "SELECT * FROM file_versions WHERE file_id = $1 ORDER BY created_at DESC LIMIT $2",
            file_id,
            limit as i64
        )
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(versions)
    }
}

/// Calculate content hash using SHA-256
fn calculate_content_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract LaTeX metadata from content
fn extract_latex_metadata(content: &str, content_type: ContentType) -> Option<FileMetadata> {
    if content_type != ContentType::Latex {
        return None;
    }

    let mut metadata = FileMetadata {
        citations: Vec::new(),
        references: Vec::new(),
        labels: Vec::new(),
        includes: Vec::new(),
        sections: Vec::new(),
        figures: Vec::new(),
        tables: Vec::new(),
        equations: Vec::new(),
    };

    // Extract citations
    let citation_regex = regex::Regex::new(r"\\cite\{([^}]+)\}").unwrap();
    for cap in citation_regex.captures_iter() {
        let citation = &cap[1];
        let cites: Vec<&str> = citation.split(',').map(|s| s.trim()).collect();
        metadata.citations.extend(cites);
    }

    // Extract references
    let ref_regex = regex::Regex::new(r"\\ref\{([^}]+)\}").unwrap();
    for cap in ref_regex.captures_iter() {
        metadata.references.push(cap[1].to_string());
    }

    // Extract labels
    let label_regex = regex::Regex::new(r"\\label\{([^}]+)\}").unwrap();
    for cap in label_regex.captures_iter() {
        metadata.labels.push(cap[1].to_string());
    }

    // Extract includes
    let include_regex = regex::Regex::new(r"\\(input|include)\{([^}]+)\}").unwrap();
    for cap in include_regex.captures_iter() {
        let include = &cap[2];
        let path = if !include.ends_with(".tex") {
            format!("{}.tex", include)
        } else {
            include.to_string()
        };
        metadata.includes.push(path);
    }

    // Extract sections
    let section_regex = regex::Regex::new(r"\\(section|subsection|subsubsection|paragraph|subparagraph)\*?\{([^}]+)\}").unwrap();
    let mut line_number = 1;
    for line in content.lines() {
        if let Some(cap) = section_regex.captures(line) {
            let section_type = &cap[1];
            let section_title = &cap[2];

            metadata.sections.push(SectionInfo {
                title: section_title.to_string(),
                level: match section_type {
                    "section" => 1,
                    "subsection" => 2,
                    "subsubsection" => 3,
                    "paragraph" => 4,
                    "subparagraph" => 5,
                    _ => 1,
                },
                line_number,
                label: None,
            });
        }
        line_number += 1;
    }

    Some(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[test]
    fn test_content_hash() {
        let content = "Hello, World!";
        let hash1 = calculate_content_hash(content);
        let hash2 = calculate_content_hash(content);
        assert_eq!(hash1, hash2);

        let different_content = "Different content";
        let hash3 = calculate_content_hash(different_content);
        assert_neq!(hash1, hash3);
    }

    #[test]
    fn test_latex_metadata_extraction() {
        let content = r#"
\documentclass{article}
\title{Test Document}
\author{Author Name}

\section{Introduction}
This is a test document with \cite{author2023} and references to \ref{fig:test}.

\begin{figure}
\caption{Test Figure}
\label{fig:test}
\end{figure}

\begin{equation}
E = mc^2
\label{eq:einstein}
\end{equation}
        "#;

        let metadata = extract_latex_metadata(content, ContentType::Latex).unwrap();

        assert_eq!(metadata.citations, vec!["author2023"]);
        assert_eq!(metadata.references, vec!["fig:test"]);
        assert_eq!(metadata.labels, vec!["fig:test", "eq:einstein"]);
        assert_eq!(metadata.sections.len(), 1);
        assert_eq!(metadata.sections[0].title, "Introduction");
        assert_eq!(metadata.sections[0].level, 1);
    }
}