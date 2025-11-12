//! LaTeX compilation models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::{CompilationStatus, Entity, LatexEngine};

/// Compilation job
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompilationJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub file_id: Option<Uuid>, // Main file to compile, None for project default
    pub engine: LatexEngine,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: String,
    pub input_files: Vec<String>, // JSON array
    pub output_files: Vec<String>, // JSON array
    pub status: CompilationStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error_message: Option<String>,
    pub log_file_path: Option<String>,
    pub artifacts_created: i32,
    pub output_size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for CompilationJob {
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

/// Compilation queue item
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompilationQueue {
    pub id: Uuid,
    pub job_id: Uuid,
    pub priority: QueuePriority,
    pub queue_position: i32,
    pub estimated_duration_seconds: Option<i32>,
    pub worker_id: Option<String>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub retry_count: i32,
    pub max_retries: i32,
}

impl Entity for CompilationQueue {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.queued_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.started_at.unwrap_or(self.queued_at)
    }
}

/// Queue priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum QueuePriority {
    #[serde(rename = "low")]
    #[sqlx(rename = "low")]
    Low,
    #[serde(rename = "normal")]
    #[sqlx(rename = "normal")]
    Normal,
    #[serde(rename = "high")]
    #[sqlx(rename = "high")]
    High,
    #[serde(rename = "urgent")]
    #[sqlx(rename = "urgent")]
    Urgent,
}

impl Default for QueuePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Compilation worker
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompilationWorker {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub status: WorkerStatus,
    pub capabilities: Vec<String>, // JSON array
    pub max_concurrent_jobs: i32,
    pub current_jobs: i32,
    pub total_jobs_processed: i64,
    pub average_processing_time_ms: i64,
    pub last_heartbeat: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Entity for CompilationWorker {
    fn id(&self) -> Uuid {
        // Use string ID for workers but convert to UUID for compatibility
        Uuid::parse_str(&self.id).unwrap_or_else(|_| Uuid::new_v4())
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.last_heartbeat
    }
}

/// Worker status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum WorkerStatus {
    #[serde(rename = "idle")]
    #[sqlx(rename = "idle")]
    Idle,
    #[serde(rename = "busy")]
    #[sqlx(rename = "busy")]
    Busy,
    #[serde(rename = "maintenance")]
    #[sqlx(rename = "maintenance")]
    Maintenance,
    #[serde(rename = "offline")]
    #[sqlx(rename = "offline")]
    Offline,
}

impl Default for WorkerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// Compilation template
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompilationTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub engine: LatexEngine,
    pub command_template: String,
    pub default_args: Vec<String>, // JSON array
    pub required_files: Vec<String>, // JSON array
    pub output_patterns: Vec<String>, // JSON array
    pub is_public: bool,
    pub created_by: Uuid,
    pub usage_count: i64,
    pub success_rate: f64, // 0.0 to 1.0
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for CompilationTemplate {
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

/// Compilation artifact
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompilationArtifact {
    pub id: Uuid,
    pub job_id: Uuid,
    pub file_path: String,
    pub file_name: String,
    pub file_type: ArtifactType,
    pub file_size_bytes: i64,
    pub mime_type: String,
    pub storage_path: String,
    pub is_downloadable: bool,
    pub download_count: i32,
    pub created_at: DateTime<Utc>,
}

impl Entity for CompilationArtifact {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// Artifact type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum ArtifactType {
    #[serde(rename = "pdf")]
    #[sqlx(rename = "pdf")]
    Pdf,
    #[serde(rename = "dvi")]
    #[sqlx(rename = "dvi")]
    Dvi,
    #[serde(rename = "ps")]
    #[sqlx(rename = "ps")]
    Ps,
    #[serde(rename = "log")]
    #[sqlx(rename = "log")]
    Log,
    #[serde(rename = "aux")]
    #[sqlx(rename = "aux")]
    Aux,
    #[serde(rename = "bbl")]
    #[sqlx(rename = "bbl")]
    Bbl,
    #[serde(rename = "other")]
    #[sqlx(rename = "other")]
    Other,
}

/// Helper struct for compilation stats query result
#[derive(Debug, Clone, FromRow)]
struct CompilationStatsRow {
    pub total_jobs: i64,
    pub successful_jobs: i64,
    pub failed_jobs: i64,
    pub cancelled_jobs: i64,
    pub avg_duration: f64,
    pub total_output_size: i64,
}

/// Compilation statistics
#[derive(Debug, Clone, Serialize)]
pub struct CompilationStats {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_jobs: i64,
    pub successful_jobs: i64,
    pub failed_jobs: i64,
    pub cancelled_jobs: i64,
    pub average_duration_ms: f64,
    pub total_output_size_mb: f64,
    pub success_rate: f64,
    pub jobs_by_engine: Vec<EngineStats>,
    pub jobs_by_status: Vec<StatusStats>,
    pub top_error_messages: Vec<ErrorStats>,
}

/// Engine-specific statistics
#[derive(Debug, Clone, Serialize)]
pub struct EngineStats {
    pub engine: LatexEngine,
    pub job_count: i64,
    pub success_count: i64,
    pub average_duration_ms: f64,
}

/// Status-specific statistics
#[derive(Debug, Clone, Serialize)]
pub struct StatusStats {
    pub status: CompilationStatus,
    pub count: i64,
}

/// Error message statistics
#[derive(Debug, Clone, Serialize)]
pub struct ErrorStats {
    pub error_message: String,
    pub count: i64,
    pub first_occurrence: DateTime<Utc>,
}

/// Request for creating a compilation job
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCompilationJob {
    pub file_id: Option<Uuid>,
    pub engine: Option<LatexEngine>,
    pub args: Option<Vec<String>>,
    pub priority: Option<QueuePriority>,
    pub template_id: Option<Uuid>,
}

/// Request for creating a compilation template
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCompilationTemplate {
    pub name: String,
    pub description: Option<String>,
    pub engine: LatexEngine,
    pub command_template: String,
    pub default_args: Option<Vec<String>>,
    pub required_files: Option<Vec<String>>,
    pub output_patterns: Option<Vec<String>>,
    pub is_public: Option<bool>,
}

impl CompilationJob {
    /// Create a new compilation job
    pub async fn create(
        db: &sqlx::PgPool,
        project_id: Uuid,
        user_id: Uuid,
        create_job: CreateCompilationJob,
        engine: LatexEngine,
        working_directory: String,
        input_files: Vec<String>,
    ) -> Result<Self, crate::error::AppError> {
        let command = match engine {
            LatexEngine::Pdflatex => "pdflatex".to_string(),
            LatexEngine::Xelatex => "xelatex".to_string(),
            LatexEngine::Lualatex => "lualatex".to_string(),
        };

        let args = create_job.args.unwrap_or_else(|| vec![
            "-interaction=nonstopmode".to_string(),
            "-file-line-error".to_string(),
            "-synctex=1".to_string(),
            "-output-directory=output".to_string(),
        ]);

        let job = sqlx::query_as::<_, CompilationJob>(
            r#"
            INSERT INTO compilation_jobs (
                project_id, user_id, file_id, engine, command, args,
                working_directory, input_files, status, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .bind(create_job.file_id)
        .bind(engine as LatexEngine)
        .bind(command)
        .bind(&args)
        .bind(working_directory)
        .bind(&input_files)
        .bind(CompilationStatus::Pending as CompilationStatus)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Add to compilation queue
        CompilationQueue::enqueue(db, job.id, create_job.priority.unwrap_or_default()).await?;

        Ok(job)
    }

    /// Find compilation job by ID
    pub async fn find_by_id(
        db: &sqlx::PgPool,
        job_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let job = sqlx::query_as::<_, CompilationJob>(
            r#"
            SELECT cj.* FROM compilation_jobs cj
            JOIN projects p ON cj.project_id = p.id
            WHERE cj.id = $1 AND (
                cj.user_id = $2 OR
                p.owner_id = $2 OR
                p.id IN (
                    SELECT project_id FROM project_collaborators
                    WHERE user_id = $2
                ) OR
                p.is_public = true
            )
            "#
        )
        .bind(job_id)
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(job)
    }

    /// Update job status
    pub async fn update_status(
        &self,
        db: &sqlx::PgPool,
        status: CompilationStatus,
        error_message: Option<String>,
    ) -> Result<(), crate::error::AppError> {
        let (completed_at, duration_ms) = match status {
            CompilationStatus::Success | CompilationStatus::Error | CompilationStatus::Cancelled => {
                let completed_at = Some(Utc::now());
                let duration_ms = if let Some(started_at) = self.started_at {
                    Some((completed_at.unwrap() - started_at).num_milliseconds())
                } else {
                    None
                };
                (completed_at, duration_ms)
            }
            _ => (None, None),
        };

        sqlx::query(
            r#"
            UPDATE compilation_jobs
            SET status = $1, error_message = $2, completed_at = $3, duration_ms = $4, updated_at = $5
            WHERE id = $6
            "#
        )
        .bind(status as CompilationStatus)
        .bind(error_message)
        .bind(completed_at)
        .bind(duration_ms)
        .bind(Utc::now())
        .bind(self.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Update project compilation status if successful
        if status == CompilationStatus::Success {
            sqlx::query(
                "UPDATE projects SET compilation_status = $1, last_compilation_at = $2 WHERE id = $3"
            )
            .bind(status as CompilationStatus)
            .bind(Utc::now())
            .bind(self.project_id)
            .execute(db)
            .await
            .map_err(crate::error::AppError::Database)?;
        }

        Ok(())
    }

    /// Start the compilation job
    pub async fn start(
        &self,
        db: &sqlx::PgPool,
        worker_id: Option<String>,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query(
            "UPDATE compilation_jobs SET status = $1, started_at = $2, updated_at = $3 WHERE id = $4"
        )
        .bind(CompilationStatus::Running as CompilationStatus)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(self.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Update queue
        if let Some(queue_id) = self.get_queue_id(db).await? {
            sqlx::query(
                "UPDATE compilation_queue SET started_at = $1, worker_id = $2 WHERE id = $3"
            )
            .bind(Utc::now())
            .bind(worker_id)
            .bind(queue_id)
            .execute(db)
            .await
            .map_err(crate::error::AppError::Database)?;
        }

        Ok(())
    }

    /// Complete the compilation job
    pub async fn complete(
        &self,
        db: &sqlx::PgPool,
        exit_code: i32,
        stdout: String,
        stderr: String,
        output_files: Vec<String>,
        artifacts_created: i32,
        output_size_bytes: i64,
    ) -> Result<(), crate::error::AppError> {
        let status = if exit_code == 0 {
            CompilationStatus::Success
        } else {
            CompilationStatus::Error
        };

        let completed_at = Some(Utc::now());
        let duration_ms = if let Some(started_at) = self.started_at {
            Some((completed_at.unwrap() - started_at).num_milliseconds())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE compilation_jobs
            SET status = $1, completed_at = $2, duration_ms = $3, exit_code = $4,
                stdout = $5, stderr = $6, output_files = $7, artifacts_created = $8,
                output_size_bytes = $9, updated_at = $10
            WHERE id = $11
            "#
        )
        .bind(status as CompilationStatus)
        .bind(completed_at)
        .bind(duration_ms)
        .bind(exit_code)
        .bind(stdout)
        .bind(stderr)
        .bind(&output_files)
        .bind(artifacts_created)
        .bind(output_size_bytes)
        .bind(Utc::now())
        .bind(self.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Remove from queue
        sqlx::query(
            "DELETE FROM compilation_queue WHERE job_id = $1"
        )
        .bind(self.id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        // Update project compilation status
        sqlx::query(
            "UPDATE projects SET compilation_status = $1, last_compilation_at = $2 WHERE id = $3"
        )
        .bind(status as CompilationStatus)
        .bind(Utc::now())
        .bind(self.project_id)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Get queue ID for this job
    async fn get_queue_id(&self, db: &sqlx::PgPool) -> Result<Option<Uuid>, crate::error::AppError> {
        let queue_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM compilation_queue WHERE job_id = $1"
        )
        .bind(self.id)
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(queue_id.and_then(|id| Some(id)))
    }

    /// List jobs for a user
    pub async fn list_for_user(
        db: &sqlx::PgPool,
        user_id: Uuid,
        params: &super::PaginationParams,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let jobs = sqlx::query_as::<_, CompilationJob>(
            r#"
            SELECT cj.* FROM compilation_jobs cj
            JOIN projects p ON cj.project_id = p.id
            WHERE cj.user_id = $1 OR p.owner_id = $1 OR p.id IN (
                SELECT project_id FROM project_collaborators WHERE user_id = $1
            )
            ORDER BY cj.created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(params.limit() as i64)
        .bind(params.offset() as i64)
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(jobs)
    }
}

impl CompilationQueue {
    /// Add job to compilation queue
    pub async fn enqueue(
        db: &sqlx::PgPool,
        job_id: Uuid,
        priority: QueuePriority,
    ) -> Result<Self, crate::error::AppError> {
        // Get the next queue position for this priority
        let queue_position = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(queue_position), 0) + 1 FROM compilation_queue WHERE priority = $1"
        )
        .bind(priority as QueuePriority)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        let queue_item = sqlx::query_as::<_, CompilationQueue>(
            r#"
            INSERT INTO compilation_queue (job_id, priority, queue_position, queued_at, retry_count, max_retries)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(job_id)
        .bind(priority as QueuePriority)
        .bind(queue_position)
        .bind(Utc::now())
        .bind(0)
        .bind(3)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(queue_item)
    }

    /// Get next job from queue
    pub async fn dequeue(db: &sqlx::PgPool) -> Result<Option<(Self, CompilationJob)>, crate::error::AppError> {
        let queue_item = sqlx::query_as::<_, CompilationQueue>(
            r#"
            UPDATE compilation_queue
            SET started_at = NOW()
            WHERE id = (
                SELECT id FROM compilation_queue
                WHERE started_at IS NULL
                ORDER BY priority DESC, queue_position ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *
            "#
        )
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        if let Some(queue_item) = queue_item {
            let job = sqlx::query_as::<_, CompilationJob>(
                "SELECT * FROM compilation_jobs WHERE id = $1"
            )
            .bind(queue_item.job_id)
            .fetch_one(db)
            .await
            .map_err(crate::error::AppError::Database)?;

            Ok(Some((queue_item, job)))
        } else {
            Ok(None)
        }
    }

    /// Get queue length
    pub async fn get_queue_length(db: &sqlx::PgPool) -> Result<i64, crate::error::AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM compilation_queue WHERE started_at IS NULL"
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(count)
    }
}

impl CompilationTemplate {
    /// Create a new compilation template
    pub async fn create(
        db: &sqlx::PgPool,
        created_by: Uuid,
        create_template: CreateCompilationTemplate,
    ) -> Result<Self, crate::error::AppError> {
        let template = sqlx::query_as::<_, CompilationTemplate>(
            r#"
            INSERT INTO compilation_templates (
                name, description, engine, command_template, default_args,
                required_files, output_patterns, is_public, created_by,
                usage_count, success_rate, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#
        )
        .bind(create_template.name)
        .bind(create_template.description)
        .bind(create_template.engine as LatexEngine)
        .bind(create_template.command_template)
        .bind(create_template.default_args.unwrap_or_default())
        .bind(create_template.required_files.unwrap_or_default())
        .bind(create_template.output_patterns.unwrap_or_default())
        .bind(create_template.is_public.unwrap_or(false))
        .bind(created_by)
        .bind(0)
        .bind(1.0)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(template)
    }

    /// Update template usage statistics
    pub async fn update_usage_stats(
        &self,
        db: &sqlx::PgPool,
        success: bool,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query(
            r#"
            UPDATE compilation_templates
            SET
                usage_count = usage_count + 1,
                success_rate = (
                    (success_rate * (usage_count - 1) + CASE WHEN $2 THEN 1.0 ELSE 0.0 END) / usage_count
                ),
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(self.id)
        .bind(success)
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }
}

impl CompilationStats {
    /// Get compilation statistics for a period
    pub async fn get_stats(
        db: &sqlx::PgPool,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Self, crate::error::AppError> {
        let basic_stats = sqlx::query_as::<_, CompilationStatsRow>(
            r#"
            SELECT
                COUNT(*) as total_jobs,
                COUNT(*) FILTER (WHERE status = 'success') as successful_jobs,
                COUNT(*) FILTER (WHERE status = 'error') as failed_jobs,
                COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled_jobs,
                COALESCE(AVG(duration_ms), 0) as avg_duration,
                COALESCE(SUM(output_size_bytes), 0) as total_output_size
            FROM compilation_jobs
            WHERE created_at BETWEEN $1 AND $2
            "#
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        let total_jobs = basic_stats.total_jobs;
        let successful_jobs = basic_stats.successful_jobs;
        let failed_jobs = basic_stats.failed_jobs;
        let cancelled_jobs = basic_stats.cancelled_jobs;

        let success_rate = if total_jobs > 0 {
            successful_jobs as f64 / total_jobs as f64
        } else {
            0.0
        };

        Ok(CompilationStats {
            period_start,
            period_end,
            total_jobs,
            successful_jobs,
            failed_jobs,
            cancelled_jobs,
            average_duration_ms: basic_stats.avg_duration,
            total_output_size_mb: basic_stats.total_output_size as f64 / (1024.0 * 1024.0),
            success_rate,
            jobs_by_engine: vec![], // TODO: Implement engine-specific stats
            jobs_by_status: vec![],  // TODO: Implement status-specific stats
            top_error_messages: vec![], // TODO: Implement error message stats
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_priority_default() {
        assert_eq!(QueuePriority::default(), QueuePriority::Normal);
    }

    #[test]
    fn test_worker_status_default() {
        assert_eq!(WorkerStatus::default(), WorkerStatus::Idle);
    }

    #[test]
    fn test_artifact_type_values() {
        assert_eq!(ArtifactType::Pdf as &str, "pdf");
        assert_eq!(ArtifactType::Log as &str, "log");
        assert_eq!(ArtifactType::Aux as &str, "aux");
    }
}