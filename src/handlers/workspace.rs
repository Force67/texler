//! Workspace and project management endpoints backed by PostgreSQL

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
    Extension,
};
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::auth::AuthContext;
use crate::models::file::{CreateFile, File};
use crate::models::project::{CreateProject, Project};
use crate::models::workspace::{
    FileUpsert,
    MainFileUpdate,
    NewWorkspace,
    NewWorkspaceProject,
    ProjectDetails as WorkspaceProjectDetails,
    ProjectFileDetails,
    Workspace,
    WorkspaceSummary,
};
use crate::models::ContentType;
use crate::server::AppState;

#[derive(Debug, Serialize)]
pub struct WorkspaceListResponse {
    pub workspaces: Vec<WorkspaceSummary>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceResponse {
    pub workspace: WorkspaceSummary,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub project: ProjectPayload,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub file: FileResponsePayload,
}

#[derive(Debug, Serialize)]
pub struct FileResponsePayload {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectPayload {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub main_file: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub files: HashMap<String, ProjectFilePayload>,
    pub file_count: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectFilePayload {
    pub path: String,
    pub content: String,
    pub is_main: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// List workspaces for the authenticated user (auto-seeding the default workspace)
pub async fn list_workspaces(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let mut workspaces = Workspace::list_for_user(&state.db_pool, auth_user.user_id).await?;

    if workspaces.is_empty() {
        let workspace = Workspace::ensure_default(&state.db_pool, auth_user.user_id).await?;
        Workspace::seed_welcome_project(&state.db_pool, auth_user.user_id, workspace.id).await?;
        workspaces = Workspace::list_for_user(&state.db_pool, auth_user.user_id).await?;
    }

    Ok(Json(WorkspaceListResponse { workspaces }))
}

/// Create a new workspace
pub async fn create_workspace(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthContext>,
    Json(payload): Json<NewWorkspace>,
) -> Result<impl IntoResponse, AppError> {
    let workspace = Workspace::create(&state.db_pool, auth_user.user_id, payload.name, payload.description).await?;
    Workspace::seed_welcome_project(&state.db_pool, auth_user.user_id, workspace.id).await?;

    let summary = Workspace::get_with_projects(&state.db_pool, workspace.id, auth_user.user_id).await?;

    Ok(Json(WorkspaceResponse { workspace: summary }))
}

/// Get workspace with nested projects
pub async fn get_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let workspace = Workspace::get_with_projects(&state.db_pool, workspace_id, auth_user.user_id).await?;
    Ok(Json(WorkspaceResponse { workspace }))
}

/// Create a project inside a workspace (with a starter main.tex)
pub async fn create_project(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthContext>,
    Json(payload): Json<NewWorkspaceProject>,
) -> Result<impl IntoResponse, AppError> {
    Workspace::find_by_id(&state.db_pool, workspace_id, auth_user.user_id).await?;

    let create_project = CreateProject {
        name: payload
            .name
            .unwrap_or_else(|| format!("Project {}", chrono::Utc::now().timestamp())),
        description: payload.description,
        is_public: Some(false),
        main_file_path: Some("main.tex".to_string()),
        latex_engine: None,
        output_format: None,
        custom_args: None,
        bibliography_path: None,
        tags: None,
        workspace_id: Some(workspace_id),
    };

    let project = Project::create(&state.db_pool, auth_user.user_id, create_project).await?;

    // Seed project with a blank main file if none exists yet
    File::create(
        &state.db_pool,
        project.id,
        CreateFile {
            name: "main.tex".to_string(),
            path: "main.tex".to_string(),
            content: Some("% Start writing LaTeX here".to_string()),
            content_type: Some(ContentType::Latex),
        },
        auth_user.user_id,
    )
    .await?;

    let details = Workspace::get_project_details(&state.db_pool, workspace_id, project.id, auth_user.user_id).await?;
    Ok(Json(ProjectResponse { project: into_payload(details) }))
}

/// Fetch project details and raw files
pub async fn get_project(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let details = Workspace::get_project_details(&state.db_pool, workspace_id, project_id, auth_user.user_id).await?;
    Ok(Json(ProjectResponse { project: into_payload(details) }))
}

/// Add a file to a project
pub async fn add_file(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthContext>,
    Json(payload): Json<FileUpsert>,
) -> Result<impl IntoResponse, AppError> {
    Workspace::get_project_details(&state.db_pool, workspace_id, project_id, auth_user.user_id).await?;

    File::create(
        &state.db_pool,
        project_id,
        CreateFile {
            name: payload.path.split('/').last().unwrap_or(&payload.path).to_string(),
            path: payload.path.clone(),
            content: Some(payload.content.clone()),
            content_type: Some(ContentType::Latex),
        },
        auth_user.user_id,
    )
    .await?;

    Ok(Json(FileResponse { file: FileResponsePayload { path: payload.path } }))
}

/// Update file contents
pub async fn update_file(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthContext>,
    Json(payload): Json<FileUpsert>,
) -> Result<impl IntoResponse, AppError> {
    Workspace::get_project_details(&state.db_pool, workspace_id, project_id, auth_user.user_id).await?;

    let file = File::find_by_path(&state.db_pool, project_id, &payload.path, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "File".to_string(),
            id: payload.path.clone(),
        })?;

    file.update_content(&state.db_pool, payload.content, auth_user.user_id).await?;

    Ok(Json(FileResponse { file: FileResponsePayload { path: payload.path } }))
}

/// Change the main compilation file for a project
pub async fn set_main_file(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthContext>,
    Json(payload): Json<MainFileUpdate>,
) -> Result<impl IntoResponse, AppError> {
    Workspace::get_project_details(&state.db_pool, workspace_id, project_id, auth_user.user_id).await?;

    let project = Project::set_main_file(&state.db_pool, project_id, auth_user.user_id, &payload.path).await?;
    let details = Workspace::get_project_details(&state.db_pool, workspace_id, project.id, auth_user.user_id).await?;

    Ok(Json(ProjectResponse { project: into_payload(details) }))
}

fn into_payload(details: WorkspaceProjectDetails) -> ProjectPayload {
    let files: HashMap<String, ProjectFilePayload> = details
        .files
        .into_iter()
        .map(|file| (file.path.clone(), into_file_payload(file)))
        .collect();

    ProjectPayload {
        id: details.id,
        workspace_id: details.workspace_id,
        name: details.name,
        description: details.description,
        main_file: details.main_file,
        created_at: details.created_at,
        updated_at: details.updated_at,
        file_count: files.len(),
        files,
    }
}

fn into_file_payload(file: ProjectFileDetails) -> ProjectFilePayload {
    ProjectFilePayload {
        path: file.path,
        content: file.content,
        is_main: file.is_main,
        updated_at: file.updated_at,
    }
}
