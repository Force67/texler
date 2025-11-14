//! Workspace persistence layer and helpers

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::error::AppError;

use super::file::{CreateFile, File};
use super::project::{CreateProject, Project};
use super::ContentType;

pub const DEFAULT_WORKSPACE_NAME: &str = "Personal Workspace";
pub const DEFAULT_WORKSPACE_DESCRIPTION: &str = "Sandbox workspace for your LaTeX experiments.";
pub const DEFAULT_PROJECT_NAME: &str = "Welcome Project";
pub const DEFAULT_PROJECT_DESCRIPTION: &str = "Starter project with sample LaTeX files.";

const DEFAULT_MAIN_TEX: &str = r"\\documentclass[12pt,a4paper]{article}

% Packages
\\usepackage[utf8]{inputenc}
\\usepackage[T1]{fontenc}
\\usepackage{amsmath,amssymb,amsfonts}
\\usepackage{graphicx}
\\usepackage{hyperref}
\\usepackage{geometry}

% Geometry
\\geometry{margin=1in}

% Title and author
\\title{Multi-File LaTeX Document}
\\author{Texler}
\\date{\\today}

\\begin{document}

\\maketitle

\\tableofcontents
\\newpage

% Include sections
\\include{sections/introduction}

% Add more sections here

\\end{document}";

const DEFAULT_INTRO_TEX: &str = r"\\section{Introduction}

This is the introduction section of your multi-file LaTeX document.

\\subsection{Background}

You can write your introduction content here. LaTeX automatically handles:

\\begin{itemize}
\\item Section numbering
\\item Cross-references
\\item Citations
\\item Mathematical equations
\\end{itemize}

\\subsection{Mathematical Example}

Here's some mathematics to test compilation:

\\begin{equation}
E = mc^2
\\end{equation}

\\begin{equation}
\\int_{0}^{\\infty} e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}
\\end{equation}";

/// Database representation of a workspace
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workspace summary with project metadata for API responses
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub project_count: usize,
    pub projects: Vec<ProjectSummary>,
}

/// Lightweight project summary returned alongside workspaces
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub main_file: Option<String>,
    pub file_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ProjectSummaryRow {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub main_file_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub file_count: Option<i64>,
}

/// Project details along with file metadata used by the frontend editor
#[derive(Debug, Clone, Serialize)]
pub struct ProjectDetails {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub main_file: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub files: Vec<ProjectFileDetails>,
}

/// File payload containing raw LaTeX content
#[derive(Debug, Clone, Serialize)]
pub struct ProjectFileDetails {
    pub id: Uuid,
    pub path: String,
    pub content: String,
    pub is_main: bool,
    pub updated_at: DateTime<Utc>,
}

/// Workspace creation payload
#[derive(Debug, Clone, Deserialize)]
pub struct NewWorkspace {
    pub name: String,
    pub description: Option<String>,
}

/// Workspace-level project creation payload
#[derive(Debug, Clone, Deserialize)]
pub struct NewWorkspaceProject {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// File upsert payload used by workspace endpoints
#[derive(Debug, Clone, Deserialize)]
pub struct FileUpsert {
    pub path: String,
    pub content: String,
}

/// Request body for changing project main file
#[derive(Debug, Clone, Deserialize)]
pub struct MainFileUpdate {
    pub path: String,
}

impl Workspace {
    /// List all workspaces owned by the user along with their projects
    pub async fn list_for_user(
        db: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<Vec<WorkspaceSummary>, AppError> {
        let workspaces = sqlx::query_as::<_, Workspace>(
            r#"
            SELECT * FROM workspaces
            WHERE owner_id = $1
            ORDER BY created_at
            "#
        )
        .bind(user_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)?;

        let workspace_ids: Vec<Uuid> = workspaces.iter().map(|w| w.id).collect();

        let mut projects_by_workspace: std::collections::HashMap<Uuid, Vec<ProjectSummary>> =
            std::collections::HashMap::new();

        if !workspace_ids.is_empty() {
            let project_rows = sqlx::query_as::<_, ProjectSummaryRow>(
                r#"
                SELECT
                    p.id,
                    p.workspace_id,
                    p.name,
                    p.description,
                    p.main_file_path,
                    p.created_at,
                    p.updated_at,
                    COALESCE(f.file_count, 0) AS file_count
                FROM projects p
                LEFT JOIN (
                    SELECT project_id, COUNT(*)::BIGINT AS file_count
                    FROM files
                    WHERE is_deleted = false
                    GROUP BY project_id
                ) f ON f.project_id = p.id
                WHERE p.workspace_id = ANY($1)
                ORDER BY p.created_at
                "#
            )
            .bind(&workspace_ids)
            .fetch_all(db)
            .await
            .map_err(AppError::Database)?;

            for row in project_rows {
                let summary = ProjectSummary {
                    id: row.id,
                    workspace_id: row.workspace_id,
                    name: row.name,
                    description: row.description,
                    main_file: row.main_file_path,
                    file_count: row.file_count.unwrap_or(0),
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                projects_by_workspace
                    .entry(row.workspace_id)
                    .or_default()
                    .push(summary);
            }
        }

        let summaries = workspaces
            .into_iter()
            .map(|workspace| {
                let projects = projects_by_workspace
                    .remove(&workspace.id)
                    .unwrap_or_default();
                WorkspaceSummary {
                    id: workspace.id,
                    name: workspace.name,
                    description: workspace.description,
                    owner_id: workspace.owner_id,
                    created_at: workspace.created_at,
                    updated_at: workspace.updated_at,
                    project_count: projects.len(),
                    projects,
                }
            })
            .collect();

        Ok(summaries)
    }

    /// Create a new workspace for the owner
    pub async fn create(
        db: &sqlx::PgPool,
        owner_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Self, AppError> {
        let trimmed = normalize_name(&name)?;
        let workspace = sqlx::query_as::<_, Workspace>(
            r#"
            INSERT INTO workspaces (name, description, owner_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#
        )
        .bind(trimmed)
        .bind(description)
        .bind(owner_id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;

        Ok(workspace)
    }

    /// Ensure the user has at least one workspace, creating the default if necessary
    pub async fn ensure_default(
        db: &sqlx::PgPool,
        owner_id: Uuid,
    ) -> Result<Self, AppError> {
        if let Some(existing) = sqlx::query_as::<_, Workspace>(
            r#"
            SELECT * FROM workspaces
            WHERE owner_id = $1
            ORDER BY created_at
            LIMIT 1
            "#
        )
        .bind(owner_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)? {
            return Ok(existing);
        }

        let workspace = Self::create(
            db,
            owner_id,
            DEFAULT_WORKSPACE_NAME.to_string(),
            Some(DEFAULT_WORKSPACE_DESCRIPTION.to_string()),
        )
        .await?;

        // TODO: Fix welcome project seeding - temporarily disabled due to type issues
        // Self::seed_welcome_project(db, owner_id, workspace.id).await?;

        Ok(workspace)
    }

    /// Fetch a specific workspace ensuring ownership
    pub async fn find_by_id(
        db: &sqlx::PgPool,
        workspace_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Self, AppError> {
        sqlx::query_as::<_, Workspace>(
            r#"
            SELECT * FROM workspaces
            WHERE id = $1 AND owner_id = $2
            "#
        )
        .bind(workspace_id)
        .bind(owner_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound {
            entity: "Workspace".to_string(),
            id: workspace_id.to_string(),
        })
    }

    /// Fetch workspace summary with nested projects
    pub async fn get_with_projects(
        db: &sqlx::PgPool,
        workspace_id: Uuid,
        owner_id: Uuid,
    ) -> Result<WorkspaceSummary, AppError> {
        let workspaces = Self::list_for_user(db, owner_id).await?;
        workspaces
            .into_iter()
            .find(|summary| summary.id == workspace_id)
            .ok_or_else(|| AppError::NotFound {
                entity: "Workspace".to_string(),
                id: workspace_id.to_string(),
            })
    }

    /// Retrieve a project with file payloads, ensuring ownership
    pub async fn get_project_details(
        db: &sqlx::PgPool,
        workspace_id: Uuid,
        project_id: Uuid,
        owner_id: Uuid,
    ) -> Result<ProjectDetails, AppError> {
        Self::find_by_id(db, workspace_id, owner_id).await?;

        let project = sqlx::query_as::<_, Project>(
            r#"
            SELECT * FROM projects
            WHERE id = $1 AND workspace_id = $2
            "#
        )
        .bind(project_id)
        .bind(workspace_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound {
            entity: "Project".to_string(),
            id: project_id.to_string(),
        })?;

        let files = sqlx::query_as::<_, File>(
            r#"
            SELECT * FROM files
            WHERE project_id = $1 AND is_deleted = false
            ORDER BY path
            "#
        )
        .bind(project_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)?;

        let details = ProjectDetails {
            id: project.id,
            workspace_id,
            name: project.name.clone(),
            description: project.description.clone(),
            main_file: Some(project.main_file_path.clone()),
            created_at: project.created_at,
            updated_at: project.updated_at,
            files: files
                .into_iter()
                .map(|file| ProjectFileDetails {
                    id: file.id,
                    path: file.path.clone(),
                    content: file.content.clone(),
                    is_main: file.is_main,
                    updated_at: file.updated_at,
                })
                .collect(),
        };

        Ok(details)
    }

    /// Create a welcome project pre-populated with useful files
    pub async fn seed_welcome_project(
        db: &sqlx::PgPool,
        owner_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<Project, AppError> {
        let create_project = CreateProject {
            name: format!("{}", DEFAULT_PROJECT_NAME),
            description: Some(DEFAULT_PROJECT_DESCRIPTION.to_string()),
            is_public: Some(false),
            main_file_path: Some("main.tex".to_string()),
            latex_engine: None,
            output_format: None,
            custom_args: None,
            bibliography_path: None,
            tags: None,
            workspace_id: Some(workspace_id),
        };

        let project = Project::create(db, owner_id, create_project).await?;

        // main.tex
        File::create(
            db,
            project.id,
            CreateFile {
                name: "main.tex".to_string(),
                path: "main.tex".to_string(),
                content: Some(DEFAULT_MAIN_TEX.to_string()),
                content_type: Some(ContentType::Latex),
            },
            owner_id,
        )
        .await?;

        // sections/introduction.tex
        File::create(
            db,
            project.id,
            CreateFile {
                name: "introduction.tex".to_string(),
                path: "sections/introduction.tex".to_string(),
                content: Some(DEFAULT_INTRO_TEX.to_string()),
                content_type: Some(ContentType::Latex),
            },
            owner_id,
        )
        .await?;

        Ok(project)
    }
}

fn normalize_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Name cannot be empty".to_string()));
    }
    Ok(trimmed.to_string())
}
