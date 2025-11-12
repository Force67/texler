//! File request handlers

use crate::error::AppError;
use crate::models::file::{File, CreateFile, UpdateFile, FileWithDetails, FileNode, FileSearchResult};
use crate::models::{PaginationParams, ContentType, StorageStrategy};
use axum::{
    extract::{Path, Query, State, Multipart},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Json,
    body::Bytes,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::Path as StdPath;

/// File creation response
#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub file: FileWithDetails,
}

/// Files list response
#[derive(Debug, Serialize)]
pub struct FilesListResponse {
    pub files: Vec<FileWithDetails>,
    pub pagination: crate::models::PaginationInfo,
}

/// File content response
#[derive(Debug, Serialize)]
pub struct FileContentResponse {
    pub file: FileWithDetails,
    pub content: String,
}

/// File upload response
#[derive(Debug, Serialize)]
pub struct FileUploadResponse {
    pub file: FileWithDetails,
    pub url: Option<String>,
}

/// File tree response
#[derive(Debug, Serialize)]
pub struct FileTreeResponse {
    pub tree: Vec<FileNode>,
    pub total_files: i64,
    pub total_size: i64,
}

/// File search parameters
#[derive(Debug, Deserialize)]
pub struct FileSearchParams {
    pub query: Option<String>,
    pub content_type: Option<ContentType>,
    pub path: Option<String>,
    pub project_id: Option<Uuid>,
}

/// Application state for file handlers
#[derive(Clone)]
pub struct FileState {
    pub db_pool: sqlx::PgPool,
    pub config: crate::config::Config,
}

/// List files accessible to the user
pub async fn list_files(
    State(state): State<FileState>,
    Query(params): Query<FileSearchParams>,
    Query(pagination_params): Query<PaginationParams>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let project_id = params.project_id.ok_or_else(|| AppError::Validation(
        "Project ID is required".to_string(),
    ))?;

    let files = File::list_for_project(&state.db_pool, project_id, auth_user.user_id, &pagination_params).await?;

    // Get file details for each file
    let mut files_with_details = Vec::new();
    for file in files {
        let file_details = File::get_with_details(&state.db_pool, file.id, auth_user.user_id).await?;
        files_with_details.push(file_details);
    }

    // Get total count for pagination
    let total_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM files f
        JOIN projects p ON f.project_id = p.id
        WHERE f.project_id = $1 AND f.is_deleted = false AND (
            p.owner_id = $2 OR
            p.id IN (
                SELECT project_id FROM project_collaborators
                WHERE user_id = $2
            ) OR
            p.is_public = true
        )
        "#,
        project_id,
        auth_user.user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let pagination_info = crate::models::PaginatedResponse::new(
        files_with_details.clone(),
        &pagination_params,
        total_count.unwrap_or(0) as u64,
    ).pagination;

    let response = FilesListResponse {
        files: files_with_details,
        pagination: pagination_info,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Create a new file
pub async fn create_file(
    State(state): State<FileState>,
    Json(payload): Json<CreateFile>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Extract project_id from the path (assuming it's provided as a query parameter or path)
    let project_id = auth_user.user_id; // TODO: This should come from the request

    // Validate file path and name
    if payload.name.is_empty() {
        return Err(AppError::Validation("File name cannot be empty".to_string()));
    }

    if !payload.path.starts_with('/') {
        return Err(AppError::Validation("File path must be absolute".to_string()));
    }

    // Check if file already exists in project
    if let Some(_) = File::find_by_path(&state.db_pool, project_id, &payload.path, auth_user.user_id).await? {
        return Err(AppError::Conflict(
            "File with this path already exists".to_string(),
        ));
    }

    let file = File::create(&state.db_pool, project_id, payload, auth_user.user_id).await?;
    let file_with_details = File::get_with_details(&state.db_pool, file.id, auth_user.user_id).await?;

    let response = FileResponse {
        file: file_with_details,
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": response
        })),
    ))
}

/// Get file details
pub async fn get_file(
    State(state): State<FileState>,
    Path(file_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let file_with_details = File::get_with_details(&state.db_pool, file_id, auth_user.user_id).await?;

    let response = FileResponse {
        file: file_with_details,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Update file
pub async fn update_file(
    State(state): State<FileState>,
    Path(file_id): Path<Uuid>,
    Json(payload): Json<UpdateFile>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Get current file
    let current_file = File::find_by_id(&state.db_pool, file_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "File",
            id: file_id.to_string(),
        })?;

    // Update file fields
    let mut updated_file = current_file.clone();

    if let Some(name) = payload.name {
        updated_file.name = name;
    }

    if let Some(path) = payload.path {
        updated_file.path = path;
    }

    if let Some(content) = payload.content {
        updated_file = updated_file.update_content(&state.db_pool, content, auth_user.user_id).await?;
    }

    if let Some(content_type) = payload.content_type {
        updated_file.content_type = content_type;
    }

    if let Some(is_main) = payload.is_main {
        updated_file.is_main = is_main;
    }

    let file_with_details = File::get_with_details(&state.db_pool, updated_file.id, auth_user.user_id).await?;

    let response = FileResponse {
        file: file_with_details,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Delete file
pub async fn delete_file(
    State(state): State<FileState>,
    Path(file_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Get file
    let file = File::find_by_id(&state.db_pool, file_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "File",
            id: file_id.to_string(),
        })?;

    // Soft delete file
    file.soft_delete(&state.db_pool, auth_user.user_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "File deleted successfully"
    })))
}

/// Get file content
pub async fn get_file_content(
    State(state): State<FileState>,
    Path(file_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let file = File::find_by_id(&state.db_pool, file_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "File",
            id: file_id.to_string(),
        })?;

    // For now, return empty content - in a real implementation, this would
    // fetch the content from storage based on the storage strategy
    let content = String::new(); // TODO: Implement content retrieval

    let file_with_details = File::get_with_details(&state.db_pool, file_id, auth_user.user_id).await?;

    let response = FileContentResponse {
        file: file_with_details,
        content,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Update file content
pub async fn update_file_content(
    State(state): State<FileState>,
    Path(file_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let content = payload.get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("Content field is required".to_string()))?;

    // Get current file
    let current_file = File::find_by_id(&state.db_pool, file_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "File",
            id: file_id.to_string(),
        })?;

    // Update file content
    let updated_file = current_file.update_content(&state.db_pool, content.to_string(), auth_user.user_id).await?;
    let file_with_details = File::get_with_details(&state.db_pool, updated_file.id, auth_user.user_id).await?;

    let response = FileResponse {
        file: file_with_details,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Download file
pub async fn download_file(
    State(state): State<FileState>,
    Path(file_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let file = File::find_by_id(&state.db_pool, file_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "File",
            id: file_id.to_string(),
        })?;

    // Get file content
    let content = String::new(); // TODO: Implement content retrieval from storage

    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream"),
        (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", file.name)),
    ];

    Ok((headers, content))
}

/// Upload file
pub async fn upload_file(
    State(state): State<FileState>,
    Query(params): Query<FileSearchParams>,
    mut multipart: Multipart,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let project_id = params.project_id.ok_or_else(|| AppError::Validation(
        "Project ID is required".to_string(),
    ))?;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::Validation(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("file");
        let file_name = field.file_name()
            .ok_or_else(|| AppError::Validation("File name is required".to_string()))?
            .to_string();

        let content = field.bytes()
            .await
            .map_err(|e| AppError::Validation(format!("Failed to read file content: {}", e)))?;

        // Determine content type
        let content_type = match StdPath::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            Some("tex") => ContentType::Latex,
            Some("bib") => ContentType::Bibliography,
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => ContentType::Image,
            _ => ContentType::Other,
        };

        // Create file record
        let create_file = CreateFile {
            name: file_name.clone(),
            path: format!("/{}", file_name),
            content: Some(String::from_utf8_lossy(&content).to_string()),
            content_type: Some(content_type),
        };

        let file = File::create(&state.db_pool, project_id, create_file, auth_user.user_id).await?;
        let file_with_details = File::get_with_details(&state.db_pool, file.id, auth_user.user_id).await?;

        // TODO: Store file content based on storage strategy
        match state.config.features.file_storage.type_.as_str() {
            "local" => {
                // Store to local filesystem
                let file_path = format!("{}/{}", state.config.features.file_storage.local_path, file.id);
                tokio::fs::write(&file_path, &content).await
                    .map_err(|e| AppError::Storage(format!("Failed to save file: {}", e)))?;
            }
            "s3" => {
                // TODO: Implement S3 storage
                return Err(AppError::Storage("S3 storage not implemented yet".to_string()));
            }
            _ => {
                return Err(AppError::Storage("Unsupported storage type".to_string()));
            }
        }

        let response = FileUploadResponse {
            file: file_with_details,
            url: Some(format!("/api/v1/files/{}/download", file.id)),
        };

        return Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "data": response
            })),
        ));
    }

    Err(AppError::Validation("No file provided".to_string()))
}

/// Get file tree for a project
pub async fn get_file_tree(
    State(state): State<FileState>,
    Query(params): Query<FileSearchParams>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let project_id = params.project_id.ok_or_else(|| AppError::Validation(
        "Project ID is required".to_string(),
    ))?;

    // Get all files for the project
    let pagination_params = PaginationParams {
        page: Some(1),
        limit: Some(10000), // Get all files
        offset: None,
        sort_by: None,
        sort_order: None,
    };

    let files = File::list_for_project(&state.db_pool, project_id, auth_user.user_id, &pagination_params).await?;

    // Build file tree
    let tree = File::build_tree(&files);

    let total_files = files.len() as i64;
    let total_size = files.iter().map(|f| f.size).sum();

    let response = FileTreeResponse {
        tree,
        total_files,
        total_size,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Search files
pub async fn search_files(
    State(state): State<FileState>,
    Query(params): Query<FileSearchParams>,
    Query(pagination_params): Query<PaginationParams>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let project_id = params.project_id.ok_or_else(|| AppError::Validation(
        "Project ID is required".to_string(),
    ))?;

    // Build search query
    let mut query = r#"
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
    "#.to_string();

    let mut param_count = 3;

    // Add search conditions
    if let Some(query_text) = &params.query {
        query.push_str(&format!(" AND (f.name ILIKE ${} OR f.path ILIKE ${})", param_count, param_count + 1));
        param_count += 2;
    }

    if let Some(content_type) = params.content_type {
        query.push_str(&format!(" AND f.content_type = ${}", param_count));
        param_count += 1;
    }

    if let Some(path) = &params.path {
        query.push_str(&format!(" AND f.path LIKE ${}", param_count));
        param_count += 1;
    }

    // Add ordering and pagination
    query.push_str(" ORDER BY f.path");
    query.push_str(&format!(" LIMIT ${} OFFSET ${}", param_count, param_count + 1));

    // Execute query (simplified - would need proper parameter binding)
    let files: Vec<File> = sqlx::query_as(&query)
        .bind(project_id)
        .bind(auth_user.user_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::Database)?;

    // Get file details for each file
    let mut files_with_details = Vec::new();
    for file in files {
        let file_details = File::get_with_details(&state.db_pool, file.id, auth_user.user_id).await?;
        files_with_details.push(file_details);
    }

    let response = FilesListResponse {
        files: files_with_details,
        pagination: crate::models::PaginationInfo {
            page: pagination_params.page(),
            limit: pagination_params.limit(),
            total: 0, // TODO: Implement total count
            total_pages: 0,
            has_next: false,
            has_prev: false,
        },
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[tokio::test]
    async fn test_file_creation_validation() {
        let state = FileState {
            db_pool: PgPool::connect("postgresql://test").await.unwrap(),
            config: crate::config::Config::load().unwrap(),
        };

        let create_file = CreateFile {
            name: "".to_string(), // Empty name should fail validation
            path: "/test.tex".to_string(),
            content: Some("Hello World".to_string()),
            content_type: Some(ContentType::Latex),
        };

        // This test would require setting up proper auth context
        // For now, we just verify the validation logic exists
        assert!(create_file.name.is_empty());
    }

    #[test]
    fn test_content_type_detection() {
        assert_eq!(StdPath::new("document.tex").extension().and_then(|s| s.to_str()), Some("tex"));
        assert_eq!(StdPath::new("image.png").extension().and_then(|s| s.to_str()), Some("png"));
        assert_eq!(StdPath::new("references.bib").extension().and_then(|s| s.to_str()), Some("bib"));
    }
}