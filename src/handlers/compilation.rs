//! Compilation request handlers

use crate::error::AppError;
use crate::models::compilation::{
    CompilationJob, CreateCompilationJob, CompilationTemplate, CreateCompilationTemplate,
    CompilationStats, QueuePriority
};
use crate::models::LatexEngine;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Compilation job response
#[derive(Debug, Serialize)]
pub struct CompilationJobResponse {
    pub job: CompilationJob,
}

/// Compilation jobs list response
#[derive(Debug, Serialize)]
pub struct CompilationJobsListResponse {
    pub jobs: Vec<CompilationJob>,
    pub pagination: crate::models::PaginationInfo,
}

/// Compilation queue status response
#[derive(Debug, Serialize)]
pub struct QueueStatusResponse {
    pub queue_length: i64,
    pub processing_jobs: i64,
    pub average_wait_time_minutes: f64,
    pub workers_online: i64,
}

/// Compilation templates list response
#[derive(Debug, Serialize)]
pub struct CompilationTemplatesListResponse {
    pub templates: Vec<CompilationTemplate>,
    pub pagination: crate::models::PaginationInfo,
}

/// Compilation job creation request
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub project_id: Uuid,
    pub file_id: Option<Uuid>,
    pub engine: Option<LatexEngine>,
    pub args: Option<Vec<String>>,
    pub priority: Option<QueuePriority>,
    pub template_id: Option<Uuid>,
}

/// Job cancellation request
#[derive(Debug, Deserialize)]
pub struct CancelJobRequest {
    pub reason: Option<String>,
}

/// Application state for compilation handlers
#[derive(Clone)]
pub struct CompilationState {
    pub db_pool: sqlx::PgPool,
    pub config: crate::config::Config,
}

/// List compilation jobs for the user
pub async fn list_jobs(
    State(state): State<CompilationState>,
    Query(params): Query<crate::models::PaginationParams>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let jobs = CompilationJob::list_for_user(&state.db_pool, auth_user.user_id, &params).await?;

    // Get total count for pagination
    let total_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM compilation_jobs cj
        JOIN projects p ON cj.project_id = p.id
        WHERE cj.user_id = $1 OR p.owner_id = $1 OR p.id IN (
            SELECT project_id FROM project_collaborators WHERE user_id = $1
        )
        "#,
        auth_user.user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let pagination_info = crate::models::PaginatedResponse::new(
        jobs.clone(),
        &params,
        total_count.unwrap_or(0) as u64,
    ).pagination;

    let response = CompilationJobsListResponse {
        jobs,
        pagination: pagination_info,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Create a new compilation job
pub async fn create_job(
    State(state): State<CompilationState>,
    Json(payload): Json<CreateJobRequest>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check project access
    if !crate::models::project::Project::has_access(&state.db_pool, payload.project_id, auth_user.user_id).await? {
        return Err(AppError::NotFound {
            entity: "Project",
            id: payload.project_id.to_string(),
        });
    }

    let create_job = CreateCompilationJob {
        file_id: payload.file_id,
        engine: payload.engine,
        args: payload.args,
        priority: payload.priority,
        template_id: payload.template_id,
    };

    let working_directory = format!("/tmp/texler/projects/{}", payload.project_id);
    let input_files = vec![]; // TODO: Get project files

    let job = CompilationJob::create(
        &state.db_pool,
        payload.project_id,
        auth_user.user_id,
        create_job,
        payload.engine.unwrap_or_default(),
        working_directory,
        input_files,
    )
    .await?;

    let response = CompilationJobResponse {
        job,
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": response
        })),
    ))
}

/// Get compilation job details
pub async fn get_job(
    State(state): State<CompilationState>,
    Path(job_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let job = CompilationJob::find_by_id(&state.db_pool, job_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CompilationJob",
            id: job_id.to_string(),
        })?;

    let response = CompilationJobResponse {
        job,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Cancel compilation job
pub async fn cancel_job(
    State(state): State<CompilationState>,
    Path(job_id): Path<Uuid>,
    Json(_payload): Json<CancelJobRequest>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let job = CompilationJob::find_by_id(&state.db_pool, job_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CompilationJob",
            id: job_id.to_string(),
        })?;

    // Only allow cancellation if job is pending or running
    match job.status {
        crate::models::CompilationStatus::Pending | crate::models::CompilationStatus::Running => {
            job.update_status(
                &state.db_pool,
                crate::models::CompilationStatus::Cancelled,
                Some("Cancelled by user".to_string()),
            )
            .await?;
        }
        _ => {
            return Err(AppError::Validation(
                "Cannot cancel a completed job".to_string(),
            ));
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Compilation job cancelled successfully"
    })))
}

/// Get compilation job logs
pub async fn get_job_logs(
    State(state): State<CompilationState>,
    Path(job_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let job = CompilationJob::find_by_id(&state.db_pool, job_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CompilationJob",
            id: job_id.to_string(),
        })?;

    let logs = serde_json::json!({
        "stdout": job.stdout,
        "stderr": job.stderr,
        "exit_code": job.exit_code,
        "duration_ms": job.duration_ms,
        "started_at": job.started_at,
        "completed_at": job.completed_at
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "data": logs
    })))
}

/// Get compilation job artifacts
pub async fn get_job_artifacts(
    State(state): State<CompilationState>,
    Path(job_id): Path<Uuid>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let job = CompilationJob::find_by_id(&state.db_pool, job_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CompilationJob",
            id: job_id.to_string(),
        })?;

    let artifacts = serde_json::json!({
        "output_files": job.output_files,
        "artifacts_created": job.artifacts_created,
        "output_size_bytes": job.output_size_bytes,
        "download_urls": job.output_files.iter().map(|f| {
            format!("/api/v1/compilation/jobs/{}/artifacts/{}", job_id, f)
        }).collect::<Vec<String>>()
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "data": artifacts
    })))
}

/// Get compilation queue status
pub async fn get_queue_status(
    State(state): State<CompilationState>,
    _auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let queue_length = crate::models::compilation::CompilationQueue::get_queue_length(&state.db_pool).await?;

    let processing_jobs = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM compilation_jobs WHERE status = 'running'"
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let workers_online = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM compilation_workers WHERE status = 'idle' OR status = 'busy'"
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    // Calculate average wait time (simplified)
    let average_wait_time_minutes = if queue_length > 0 {
        5.0 // Placeholder - would need actual calculation based on historical data
    } else {
        0.0
    };

    let response = QueueStatusResponse {
        queue_length,
        processing_jobs: processing_jobs.unwrap_or(0),
        average_wait_time_minutes,
        workers_online: workers_online.unwrap_or(0),
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// List compilation templates
pub async fn list_templates(
    State(state): State<CompilationState>,
    Query(params): Query<crate::models::PaginationParams>,
    _auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let templates = sqlx::query_as!(
        CompilationTemplate,
        r#"
        SELECT * FROM compilation_templates
        WHERE is_public = true
        ORDER BY success_rate DESC, usage_count DESC
        LIMIT $1 OFFSET $2
        "#,
        params.limit() as i64,
        params.offset() as i64
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    // Get total count for pagination
    let total_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM compilation_templates WHERE is_public = true"
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let pagination_info = crate::models::PaginatedResponse::new(
        templates.clone(),
        &params,
        total_count.unwrap_or(0) as u64,
    ).pagination;

    let response = CompilationTemplatesListResponse {
        templates,
        pagination: pagination_info,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Create compilation template
pub async fn create_template(
    State(state): State<CompilationState>,
    Json(payload): Json<CreateCompilationTemplate>,
    auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let template = CompilationTemplate::create(&state.db_pool, auth_user.user_id, payload).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "template": template
        }
    })))
}

/// Get compilation template details
pub async fn get_template(
    State(state): State<CompilationState>,
    Path(template_id): Path<Uuid>,
    _auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let template = sqlx::query_as!(
        CompilationTemplate,
        "SELECT * FROM compilation_templates WHERE id = $1",
        template_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    if let Some(template) = template {
        Ok(Json(serde_json::json!({
            "success": true,
            "data": {
                "template": template
            }
        })))
    } else {
        Err(AppError::NotFound {
            entity: "CompilationTemplate",
            id: template_id.to_string(),
        })
    }
}

/// Get compilation statistics
pub async fn get_compilation_stats(
    State(state): State<CompilationState>,
    Query(params): Query<CompilationStatsParams>,
    _auth_user: axum::Extension<crate::models::auth::AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    use chrono::Duration;

    let period_days = params.days.unwrap_or(7);
    let period_start = chrono::Utc::now() - Duration::days(period_days);
    let period_end = chrono::Utc::now();

    let stats = CompilationStats::get_stats(&state.db_pool, period_start, period_end).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": stats
    })))
}

/// Compilation statistics parameters
#[derive(Debug, Deserialize)]
pub struct CompilationStatsParams {
    pub days: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[tokio::test]
    async fn test_compilation_job_creation() {
        let state = CompilationState {
            db_pool: PgPool::connect("postgresql://test").await.unwrap(),
            config: crate::config::Config::load().unwrap(),
        };

        let request = CreateJobRequest {
            project_id: uuid::Uuid::new_v4(),
            file_id: None,
            engine: Some(LatexEngine::Pdflatex),
            args: Some(vec!["-interaction=nonstopmode".to_string()]),
            priority: Some(QueuePriority::Normal),
            template_id: None,
        };

        // This test would require setting up proper auth context and test project
        // For now, we just verify the request structure
        assert_eq!(request.engine, Some(LatexEngine::Pdflatex));
        assert_eq!(request.priority, Some(QueuePriority::Normal));
    }

    #[test]
    fn test_queue_status_response() {
        let response = QueueStatusResponse {
            queue_length: 5,
            processing_jobs: 2,
            average_wait_time_minutes: 3.5,
            workers_online: 3,
        };

        assert_eq!(response.queue_length, 5);
        assert_eq!(response.processing_jobs, 2);
        assert_eq!(response.workers_online, 3);
    }
}