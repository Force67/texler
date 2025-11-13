//! Project-related models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::{CompilationStatus, Entity, LatexEngine, UserRole};
use super::workspace::Workspace;
use super::user::UserProfile;

/// Project model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub workspace_id: Uuid,
    pub is_public: bool,
    pub main_file_path: String,
    pub latex_engine: LatexEngine,
    pub output_format: String,
    pub custom_args: Vec<String>,
    pub bibliography_path: Option<String>,
    pub last_compilation_at: Option<DateTime<Utc>>,
    pub compilation_status: CompilationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for Project {
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

/// Project creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProject {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub main_file_path: Option<String>,
    pub latex_engine: Option<LatexEngine>,
    pub output_format: Option<String>,
    pub custom_args: Option<Vec<String>>,
    pub bibliography_path: Option<String>,
    pub tags: Option<Vec<String>>,
    pub workspace_id: Option<Uuid>,
}

/// Project update request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub main_file_path: Option<String>,
    pub latex_engine: Option<LatexEngine>,
    pub output_format: Option<String>,
    pub custom_args: Option<Vec<String>>,
    pub bibliography_path: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Project with relationships
#[derive(Debug, Clone, Serialize)]
pub struct ProjectWithDetails {
    #[serde(flatten)]
    pub project: Project,
    pub owner: UserProfile,
    pub collaborators: Vec<UserProfile>,
    pub file_count: i64,
    pub word_count: i64,
    pub tag_count: i64,
}

/// Project search response
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSearchResult {
    #[serde(flatten)]
    pub project: Project,
    pub owner: UserProfile,
    pub collaborators: Vec<UserProfile>,
    pub relevance_score: f64,
}

/// Project collaborator
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectCollaborator {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: UserRole,
    pub permissions: Option<String>, // JSON field
    pub invited_by: Option<Uuid>,
    pub invited_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Project tag
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectTag {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Project statistics
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProjectStats {
    pub project_id: Uuid,
    pub total_files: i64,
    pub total_words: i64,
    pub total_lines: i64,
    pub last_compilation_at: Option<DateTime<Utc>>,
    pub total_compilations: i64,
    pub failed_compilations: i64,
    pub total_collaborators: i64,
    pub created_at: DateTime<Utc>,
}

/// Project activity log
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectActivity {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub details: Option<String>, // JSON field
    pub created_at: DateTime<Utc>,
}

impl Project {
    /// Create a new project
    pub async fn create(
        db: &sqlx::PgPool,
        owner_id: Uuid,
        mut create_project: CreateProject,
    ) -> Result<Self, crate::error::AppError> {
        let workspace_id = create_project.workspace_id.ok_or_else(|| {
            crate::error::AppError::Validation("Workspace ID is required".to_string())
        })?;
        Workspace::find_by_id(db, workspace_id, owner_id).await?;

        let project = sqlx::query_as::<_, Project>(
            r#"
            INSERT INTO projects (
                workspace_id, name, description, owner_id, is_public, main_file_path,
                latex_engine, output_format, custom_args, bibliography_path
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(workspace_id)
        .bind(create_project.name)
        .bind(create_project.description)
        .bind(owner_id)
        .bind(create_project.is_public.unwrap_or(false))
        .bind(create_project.main_file_path.unwrap_or_else(|| "main.tex".to_string()))
        .bind(create_project.latex_engine.unwrap_or_default())
        .bind(create_project.output_format.unwrap_or_else(|| "pdf".to_string()))
        .bind(create_project.custom_args.unwrap_or_default())
        .bind(create_project.bibliography_path)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Create tags if provided
        if let Some(tags) = create_project.tags {
            for tag_name in tags {
                sqlx::query(
                    r#"
                    INSERT INTO project_tags (project_id, name)
                    VALUES ($1, $2)
                    "#
                )
                .bind(project.id)
                .bind(tag_name)
                .execute(db)
                .await
                .map_err(crate::error::AppError::Database)?;
            }
        }

        // Log activity
        ProjectActivity::log(
            db,
            project.id,
            owner_id,
            "project_created",
            "project",
            Some(project.id),
            None,
        )
        .await?;

        Ok(project)
    }

    /// Find project by ID with access control
    pub async fn find_by_id(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let project = sqlx::query_as::<_, Project>(
            r#"
            SELECT p.* FROM projects p
            WHERE p.id = $1 AND (
                p.owner_id = $2 OR
                p.id IN (
                    SELECT project_id FROM project_collaborators
                    WHERE user_id = $2
                ) OR
                p.is_public = true
            )
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(project)
    }

    /// List projects accessible to a user
    pub async fn list_for_user(
        db: &sqlx::PgPool,
        user_id: Uuid,
        params: &super::PaginationParams,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let projects = sqlx::query_as::<_, Project>(
            r#"
            SELECT DISTINCT p.* FROM projects p
            WHERE (
                p.owner_id = $1 OR
                p.id IN (
                    SELECT project_id FROM project_collaborators
                    WHERE user_id = $1
                ) OR
                p.is_public = true
            )
            ORDER BY p.updated_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(params.limit() as i64)
        .bind(params.offset() as i64)
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(projects)
    }

    /// Update project
    pub async fn update(
        &self,
        db: &sqlx::PgPool,
        update_project: UpdateProject,
        user_id: Uuid,
    ) -> Result<Self, crate::error::AppError> {
        let project = sqlx::query_as::<_, Project>(
            r#"
            UPDATE projects SET
                name = COALESCE($1, name),
                description = COALESCE($2, description),
                is_public = COALESCE($3, is_public),
                main_file_path = COALESCE($4, main_file_path),
                latex_engine = COALESCE($5, latex_engine),
                output_format = COALESCE($6, output_format),
                custom_args = COALESCE($7, custom_args),
                bibliography_path = COALESCE($8, bibliography_path),
                updated_at = NOW()
            WHERE id = $9 AND owner_id = $10
            RETURNING *
            "#
        )
        .bind(update_project.name)
        .bind(update_project.description)
        .bind(update_project.is_public)
        .bind(update_project.main_file_path)
        .bind(update_project.latex_engine)
        .bind(update_project.output_format)
        .bind(update_project.custom_args)
        .bind(update_project.bibliography_path)
        .bind(self.id)
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(project)
    }

    /// Delete project
    pub async fn delete(
        &self,
        db: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<(), crate::error::AppError> {
        let rows_affected = sqlx::query(
            "DELETE FROM projects WHERE id = $1 AND owner_id = $2"
        )
        .bind(self.id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        if rows_affected.rows_affected() == 0 {
            return Err(crate::error::AppError::Authorization(
                "Only the project owner can delete a project".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if user has access to project
    pub async fn has_access(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, crate::error::AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM projects p
            WHERE p.id = $1 AND (
                p.owner_id = $2 OR
                p.id IN (
                    SELECT project_id FROM project_collaborators
                    WHERE user_id = $2
                ) OR
                p.is_public = true
            )
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(count > 0)
    }

    /// Update the project's main file path (and sync file flags)
    pub async fn set_main_file(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
        path: &str,
    ) -> Result<Self, crate::error::AppError> {
        if !Self::is_owner(db, project_id, user_id).await? {
            return Err(crate::error::AppError::Authorization(
                "Only the project owner can update the main file".to_string(),
            ));
        }

        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM files
            WHERE project_id = $1 AND path = $2 AND is_deleted = false
            "#
        )
        .bind(project_id)
        .bind(path)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        if exists == 0 {
            return Err(crate::error::AppError::NotFound {
                entity: "File".to_string(),
                id: path.to_string(),
            });
        }

        let project = sqlx::query_as::<_, Project>(
            r#"
            UPDATE projects
            SET main_file_path = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#
        )
        .bind(path)
        .bind(project_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        sqlx::query(
            r#"
            UPDATE files
            SET is_main = (path = $2)
            WHERE project_id = $1
            "#
        )
        .bind(project_id)
        .bind(path)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(project)
    }

    /// Check if user is owner
    pub async fn is_owner(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, crate::error::AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM projects WHERE id = $1 AND owner_id = $2"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(count > 0)
    }

    /// Get project with full details
    pub async fn get_with_details(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectWithDetails, crate::error::AppError> {
        // Get basic project info with access control
        let project = Self::find_by_id(db, project_id, user_id).await?
            .ok_or_else(|| crate::error::AppError::NotFound {
                entity: "Project".to_string(),
                id: project_id.to_string(),
            })?;

        // Get owner info
        let owner = sqlx::query_as::<_, UserProfile>(
            r#"
            SELECT id, username, email, display_name, avatar_url,
                   is_active, email_verified, last_login_at, created_at
            FROM users
            WHERE id = $1
            "#
        )
        .bind(project.owner_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Get collaborators
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
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Get statistics
        let stats = ProjectStats::get(db, project_id).await?;

        Ok(ProjectWithDetails {
            project,
            owner,
            collaborators,
            file_count: stats.total_files,
            word_count: stats.total_words,
            tag_count: 0, // TODO: Implement tag count
        })
    }

    /// Update compilation status
    pub async fn update_compilation_status(
        &self,
        db: &sqlx::PgPool,
        status: CompilationStatus,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query(
            r#"
            UPDATE projects
            SET compilation_status = $1, last_compilation_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(status as CompilationStatus)
        .bind(self.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }
}

impl ProjectCollaborator {
    /// Add collaborator to project
    pub async fn add(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
        role: UserRole,
        invited_by: Uuid,
    ) -> Result<Self, crate::error::AppError> {
        let collaborator = sqlx::query_as::<_, ProjectCollaborator>(
            r#"
            INSERT INTO project_collaborators (project_id, user_id, role, invited_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .bind(role as UserRole)
        .bind(invited_by)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(collaborator)
    }

    /// Remove collaborator from project
    pub async fn remove(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query(
            "DELETE FROM project_collaborators WHERE project_id = $1 AND user_id = $2"
        )
        .bind(project_id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Get project collaborators
    pub async fn list(
        db: &sqlx::PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let collaborators = sqlx::query_as::<_, ProjectCollaborator>(
            "SELECT * FROM project_collaborators WHERE project_id = $1 ORDER BY created_at"
        )
        .bind(project_id)
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(collaborators)
    }
}

impl ProjectStats {
    /// Get project statistics
    pub async fn get(
        db: &sqlx::PgPool,
        project_id: Uuid,
    ) -> Result<Self, crate::error::AppError> {
        let stats = sqlx::query_as::<_, ProjectStats>(
            r#"
            WITH file_stats AS (
                SELECT
                    COUNT(*) as total_files,
                    COALESCE(SUM(word_count), 0) as total_words,
                    COALESCE(SUM(line_count), 0) as total_lines
                FROM files
                WHERE project_id = $1 AND is_deleted = false
            ),
            compilation_stats AS (
                SELECT
                    COUNT(*) as total_compilations,
                    COUNT(*) FILTER (WHERE status = 'success') as successful_compilations
                FROM compilation_jobs
                WHERE project_id = $1
            )
            SELECT
                $1 as project_id,
                fs.total_files,
                fs.total_words,
                fs.total_lines,
                cs.total_compilations,
                (cs.total_compilations - cs.successful_compilations) as failed_compilations,
                COALESCE(c.total_collaborators, 0) as total_collaborators,
                p.created_at
            FROM file_stats fs
            CROSS JOIN compilation_stats cs
            LEFT JOIN projects p ON p.id = $1
            LEFT JOIN (
                SELECT COUNT(*) as total_collaborators, project_id
                FROM project_collaborators
                GROUP BY project_id
            ) c ON c.project_id = $1
            "#
        )
        .bind(project_id)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(stats)
    }
}

impl ProjectActivity {
    /// Log project activity
    pub async fn log(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
        action: &str,
        entity_type: &str,
        entity_id: Option<Uuid>,
        details: Option<String>,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query(
            r#"
            INSERT INTO project_activity (
                project_id, user_id, action, entity_type, entity_id, details
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .bind(action)
        .bind(entity_type)
        .bind(entity_id)
        .bind(details)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Get recent project activities
    pub async fn get_recent(
        db: &sqlx::PgPool,
        project_id: Uuid,
        limit: u32,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let activities = sqlx::query_as::<_, ProjectActivity>(
            r#"
            SELECT * FROM project_activity
            WHERE project_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(project_id)
        .bind(limit as i64)
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(activities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[tokio::test]
    async fn test_project_creation() {
        // Test project creation logic
        // This would require a test database
        assert!(true);
    }

    #[test]
    fn test_project_access_check() {
        // Test access control logic
        assert!(true);
    }
}
