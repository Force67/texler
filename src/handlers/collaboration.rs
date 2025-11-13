//! Collaboration request handlers

use crate::error::AppError;
use crate::models::collaboration::{
    CollaborationSession, CreateCollaborationSession, UpdateCollaborationSession,
    SessionParticipant, SessionOperation, SessionMessage, SessionInvitation,
    SessionType, ParticipantRole, OperationType, MessageType
};
use crate::models::auth::AuthContext;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Collaboration session response
#[derive(Debug, Serialize)]
pub struct CollaborationSessionResponse {
    pub session: CollaborationSession,
    pub participants: Vec<SessionParticipant>,
}

/// Sessions list response
#[derive(Debug, Serialize)]
pub struct SessionsListResponse {
    pub sessions: Vec<CollaborationSession>,
    pub pagination: crate::models::PaginationInfo,
}

/// Session join request
#[derive(Debug, Deserialize)]
pub struct JoinSessionRequest {
    pub role: ParticipantRole,
    pub password: Option<String>,
}

/// Session operation request
#[derive(Debug, Deserialize)]
pub struct SessionOperationRequest {
    pub operation_type: OperationType,
    pub position: Option<i32>,
    pub content: Option<String>,
    pub length: Option<i32>,
    pub file_id: Option<Uuid>,
}

/// Session message request
#[derive(Debug, Deserialize)]
pub struct SessionMessageRequest {
    pub content: String,
    pub message_type: MessageType,
    pub reply_to: Option<Uuid>,
}

/// Session invitation request
#[derive(Debug, Deserialize)]
pub struct SessionInvitationRequest {
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
    pub role: ParticipantRole,
    pub message: Option<String>,
}

/// Session statistics response
#[derive(Debug, Serialize)]
pub struct SessionStatsResponse {
    pub stats: crate::models::collaboration::SessionStats,
}

/// Application state for collaboration handlers
#[derive(Clone)]
pub struct CollaborationState {
    pub db_pool: sqlx::PgPool,
    pub config: crate::config::Config,
}

/// List collaboration sessions
pub async fn list_sessions(
    State(state): State<crate::server::AppState>,
    Query(params): Query<crate::models::PaginationParams>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let sessions = CollaborationSession::list_for_user(&state.db_pool, auth_user.user_id, &params).await?;

    // Get total count for pagination
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(DISTINCT cs.id) FROM collaboration_sessions cs
        LEFT JOIN session_participants sp ON cs.id = sp.session_id
        WHERE cs.created_by = $1 OR sp.user_id = $1
        "#
    )
    .bind(auth_user.user_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let pagination_info = crate::models::PaginatedResponse::new(
        sessions.clone(),
        &params,
        total_count as u64,
    ).pagination;

    let response = SessionsListResponse {
        sessions,
        pagination: pagination_info,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Create a new collaboration session
pub async fn create_session(
    State(state): State<crate::server::AppState>,
    auth_user: axum::Extension<AuthContext>,
    Json(payload): Json<CreateCollaborationSession>,
) -> Result<impl IntoResponse, AppError> {
    let session = CollaborationSession::create(&state.db_pool, auth_user.user_id, payload).await?;
    let participants = SessionParticipant::get_active_participants(&state.db_pool, session.id).await?;

    let response = CollaborationSessionResponse {
        session,
        participants,
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": response
        })),
    ))
}

/// Get collaboration session details
pub async fn get_session(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    let session = CollaborationSession::find_by_id(&state.db_pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession".to_string(),
            id: session_id.to_string(),
        })?;

    // Check if user has access (is creator or participant)
    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;
    let has_access = session.created_by == auth_user.user_id ||
        participants.iter().any(|p| p.user_id == auth_user.user_id);

    if !has_access {
        return Err(AppError::Authorization(
            "Access denied to this collaboration session".to_string(),
        ));
    }

    let response = CollaborationSessionResponse {
        session,
        participants,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Update collaboration session
pub async fn update_session(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
    Json(payload): Json<UpdateCollaborationSession>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is session creator
    let session = CollaborationSession::find_by_id(&state.db_pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession".to_string(),
            id: session_id.to_string(),
        })?;

    if session.created_by != auth_user.user_id {
        return Err(AppError::Authorization(
            "Only session creators can update sessions".to_string(),
        ));
    }

    // Update session (simplified - would need proper implementation)
    // TODO: Implement session update logic in the model
    let updated_session = session.clone(); // Placeholder

    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;

    let response = CollaborationSessionResponse {
        session: updated_session,
        participants,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Delete collaboration session
pub async fn delete_session(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is session creator
    let session = CollaborationSession::find_by_id(&state.db_pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession".to_string(),
            id: session_id.to_string(),
        })?;

    if session.created_by != auth_user.user_id {
        return Err(AppError::Authorization(
            "Only session creators can delete sessions".to_string(),
        ));
    }

    // End session (soft delete)
    session.end(&state.db_pool).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Collaboration session deleted successfully"
    })))
}

/// Join collaboration session
pub async fn join_session(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
    Json(payload): Json<JoinSessionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let participant = SessionParticipant::join(
        &state.db_pool,
        session_id,
        auth_user.user_id,
        payload.role,
    )
    .await?;

    let updated_participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "participant": participant,
            "participants": updated_participants
        }
    })))
}

/// Leave collaboration session
pub async fn leave_session(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Find participant
    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;
    let participant = participants.iter()
        .find(|p| p.user_id == auth_user.user_id)
        .ok_or_else(|| AppError::NotFound {
            entity: "SessionParticipant".to_string(),
            id: session_id.to_string(),
        })?;

    participant.leave(&state.db_pool).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Left collaboration session successfully"
    })))
}

/// Get session participants
pub async fn get_participants(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user has access to session
    let session = CollaborationSession::find_by_id(&state.db_pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession".to_string(),
            id: session_id.to_string(),
        })?;

    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;
    let has_access = session.created_by == auth_user.user_id ||
        participants.iter().any(|p| p.user_id == auth_user.user_id);

    if !has_access {
        return Err(AppError::Authorization(
            "Access denied to this collaboration session".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "participants": participants
        }
    })))
}

/// Create operation in session
pub async fn create_operation(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
    Json(payload): Json<SessionOperationRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is participant
    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;
    if !participants.iter().any(|p| p.user_id == auth_user.user_id) {
        return Err(AppError::Authorization(
            "You must be a session participant to create operations".to_string(),
        ));
    }

    let operation_data = serde_json::json!({
        "position": payload.position,
        "content": payload.content,
        "length": payload.length,
    }).to_string();

    let operation = SessionOperation::create(
        &state.db_pool,
        session_id,
        auth_user.user_id,
        payload.operation_type,
        operation_data,
        payload.file_id,
        payload.position,
        payload.content,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "operation": operation
        }
    })))
}

/// Get session messages
pub async fn get_messages(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<crate::models::PaginationParams>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user has access to session
    let session = CollaborationSession::find_by_id(&state.db_pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession".to_string(),
            id: session_id.to_string(),
        })?;

    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;
    let has_access = session.created_by == auth_user.user_id ||
        participants.iter().any(|p| p.user_id == auth_user.user_id);

    if !has_access {
        return Err(AppError::Authorization(
            "Access denied to this collaboration session".to_string(),
        ));
    }

    let messages = sqlx::query_as::<_, SessionMessage>(
        r#"
        SELECT * FROM session_messages
        WHERE session_id = $1 AND deleted = false
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(session_id)
    .bind(params.limit() as i64)
    .bind(params.offset() as i64)
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "messages": messages
        }
    })))
}

/// Send message to session
pub async fn send_message(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
    Json(payload): Json<SessionMessageRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is participant
    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;
    if !participants.iter().any(|p| p.user_id == auth_user.user_id) {
        return Err(AppError::Authorization(
            "You must be a session participant to send messages".to_string(),
        ));
    }

    let message = sqlx::query_as::<_, SessionMessage>(
        r#"
        INSERT INTO session_messages (session_id, user_id, message_type, content, reply_to, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(session_id)
    .bind(auth_user.user_id)
    .bind(payload.message_type as MessageType)
    .bind(payload.content)
    .bind(payload.reply_to)
    .bind(chrono::Utc::now())
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "message": message
        }
    })))
}

/// Invite participant to session
pub async fn invite_participant(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
    Json(payload): Json<SessionInvitationRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is session creator
    let session = CollaborationSession::find_by_id(&state.db_pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession".to_string(),
            id: session_id.to_string(),
        })?;

    if session.created_by != auth_user.user_id {
        return Err(AppError::Authorization(
            "Only session creators can invite participants".to_string(),
        ));
    }

    // Create invitation (simplified implementation)
    let invitation = SessionInvitation {
        id: Uuid::new_v4(),
        session_id,
        invited_by: auth_user.user_id,
        invited_user: payload.user_id,
        email: payload.email,
        role: payload.role,
        message: payload.message,
        token: Uuid::new_v4().to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        accepted: false,
        accepted_at: None,
        declined: false,
        declined_at: None,
        created_at: chrono::Utc::now(),
    };

    // TODO: Save invitation to database and send notification

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "invitation": invitation
        }
    })))
}

/// Get session statistics
pub async fn get_session_stats(
    State(state): State<crate::server::AppState>,
    Path(session_id): Path<Uuid>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user has access to session
    let session = CollaborationSession::find_by_id(&state.db_pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession".to_string(),
            id: session_id.to_string(),
        })?;

    let participants = SessionParticipant::get_active_participants(&state.db_pool, session_id).await?;
    let has_access = session.created_by == auth_user.user_id ||
        participants.iter().any(|p| p.user_id == auth_user.user_id);

    if !has_access {
        return Err(AppError::Authorization(
            "Access denied to this collaboration session".to_string(),
        ));
    }

    let stats = crate::models::collaboration::SessionStats::get(&state.db_pool, session_id).await?;

    let response = SessionStatsResponse {
        stats,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

/// Get invitation details
pub async fn get_invitation(
    State(state): State<crate::server::AppState>,
    Path(token): Path<String>,
    _auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Implement invitation lookup from database
    Err::<Response, AppError>(AppError::NotFound {
        entity: "SessionInvitation".to_string(),
        id: token,
    })
}

/// Accept invitation
pub async fn accept_invitation(
    State(state): State<crate::server::AppState>,
    Path(token): Path<String>,
    auth_user: axum::Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Implement invitation acceptance logic
    Err::<Response, AppError>(AppError::NotFound {
        entity: "SessionInvitation".to_string(),
        id: token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[tokio::test]
    async fn test_session_creation() {
        let state = crate::handlers::collaboration::CollaborationState {
            db_pool: PgPool::connect("postgresql://test").await.unwrap(),
            config: crate::config::Config::load().unwrap(),
        };

        let request = CreateCollaborationSession {
            title: Some("Test Session".to_string()),
            description: Some("A test collaboration session".to_string()),
            session_type: Some(SessionType::Realtime),
            file_id: None,
            max_participants: Some(5),
            password: None,
            settings: None,
        };

        // This test would require setting up proper auth context
        // For now, we just verify the request structure
        assert_eq!(request.title, Some("Test Session".to_string()));
        assert_eq!(request.session_type, Some(SessionType::Realtime));
        assert_eq!(request.max_participants, Some(5));
    }

    #[test]
    fn test_join_session_request() {
        let request = JoinSessionRequest {
            role: ParticipantRole::Editor,
            password: Some("password123".to_string()),
        };

        assert_eq!(request.role, ParticipantRole::Editor);
        assert_eq!(request.password, Some("password123".to_string()));
    }

    #[test]
    fn test_operation_request() {
        let request = SessionOperationRequest {
            operation_type: OperationType::Insert,
            position: Some(100),
            content: Some("Hello World".to_string()),
            length: Some(11),
            file_id: Some(uuid::Uuid::new_v4()),
        };

        assert_eq!(request.operation_type, OperationType::Insert);
        assert_eq!(request.position, Some(100));
        assert_eq!(request.content, Some("Hello World".to_string()));
    }
}
