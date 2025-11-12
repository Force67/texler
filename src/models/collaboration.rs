//! Collaboration models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::{Entity, UserRole};

/// Collaboration session
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CollaborationSession {
    pub id: Uuid,
    pub project_id: Uuid,
    pub file_id: Option<Uuid>,
    pub created_by: Uuid,
    pub session_type: SessionType,
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub max_participants: i32,
    pub password_hash: Option<String>,
    pub settings: Option<String>, // JSON field
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for CollaborationSession {
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

/// Session type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum SessionType {
    #[serde(rename = "realtime")]
    #[sqlx(type_name = "text")]
    Realtime,
    #[serde(rename = "review")]
    #[sqlx(type_name = "text")]
    Review,
    #[serde(rename = "tutorial")]
    #[sqlx(type_name = "text")]
    Tutorial,
    #[serde(rename = "meeting")]
    #[sqlx(type_name = "text")]
    Meeting,
}

impl Default for SessionType {
    fn default() -> Self {
        Self::Realtime
    }
}

/// Session participant
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionParticipant {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub role: ParticipantRole,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub cursor_position: Option<i32>,
    pub selection: Option<String>, // JSON field
    pub is_online: bool,
    pub last_seen_at: DateTime<Utc>,
    pub permissions: Option<String>, // JSON field
}

impl Entity for SessionParticipant {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.joined_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.last_seen_at
    }
}

/// Participant role in session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum ParticipantRole {
    #[serde(rename = "host")]
    #[sqlx(type_name = "text")]
    Host,
    #[serde(rename = "presenter")]
    #[sqlx(type_name = "text")]
    Presenter,
    #[serde(rename = "editor")]
    #[sqlx(type_name = "text")]
    Editor,
    #[serde(rename = "viewer")]
    #[sqlx(type_name = "text")]
    Viewer,
}

impl Default for ParticipantRole {
    fn default() -> Self {
        Self::Viewer
    }
}

/// Session operation/changes
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionOperation {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub operation_type: OperationType,
    pub operation_data: String, // JSON field
    pub file_id: Option<Uuid>,
    pub position: Option<i32>,
    pub length: Option<i32>,
    pub content: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub applied: bool,
    pub applied_at: Option<DateTime<Utc>>,
    pub rejected: bool,
    pub rejected_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
}

impl Entity for SessionOperation {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// Operation type for collaborative editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum OperationType {
    #[serde(rename = "insert")]
    #[sqlx(type_name = "text")]
    Insert,
    #[serde(rename = "delete")]
    #[sqlx(type_name = "text")]
    Delete,
    #[serde(rename = "replace")]
    #[sqlx(type_name = "text")]
    Replace,
    #[serde(rename = "format")]
    #[sqlx(type_name = "text")]
    Format,
    #[serde(rename = "cursor")]
    #[sqlx(type_name = "text")]
    Cursor,
    #[serde(rename = "selection")]
    #[sqlx(type_name = "text")]
    Selection,
}

/// Session chat message
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub message_type: MessageType,
    pub content: String,
    pub reply_to: Option<Uuid>,
    pub reactions: Option<String>, // JSON field
    pub edited: bool,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Entity for SessionMessage {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.edited_at.unwrap_or(self.created_at)
    }
}

/// Message type in session chat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum MessageType {
    #[serde(rename = "text")]
    #[sqlx(type_name = "text")]
    Text,
    #[serde(rename = "system")]
    #[sqlx(type_name = "text")]
    System,
    #[serde(rename = "file")]
    #[sqlx(type_name = "text")]
    File,
    #[serde(rename = "code")]
    #[sqlx(type_name = "text")]
    Code,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Text
    }
}

/// Session invitation
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionInvitation {
    pub id: Uuid,
    pub session_id: Uuid,
    pub invited_by: Uuid,
    pub invited_user: Option<Uuid>,
    pub email: Option<String>,
    pub role: ParticipantRole,
    pub message: Option<String>,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub accepted: bool,
    pub accepted_at: Option<DateTime<Utc>>,
    pub declined: bool,
    pub declined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Entity for SessionInvitation {
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

/// Session recording
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionRecording {
    pub id: Uuid,
    pub session_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub file_path: String,
    pub file_size: i64,
    pub format: String, // "webm", "mp4", etc.
    pub quality: String,
    pub created_at: DateTime<Utc>,
}

impl Entity for SessionRecording {
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

/// Creation request for collaboration session
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCollaborationSession {
    pub title: Option<String>,
    pub description: Option<String>,
    pub session_type: Option<SessionType>,
    pub file_id: Option<Uuid>,
    pub max_participants: Option<i32>,
    pub password: Option<String>,
    pub settings: Option<String>,
}

/// Update request for collaboration session
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCollaborationSession {
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub max_participants: Option<i32>,
    pub password: Option<String>,
    pub settings: Option<String>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    pub session_id: Uuid,
    pub total_participants: i64,
    pub current_participants: i64,
    pub total_operations: i64,
    pub total_messages: i64,
    pub duration_minutes: i64,
    pub peak_participants: i64,
    pub files_edited: i64,
    pub total_characters_typed: i64,
}

impl CollaborationSession {
    /// Create a new collaboration session
    pub async fn create(
        db: &sqlx::PgPool,
        created_by: Uuid,
        create_session: CreateCollaborationSession,
    ) -> Result<Self, crate::error::AppError> {
        let password_hash = if let Some(password) = &create_session.password {
            Some(bcrypt::hash(password, bcrypt::DEFAULT_COST)?)
        } else {
            None
        };

        let session = sqlx::query_as!(
            CollaborationSession,
            r#"
            INSERT INTO collaboration_sessions (
                project_id, file_id, created_by, session_type, title, description,
                is_active, max_participants, password_hash, settings
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
            // TODO: Add project_id to create_session
            Uuid::new_v4(), // Temporary - should come from create_session
            create_session.file_id,
            created_by,
            create_session.session_type.unwrap_or_default() as SessionType,
            create_session.title,
            create_session.description,
            true,
            create_session.max_participants.unwrap_or(10),
            password_hash,
            create_session.settings
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(session)
    }

    /// Find session by ID
    pub async fn find_by_id(
        db: &sqlx::PgPool,
        session_id: Uuid,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let session = sqlx::query_as!(
            CollaborationSession,
            "SELECT * FROM collaboration_sessions WHERE id = $1",
            session_id
        )
        .fetch_optional(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(session)
    }

    /// Get session with access control
    pub async fn find_with_access(
        db: &sqlx::PgPool,
        session_id: Uuid,
        user_id: Uuid,
        password: Option<&str>,
    ) -> Result<Option<Self>, crate::error::AppError> {
        let session = Self::find_by_id(db, session_id).await?;

        if let Some(session) = session {
            // Check if session is active
            if !session.is_active {
                return Ok(None);
            }

            // Check password protection
            if let (Some(session_password), Some(provided_password)) = (&session.password_hash, password) {
                if !bcrypt::verify(provided_password, session_password).unwrap_or(false) {
                    return Ok(None);
                }
            } else if session.password_hash.is_some() && password.is_none() {
                return Ok(None);
            }

            // TODO: Add additional access control logic
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    /// List sessions for a user
    pub async fn list_for_user(
        db: &sqlx::PgPool,
        user_id: Uuid,
        params: &super::PaginationParams,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let sessions = sqlx::query_as!(
            CollaborationSession,
            r#"
            SELECT DISTINCT cs.* FROM collaboration_sessions cs
            LEFT JOIN session_participants sp ON cs.id = sp.session_id
            WHERE cs.created_by = $1 OR sp.user_id = $1
            ORDER BY cs.updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            params.limit() as i64,
            params.offset() as i64
        )
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(sessions)
    }

    /// Start session
    pub async fn start(&self, db: &sqlx::PgPool) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE collaboration_sessions SET started_at = NOW() WHERE id = $1",
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// End session
    pub async fn end(&self, db: &sqlx::PgPool) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE collaboration_sessions SET is_active = false, ended_at = NOW() WHERE id = $1",
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }
}

impl SessionParticipant {
    /// Add participant to session
    pub async fn join(
        db: &sqlx::PgPool,
        session_id: Uuid,
        user_id: Uuid,
        role: ParticipantRole,
    ) -> Result<Self, crate::error::AppError> {
        let participant = sqlx::query_as!(
            SessionParticipant,
            r#"
            INSERT INTO session_participants (session_id, user_id, role, is_online, last_seen_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
            session_id,
            user_id,
            role as ParticipantRole,
            true,
            Utc::now()
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(participant)
    }

    /// Leave session
    pub async fn leave(
        &self,
        db: &sqlx::PgPool,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE session_participants SET is_online = false, left_at = NOW() WHERE id = $1",
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Update online status
    pub async fn update_online_status(
        &self,
        db: &sqlx::PgPool,
        is_online: bool,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE session_participants SET is_online = $1, last_seen_at = NOW() WHERE id = $2",
            is_online,
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Update cursor position
    pub async fn update_cursor(
        &self,
        db: &sqlx::PgPool,
        position: Option<i32>,
        selection: Option<String>,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE session_participants SET cursor_position = $1, selection = $2 WHERE id = $3",
            position,
            selection,
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Get active participants for session
    pub async fn get_active_participants(
        db: &sqlx::PgPool,
        session_id: Uuid,
    ) -> Result<Vec<Self>, crate::error::AppError> {
        let participants = sqlx::query_as!(
            SessionParticipant,
            "SELECT * FROM session_participants WHERE session_id = $1 AND is_online = true",
            session_id
        )
        .fetch_all(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(participants)
    }
}

impl SessionOperation {
    /// Create operation
    pub async fn create(
        db: &sqlx::PgPool,
        session_id: Uuid,
        user_id: Uuid,
        operation_type: OperationType,
        operation_data: String,
        file_id: Option<Uuid>,
        position: Option<i32>,
        content: Option<String>,
    ) -> Result<Self, crate::error::AppError> {
        let operation = sqlx::query_as!(
            SessionOperation,
            r#"
            INSERT INTO session_operations (
                session_id, user_id, operation_type, operation_data,
                file_id, position, content, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            session_id,
            user_id,
            operation_type as OperationType,
            operation_data,
            file_id,
            position,
            content,
            Utc::now()
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(operation)
    }

    /// Apply operation
    pub async fn apply(&self, db: &sqlx::PgPool) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE session_operations SET applied = true, applied_at = NOW() WHERE id = $1",
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }

    /// Reject operation
    pub async fn reject(
        &self,
        db: &sqlx::PgPool,
        reason: Option<String>,
    ) -> Result<(), crate::error::AppError> {
        sqlx::query!(
            "UPDATE session_operations SET rejected = true, rejected_at = NOW(), rejection_reason = $1 WHERE id = $2",
            reason,
            self.id
        )
        .execute(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(())
    }
}

impl SessionStats {
    /// Get session statistics
    pub async fn get(
        db: &sqlx::PgPool,
        session_id: Uuid,
    ) -> Result<Self, crate::error::AppError> {
        let stats = sqlx::query_as!(
            SessionStats,
            r#"
            WITH participant_stats AS (
                SELECT
                    COUNT(*) as total_participants,
                    COUNT(*) FILTER (WHERE is_online = true) as current_participants,
                    MAX(participant_count) as peak_participants
                FROM session_participants sp
                LEFT JOIN (
                    SELECT session_id, COUNT(*) as participant_count
                    FROM session_participants
                    WHERE is_online = true
                    GROUP BY session_id
                ) pc ON sp.session_id = pc.session_id
                WHERE sp.session_id = $1
            ),
            operation_stats AS (
                SELECT
                    COUNT(*) as total_operations,
                    COALESCE(SUM(LENGTH(content)), 0) as total_characters_typed,
                    COUNT(DISTINCT file_id) as files_edited
                FROM session_operations
                WHERE session_id = $1 AND applied = true
            ),
            message_stats AS (
                SELECT COUNT(*) as total_messages
                FROM session_messages
                WHERE session_id = $1 AND deleted = false
            ),
            session_info AS (
                SELECT
                    EXTRACT(EPOCH FROM (COALESCE(ended_at, NOW()) - started_at)) / 60 as duration_minutes
                FROM collaboration_sessions
                WHERE id = $1
            )
            SELECT
                $1 as session_id,
                COALESCE(ps.total_participants, 0) as total_participants,
                COALESCE(ps.current_participants, 0) as current_participants,
                COALESCE(os.total_operations, 0) as total_operations,
                COALESCE(ms.total_messages, 0) as total_messages,
                COALESCE(si.duration_minutes, 0)::bigint as duration_minutes,
                COALESCE(ps.peak_participants, 0) as peak_participants,
                COALESCE(os.files_edited, 0) as files_edited,
                COALESCE(os.total_characters_typed, 0) as total_characters_typed
            FROM participant_stats ps
            CROSS JOIN operation_stats os
            CROSS JOIN message_stats ms
            CROSS JOIN session_info si
            "#,
            session_id
        )
        .fetch_one(db)
        .await
        .map_err(crate::error::AppError::Database)?;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_type_default() {
        assert_eq!(SessionType::default(), SessionType::Realtime);
    }

    #[test]
    fn test_participant_role_default() {
        assert_eq!(ParticipantRole::default(), ParticipantRole::Viewer);
    }

    #[test]
    fn test_message_type_default() {
        assert_eq!(MessageType::default(), MessageType::Text);
    }
}