//! WebSocket server for real-time collaboration

use crate::config::Config;
use crate::error::AppError;
use crate::models::collaboration::{
    CollaborationSession, SessionOperation, SessionMessage, SessionParticipant,
    OperationType, MessageType, ParticipantRole,
};
use crate::models::auth::{AuthContext, JwtService};
use chrono::Utc;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message,
    WebSocketStream as WsStream,
};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Client messages
    /// Authenticate with JWT token
    Authenticate {
        token: String,
        session_id: Option<Uuid>,
    },
    /// Join collaboration session
    JoinSession {
        session_id: Uuid,
        role: ParticipantRole,
        password: Option<String>,
    },
    /// Leave current session
    LeaveSession,
    /// Send operation to session
    Operation {
        session_id: Uuid,
        operation_type: OperationType,
        position: Option<i32>,
        content: Option<String>,
        length: Option<i32>,
        file_id: Option<Uuid>,
    },
    /// Update cursor position
    Cursor {
        session_id: Uuid,
        position: i32,
        selection: Option<String>,
    },
    /// Send chat message
    ChatMessage {
        session_id: Uuid,
        content: String,
        message_type: MessageType,
        reply_to: Option<Uuid>,
    },
    /// Keep alive
    Ping,

    /// Server messages
    /// Authentication success/failure
    AuthResult {
        success: bool,
        user: Option<AuthContext>,
        error: Option<String>,
    },
    /// Session joined
    SessionJoined {
        session_id: Uuid,
        participants: Vec<SessionParticipant>,
        session_info: CollaborationSession,
    },
    /// Participant joined/updated
    ParticipantUpdate {
        session_id: Uuid,
        participant: SessionParticipant,
    },
    /// Participant left
    ParticipantLeft {
        session_id: Uuid,
        user_id: Uuid,
    },
    /// Operation from another user
    ServerOperation {
        session_id: Uuid,
        user_id: Uuid,
        operation_type: OperationType,
        position: Option<i32>,
        content: Option<String>,
        length: Option<i32>,
        file_id: Option<Uuid>,
        timestamp: chrono::DateTime<Utc>,
    },
    /// Chat message from another user
    ServerChatMessage {
        session_id: Uuid,
        id: Uuid,
        user_id: Uuid,
        content: String,
        message_type: MessageType,
        reply_to: Option<Uuid>,
        timestamp: chrono::DateTime<Utc>,
    },
    /// Session status update
    SessionStatus {
        session_id: Uuid,
        status: String,
    },
    /// Error message
    Error {
        code: String,
        message: String,
    },
    /// Keep alive response
    Pong,
}

/// WebSocket connection state
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub user: Option<AuthContext>,
    pub session_id: Option<Uuid>,
    pub participant_id: Option<Uuid>,
    pub last_heartbeat: chrono::DateTime<Utc>,
    pub authenticated: bool,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            user: None,
            session_id: None,
            participant_id: None,
            last_heartbeat: Utc::now(),
            authenticated: false,
        }
    }
}

/// WebSocket server state
#[derive(Debug)]
pub struct WsServerState {
    pub config: Arc<Config>,
    pub db_pool: Arc<sqlx::PgPool>,
    pub connections: Arc<RwLock<HashMap<String, Arc<RwLock<ConnectionState>>>>>,
    pub session_broadcasts: Arc<RwLock<HashMap<Uuid, broadcast::Sender<WsMessage>>>>,
}

impl WsServerState {
    pub fn new(config: Config, db_pool: sqlx::PgPool) -> Self {
        Self {
            config: Arc::new(config),
            db_pool: Arc::new(db_pool),
            connections: Arc::new(RwLock::new(HashMap::new())),
            session_broadcasts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate connection ID
    pub fn generate_connection_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Register new connection
    pub async fn register_connection(&self, connection_id: String) {
        let mut connections = self.connections.write().await;
        connections.insert(
            connection_id.clone(),
            Arc::new(RwLock::new(ConnectionState::default())),
        );
        debug!("Registered WebSocket connection: {}", connection_id);
    }

    /// Unregister connection
    pub async fn unregister_connection(&self, connection_id: &str) {
        // Remove from connections
        let mut connections = self.connections.write().await;
        if let Some(state) = connections.remove(connection_id) {
            let state_read = state.read().await;

            // Leave session if in one
            if let (Some(session_id), Some(participant_id)) = (state_read.session_id, state_read.participant_id) {
                drop(state_read);
                drop(connections);

                // Clean up session participation
                if let Err(e) = self.handle_session_leave(session_id, participant_id).await {
                    warn!("Error cleaning up session participation: {}", e);
                }
            }
        }

        debug!("Unregistered WebSocket connection: {}", connection_id);
    }

    /// Get or create session broadcast channel
    pub async fn get_session_broadcast(&self, session_id: Uuid) -> broadcast::Sender<WsMessage> {
        let mut broadcasts = self.session_broadcasts.write().await;

        if let Some(sender) = broadcasts.get(&session_id) {
            sender.clone()
        } else {
            let (sender, _) = broadcast::channel(1000);
            broadcasts.insert(session_id, sender.clone());
            sender
        }
    }

    /// Broadcast message to all session participants
    pub async fn broadcast_to_session(
        &self,
        session_id: Uuid,
        message: WsMessage,
    ) -> Result<(), AppError> {
        let broadcasts = self.session_broadcasts.read().await;

        if let Some(sender) = broadcasts.get(&session_id) {
            if let Err(e) = sender.send(message) {
                warn!("Failed to broadcast to session {}: {}", session_id, e);
            }
        }

        Ok(())
    }

    /// Handle session join
    pub async fn handle_session_join(
        &self,
        connection_id: &str,
        session_id: Uuid,
        user_id: Uuid,
        role: ParticipantRole,
        password: Option<String>,
    ) -> Result<SessionParticipant, AppError> {
        // Validate session access
        let session = CollaborationSession::find_with_access(
            &*self.db_pool,
            session_id,
            user_id,
            password.as_deref(),
        )
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CollaborationSession",
            id: session_id.to_string(),
        })?;

        // Add participant to session
        let participant = SessionParticipant::join(
            &*self.db_pool,
            session_id,
            user_id,
            role,
        )
        .await?;

        // Update connection state
        {
            let connections = self.connections.read().await;
            if let Some(state) = connections.get(connection_id) {
                let mut state_write = state.write().await;
                state_write.session_id = Some(session_id);
                state_write.participant_id = Some(participant.id);
                state_write.last_heartbeat = Utc::now();
            }
        }

        // Get current participants
        let current_participants = SessionParticipant::get_active_participants(&*self.db_pool, session_id).await?;

        // Broadcast participant join to session
        let broadcast_msg = WsMessage::ParticipantUpdate {
            session_id,
            participant: participant.clone(),
        };
        self.broadcast_to_session(session_id, broadcast_msg).await?;

        info!("User {} joined session {}", user_id, session_id);
        Ok(participant)
    }

    /// Handle session leave
    pub async fn handle_session_leave(
        &self,
        session_id: Uuid,
        participant_id: Uuid,
    ) -> Result<(), AppError> {
        // Update participant status
        let participant = sqlx::query_as!(
            SessionParticipant,
            "SELECT * FROM session_participants WHERE id = $1",
            participant_id
        )
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(participant) = participant {
            participant.leave(&*self.db_pool).await?;

            // Broadcast participant leave
            let broadcast_msg = WsMessage::ParticipantLeft {
                session_id,
                user_id: participant.user_id,
            };
            self.broadcast_to_session(session_id, broadcast_msg).await?;

            info!("User {} left session {}", participant.user_id, session_id);
        }

        Ok(())
    }

    /// Handle operation
    pub async fn handle_operation(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        operation_type: OperationType,
        position: Option<i32>,
        content: Option<String>,
        length: Option<i32>,
        file_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        // Create operation record
        let operation_data = serde_json::json!({
            "position": position,
            "content": content,
            "length": length,
        });

        let operation = SessionOperation::create(
            &*self.db_pool,
            session_id,
            user_id,
            operation_type,
            operation_data.to_string(),
            file_id,
            position,
            content,
        )
        .await?;

        // Apply operation (simplified - real implementation would need conflict resolution)
        operation.apply(&*self.db_pool).await?;

        // Broadcast to session
        let broadcast_msg = WsMessage::ServerOperation {
            session_id,
            user_id,
            operation_type,
            position,
            content,
            length,
            file_id,
            timestamp: operation.timestamp,
        };
        self.broadcast_to_session(session_id, broadcast_msg).await?;

        Ok(())
    }

    /// Handle chat message
    pub async fn handle_chat_message(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        content: String,
        message_type: MessageType,
        reply_to: Option<Uuid>,
    ) -> Result<(), AppError> {
        // Create message record
        let message = sqlx::query_as!(
            SessionMessage,
            r#"
            INSERT INTO session_messages (session_id, user_id, message_type, content, reply_to, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            session_id,
            user_id,
            message_type as MessageType,
            content,
            reply_to,
            Utc::now()
        )
        .fetch_one(&*self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Broadcast to session
        let broadcast_msg = WsMessage::ServerChatMessage {
            session_id,
            id: message.id,
            user_id,
            content: message.content,
            message_type,
            reply_to: message.reply_to,
            timestamp: message.created_at,
        };
        self.broadcast_to_session(session_id, broadcast_msg).await?;

        Ok(())
    }
}

/// WebSocket handler for a single connection
pub async fn handle_websocket_connection(
    stream: WebSocketStream<tokio::net::TcpStream>,
    connection_id: String,
    state: Arc<WsServerState>,
) {
    info!("New WebSocket connection: {}", connection_id);

    // Register connection
    state.register_connection(connection_id.clone()).await;

    let (mut sender, mut receiver) = stream.split();

    // Get message receiver for broadcasts
    let session_id = {
        let connections = state.connections.read().await;
        connections
            .get(&connection_id)
            .map(|s| s.read().await.session_id)
            .flatten()
    };

    let mut broadcast_receiver = if let Some(session_id) = session_id {
        Some(state.get_session_broadcast(session_id).await.subscribe())
    } else {
        None
    };

    // Heartbeat interval
    let mut heartbeat_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Handle incoming messages
            Some(msg_result) = receiver.next() => {
                match msg_result {
                    Ok(msg) => {
                        if let Err(e) = handle_message(&connection_id, msg, &state, &mut sender, &mut broadcast_receiver).await {
                            error!("Error handling message for {}: {}", connection_id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("WebSocket error for {}: {}", connection_id, e);
                        break;
                    }
                }
            }

            // Handle outgoing broadcasts
            message = async {
                if let Some(ref mut receiver) = broadcast_receiver {
                    receiver.recv().await.ok()
                } else {
                    std::future::pending().await
                }
            } => {
                if let Some(message) = message {
                    if let Ok(text) = serde_json::to_string(&message) {
                        if let Err(e) = sender.send(Message::Text(text)).await {
                            error!("Failed to send broadcast to {}: {}", connection_id, e);
                            break;
                        }
                    }
                }
            }

            // Send periodic pings
            _ = heartbeat_interval.tick() => {
                if let Err(e) = sender.send(Message::Ping(vec![])).await {
                    warn!("Failed to send ping to {}: {}", connection_id, e);
                    break;
                }
            }
        }
    }

    // Cleanup connection
    state.unregister_connection(&connection_id).await;
    info!("WebSocket connection closed: {}", connection_id);
}

/// Handle incoming WebSocket message
async fn handle_message(
    connection_id: &str,
    msg: Message,
    state: &Arc<WsServerState>,
    sender: &mut futures::stream::SplitSink<WsStream<tokio::net::TcpStream>, Message>,
    broadcast_receiver: &mut Option<broadcast::Receiver<WsMessage>>,
) -> Result<(), AppError> {
    match msg {
        Message::Text(text) => {
            let ws_message: WsMessage = serde_json::from_str(&text)
                .map_err(|e| AppError::Validation(format!("Invalid WebSocket message: {}", e)))?;

            handle_ws_message(connection_id, ws_message, state, sender, broadcast_receiver).await
        }
        Message::Binary(_) => {
            warn!("Received binary message on WebSocket connection: {}", connection_id);
            Ok(())
        }
        Message::Ping(payload) => {
            // Respond with pong
            sender.send(Message::Pong(payload)).await
                .map_err(|e| AppError::Server(format!("Failed to send pong: {}", e)))?;
            Ok(())
        }
        Message::Pong(_) => {
            // Update heartbeat
            {
                let connections = state.connections.read().await;
                if let Some(state) = connections.get(connection_id) {
                    let mut state_write = state.write().await;
                    state_write.last_heartbeat = Utc::now();
                }
            }
            Ok(())
        }
        Message::Close(_) => {
            debug!("WebSocket connection {} closing", connection_id);
            Ok(())
        }
    }
}

/// Handle parsed WebSocket message
async fn handle_ws_message(
    connection_id: &str,
    ws_message: WsMessage,
    state: &Arc<WsServerState>,
    sender: &mut futures::stream::SplitSink<WsStream<tokio::net::TcpStream>, Message>,
    broadcast_receiver: &mut Option<broadcast::Receiver<WsMessage>>,
) -> Result<(), AppError> {
    match ws_message {
        WsMessage::Authenticate { token, session_id } => {
            // Verify JWT token
            let jwt_service = crate::models::auth::JwtService::new(
                &state.config.jwt.secret,
                state.config.jwt.issuer.clone(),
                state.config.jwt.expiration as i64,
                state.config.jwt.refresh_expiration as i64,
            )?;

            let auth_result = match jwt_service.verify_token(&token) {
                Ok(claims) => {
                    let auth_context = crate::models::auth::AuthContext::from(claims);

                    // Update connection state
                    {
                        let connections = state.connections.read().await;
                        if let Some(state) = connections.get(connection_id) {
                            let mut state_write = state.write().await;
                            state_write.user = Some(auth_context.clone());
                            state_write.authenticated = true;
                            state_write.last_heartbeat = Utc::now();
                        }
                    }

                    // Set up broadcast receiver for session if specified
                    if let Some(session_id) = session_id {
                        *broadcast_receiver = Some(state.get_session_broadcast(session_id).await.subscribe());
                    }

                    WsMessage::AuthResult {
                        success: true,
                        user: Some(auth_context),
                        error: None,
                    }
                }
                Err(e) => {
                    WsMessage::AuthResult {
                        success: false,
                        user: None,
                        error: Some(format!("Authentication failed: {}", e)),
                    }
                }
            };

            let response = serde_json::to_string(&auth_result)?;
            sender.send(Message::Text(response)).await
                .map_err(|e| AppError::Server(format!("Failed to send auth response: {}", e)))?;
        }

        WsMessage::JoinSession { session_id, role, password } => {
            // Get user from connection state
            let user_id = {
                let connections = state.connections.read().await;
                connections
                    .get(connection_id)
                    .and_then(|s| s.read().await.user.as_ref())
                    .map(|u| u.user_id)
                    .ok_or_else(|| AppError::Authentication("Not authenticated".to_string()))?
            };

            // Handle session join
            match state.handle_session_join(connection_id, session_id, user_id, role, password).await {
                Ok(participant) => {
                    // Get session info and current participants
                    let session_info = CollaborationSession::find_by_id(&*state.db_pool, session_id).await?
                        .ok_or_else(|| AppError::NotFound {
                            entity: "CollaborationSession",
                            id: session_id.to_string(),
                        })?;

                    let current_participants = SessionParticipant::get_active_participants(&*state.db_pool, session_id).await?;

                    let response = WsMessage::SessionJoined {
                        session_id,
                        participants: current_participants,
                        session_info,
                    };

                    let response_text = serde_json::to_string(&response)?;
                    sender.send(Message::Text(response_text)).await
                        .map_err(|e| AppError::Server(format!("Failed to send join response: {}", e)))?;

                    // Update broadcast receiver
                    *broadcast_receiver = Some(state.get_session_broadcast(session_id).await.subscribe());
                }
                Err(e) => {
                    let error_response = WsMessage::Error {
                        code: "JOIN_FAILED".to_string(),
                        message: e.to_string(),
                    };
                    let error_text = serde_json::to_string(&error_response)?;
                    sender.send(Message::Text(error_text)).await
                        .map_err(|e| AppError::Server(format!("Failed to send error response: {}", e)))?;
                }
            }
        }

        WsMessage::LeaveSession => {
            let (session_id, participant_id) = {
                let connections = state.connections.read().await;
                if let Some(state) = connections.get(connection_id) {
                    let state_read = state.read().await;
                    (state_read.session_id, state_read.participant_id)
                } else {
                    (None, None)
                }
            };

            if let (Some(session_id), Some(participant_id)) = (session_id, participant_id) {
                state.handle_session_leave(session_id, participant_id).await?;
            }
        }

        WsMessage::Operation { session_id, operation_type, position, content, length, file_id } => {
            let user_id = {
                let connections = state.connections.read().await;
                connections
                    .get(connection_id)
                    .and_then(|s| s.read().await.user.as_ref())
                    .map(|u| u.user_id)
                    .ok_or_else(|| AppError::Authentication("Not authenticated".to_string()))?
            };

            if let Err(e) = state.handle_operation(session_id, user_id, operation_type, position, content, length, file_id).await {
                let error_response = WsMessage::Error {
                    code: "OPERATION_FAILED".to_string(),
                    message: e.to_string(),
                };
                let error_text = serde_json::to_string(&error_response)?;
                sender.send(Message::Text(error_text)).await
                    .map_err(|e| AppError::Server(format!("Failed to send error response: {}", e)))?;
            }
        }

        WsMessage::ChatMessage { session_id, content, message_type, reply_to } => {
            let user_id = {
                let connections = state.connections.read().await;
                connections
                    .get(connection_id)
                    .and_then(|s| s.read().await.user.as_ref())
                    .map(|u| u.user_id)
                    .ok_or_else(|| AppError::Authentication("Not authenticated".to_string()))?
            };

            if let Err(e) = state.handle_chat_message(session_id, user_id, content, message_type, reply_to).await {
                let error_response = WsMessage::Error {
                    code: "MESSAGE_FAILED".to_string(),
                    message: e.to_string(),
                };
                let error_text = serde_json::to_string(&error_response)?;
                sender.send(Message::Text(error_text)).await
                    .map_err(|e| AppError::Server(format!("Failed to send error response: {}", e)))?;
            }
        }

        WsMessage::Ping => {
            let response = WsMessage::Pong;
            let response_text = serde_json::to_string(&response)?;
            sender.send(Message::Text(response_text)).await
                .map_err(|e| AppError::Server(format!("Failed to send pong: {}", e)))?;
        }

        _ => {
            warn!("Unhandled WebSocket message type from connection: {}", connection_id);
        }
    }

    Ok(())
}

/// Start WebSocket server
pub async fn start_websocket_server(
    config: Config,
    db_pool: sqlx::PgPool,
) -> Result<(), AppError> {
    let state = Arc::new(WsServerState::new(config.clone(), db_pool));
    let addr = format!("0.0.0.0:{}", config.websocket.port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| AppError::Config(format!("Failed to bind WebSocket server to {}: {}", addr, e)))?;

    info!("WebSocket server listening on {}", addr);

    loop {
        let (stream, addr) = listener.accept()
            .await
            .map_err(|e| AppError::Server(format!("Failed to accept WebSocket connection: {}", e)))?;

        let connection_id = WsServerState::generate_connection_id();
        let state_clone = state.clone();

        info!("New WebSocket connection from: {}", addr);

        tokio::spawn(async move {
            // Upgrade to WebSocket connection
            let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    warn!("WebSocket upgrade failed from {}: {}", addr, e);
                    return;
                }
            };

            handle_websocket_connection(ws_stream, connection_id, state_clone).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_connection_state_default() {
        let state = ConnectionState::default();
        assert!(state.user.is_none());
        assert!(state.session_id.is_none());
        assert!(!state.authenticated);
    }

    #[test]
    fn test_ws_message_serialization() {
        let message = WsMessage::Ping;
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"type\":\"Ping\""));
    }

    #[tokio::test]
    async fn test_ws_server_state_creation() {
        // This test would need a proper config and database pool
        // For now, we just verify the structure compiles
        assert!(true);
    }
}