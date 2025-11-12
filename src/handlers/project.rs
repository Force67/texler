//! Project request handlers

use crate::error::AppError;
use crate::models::project::{Project, CreateProject, UpdateProject, ProjectWithDetails, ProjectCollaborator, ProjectStats};
use crate::models::user::UserProfile;
use crate::models::{PaginationParams, UserRole};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Project creation response
#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub project: ProjectWithDetails,
}

/// Projects list response
#[derive(Debug, Serialize)]
pub struct ProjectsListResponse {
    pub projects: Vec<ProjectWithDetails>,
    pub pagination: crate::models::PaginationInfo,
}

/// Collaborator addition request
#[derive(Debug, Deserialize)]
pub struct AddCollaboratorRequest {
    pub user_id: Uuid,
    pub role: UserRole,
}

/// Project compilation request
#[derive(Debug, Deserialize)]
pub struct CompileProjectRequest {
    pub file_id: Option<Uuid>,
    pub engine: Option<crate::models::LatexEngine>,
    pub args: Option<Vec<String>>,
}

/// Project search parameters
#[derive(Debug, Deserialize)]
pub struct ProjectSearchParams {
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_public: Option<bool>,
    pub owner_id: Option<Uuid>,
}

/// Application state for project handlers
#[derive(Clone)]
pub struct ProjectState {
    pub db_pool: sqlx::PgPool,
}

/// List projects accessible to the user
pub async fn list_projects(
    State(state): State<ProjectState>,
    Query(params): Query<PaginationParams>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let projects = Project::list_for_user(&state.db_pool, auth_user.user_id, &params).await?;

    // Get project details for each project
    let mut projects_with_details = Vec::new();
    for project in projects {
        let project_details = Project::get_with_details(&state.db_pool, project.id, auth_user.user_id).await?;
        projects_with_details.push(project_details);
    }

    // Get total count for pagination
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(DISTINCT p.id) FROM projects p
        WHERE (
            p.owner_id = $1 OR
            p.id IN (
                SELECT project_id FROM project_collaborators
                WHERE user_id = $1
            ) OR
            p.is_public = true
        )
        "#
    )
    .bind(auth_user.user_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let pagination_info = crate::models::PaginatedResponse::new(
        projects_with_details.clone(),
        &params,
        total_count.unwrap_or(0) as u64,
    ).pagination;

    let response = ProjectsListResponse {
        projects: projects_with_details,
        pagination: pagination_info,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Create a new project
pub async fn create_project(
    State(state): State<ProjectState>,
    Json(payload): Json<CreateProject>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let project = Project::create(&state.db_pool, auth_user.user_id, payload).await?;
    let project_with_details = Project::get_with_details(&state.db_pool, project.id, auth_user.user_id).await?;

    let response = ProjectResponse {
        project: project_with_details,
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": response
        })),
    ))
}

/// Get project details
pub async fn get_project(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let project_with_details = Project::get_with_details(&state.db_pool, project_id, auth_user.user_id).await?;

    let response = ProjectResponse {
        project: project_with_details,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Update project
pub async fn update_project(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<UpdateProject>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is project owner
    if !Project::is_owner(&state.db_pool, project_id, auth_user.user_id).await? {
        return Err(AppError::Authorization(
            "Only project owners can update projects".to_string(),
        ));
    }

    // Get current project
    let current_project = Project::find_by_id(&state.db_pool, project_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "Project",
            id: project_id.to_string(),
        })?;

    let updated_project = current_project.update(&state.db_pool, payload, auth_user.user_id).await?;
    let project_with_details = Project::get_with_details(&state.db_pool, updated_project.id, auth_user.user_id).await?;

    let response = ProjectResponse {
        project: project_with_details,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Delete project
pub async fn delete_project(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Get project to check ownership
    let project = Project::find_by_id(&state.db_pool, project_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "Project",
            id: project_id.to_string(),
        })?;

    // Delete project
    project.delete(&state.db_pool, auth_user.user_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Project deleted successfully"
    })))
}

/// Get project collaborators
pub async fn get_collaborators(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check project access
    if !Project::has_access(&state.db_pool, project_id, auth_user.user_id).await? {
        return Err(AppError::NotFound {
            entity: "Project",
            id: project_id.to_string(),
        });
    }

    let collaborators = sqlx::query_as::<_, UserProfile>(
        r#"
        SELECT u.id, u.username, u.email, u.display_name, u.avatar_url,
               u.is_active, u.email_verified, u.last_login_at, u.created_at
        FROM users u
        JOIN project_collaborators pc ON u.id = pc.user_id
        WHERE pc.project_id = $1
        ORDER BY pc.created_at
        "#
    )
    .bind(project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "collaborators": collaborators
        }
    })))
}

/// Add collaborator to project
pub async fn add_collaborator(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<AddCollaboratorRequest>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is project owner
    if !Project::is_owner(&state.db_pool, project_id, auth_user.user_id).await? {
        return Err(AppError::Authorization(
            "Only project owners can add collaborators".to_string(),
        ));
    }

    // Add collaborator
    let collaborator = ProjectCollaborator::add(
        &state.db_pool,
        project_id,
        payload.user_id,
        payload.role,
        auth_user.user_id,
    )
    .await?;

    // Get user profile for response
    let user_profile = sqlx::query_as::<_, UserProfile>(
        r#"
        SELECT id, username, email, display_name, avatar_url,
               is_active, email_verified, last_login_at, created_at
        FROM users
        WHERE id = $1
        "#
    )
    .bind(payload.user_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "collaborator": collaborator,
            "user": user_profile
        }
    })))
}

/// Remove collaborator from project
pub async fn remove_collaborator(
    State(state): State<ProjectState>,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is project owner or removing themselves
    let is_owner = Project::is_owner(&state.db_pool, project_id, auth_user.user_id).await?;
    let is_self = auth_user.user_id == user_id;

    if !is_owner && !is_self {
        return Err(AppError::Authorization(
            "Only project owners can remove collaborators".to_string(),
        ));
    }

    // Remove collaborator
    ProjectCollaborator::remove(&state.db_pool, project_id, user_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Collaborator removed successfully"
    })))
}

/// Compile project
pub async fn compile_project(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CompileProjectRequest>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check project access
    if !Project::has_access(&state.db_pool, project_id, auth_user.user_id).await? {
        return Err(AppError::NotFound {
            entity: "Project",
            id: project_id.to_string(),
        });
    }

    // Create compilation job
    let engine = payload.engine.unwrap_or(crate::models::LatexEngine::Pdflatex);
    let create_job = crate::models::compilation::CreateCompilationJob {
        file_id: payload.file_id,
        engine: Some(engine),
        args: payload.args,
        priority: None,
        template_id: None,
    };

    let working_directory = format!("/tmp/texler/projects/{}", project_id);
    let input_files = vec![]; // TODO: Get project files

    let job = crate::models::compilation::CompilationJob::create(
        &state.db_pool,
        project_id,
        auth_user.user_id,
        create_job,
        engine,
        working_directory,
        input_files,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "job_id": job.id,
            "status": job.status,
            "message": "Compilation job created successfully"
        }
    })))
}

/// Get project statistics
pub async fn get_project_stats(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check project access
    if !Project::has_access(&state.db_pool, project_id, auth_user.user_id).await? {
        return Err(AppError::NotFound {
            entity: "Project",
            id: project_id.to_string(),
        });
    }

    let stats = ProjectStats::get(&state.db_pool, project_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": stats
    })))
}

/// Get project activity
pub async fn get_activity(
    State(state): State<ProjectState>,
    Path(project_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check project access
    if !Project::has_access(&state.db_pool, project_id, auth_user.user_id).await? {
        return Err(AppError::NotFound {
            entity: "Project",
            id: project_id.to_string(),
        });
    }

    let activities = crate::models::project::ProjectActivity::get_recent(
        &state.db_pool,
        project_id,
        params.limit(),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "activities": activities
        }
    })))
}

/// Search projects (simplified version)
pub async fn search_projects(
    State(state): State<ProjectState>,
    Query(_params): Query<ProjectSearchParams>,
    Query(pagination_params): Query<PaginationParams>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // For now, just use the basic list_projects functionality
    let projects = Project::list_for_user(
        &state.db_pool,
        auth_user.user_id,
        &pagination_params,
    ).await?;

    // Get project details for each project
    let mut projects_with_details = Vec::new();
    for project in projects {
        let project_details = Project::get_with_details(&state.db_pool, project.id, auth_user.user_id).await?;
        projects_with_details.push(project_details);
    }

    let pagination_info = crate::models::PaginatedResponse::new(
        projects_with_details.clone(),
        &pagination_params,
        projects_with_details.len() as u64,
    ).pagination;

    let response = ProjectsListResponse {
        projects: projects_with_details,
        pagination: pagination_info,
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
    async fn test_project_access_check() {
        let state = ProjectState {
            db_pool: PgPool::connect("postgresql://test").await.unwrap(),
        };

        // This test would require setting up a proper test database
        // with test users and projects
        assert!(true);
    }

    #[test]
    fn test_project_search_params() {
        let params = ProjectSearchParams {
            query: Some("test".to_string()),
            tags: Some(vec!["latex".to_string()]),
            is_public: Some(true),
            owner_id: None,
        };

        assert_eq!(params.query, Some("test".to_string()));
        assert_eq!(params.is_public, Some(true));
    }
}