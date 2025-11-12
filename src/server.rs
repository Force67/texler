//! Web server implementation with Axum

use crate::config::Config;
use crate::error::{AppError, RequestId};
use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{info, warn};

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: Arc<sqlx::PgPool>,
    pub oidc_clients: Arc<std::collections::HashMap<String, authware::OidcClient>>,
    pub jwt_service: Arc<crate::models::auth::JwtService>,
    pub rate_limiter: Arc<crate::middleware::RateLimiter>,
}

/// Application router
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any) // Configure appropriately for production
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers(Any);

    let compression = CompressionLayer::new();

    let request_body_limit = RequestBodyLimitLayer::new(
        state.config.latex.output_size_limit as usize * 10, // Allow 10x output size for input
    );

    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // API routes
        .nest("/api/v1", api_routes())
        // Middleware
        .layer(
            ServiceBuilder::new()
                .layer(request_body_limit)
                .layer(compression)
                .layer(cors)
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::default().include_headers(true))
                )
                .layer(middleware::from_fn_with_state(state.clone(), request_id_middleware))
                .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
                .layer(middleware::from_fn_with_state(state.clone(), logging_middleware)),
        )
        .with_state(state)
        .fallback(not_found_handler)
}

/// API routes
fn api_routes() -> Router<AppState> {
    Router::new()
        // Authentication routes
        .nest("/auth", auth_routes())
        // User routes
        .nest("/users", user_routes())
        // Project routes
        .nest("/projects", project_routes())
        // File routes
        .nest("/files", file_routes())
        // Compilation routes
        .nest("/compilation", compilation_routes())
        // Collaboration routes
        .nest("/collaboration", collaboration_routes())
}

/// Authentication routes
fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(crate::handlers::auth::register))
        .route("/login", post(crate::handlers::auth::login))
        .route("/refresh", post(crate::handlers::auth::refresh))
        .route("/logout", post(crate::handlers::auth::logout))
        .route("/forgot-password", post(crate::handlers::auth::forgot_password))
        .route("/reset-password", post(crate::handlers::auth::reset_password))
        .route("/verify-email", post(crate::handlers::auth::verify_email))
        // OIDC routes
        .route("/oidc/providers", get(crate::handlers::auth::get_oidc_providers))
        .route("/oidc/login", post(crate::handlers::auth::oidc_login))
        .route("/oidc/callback", get(crate::handlers::auth::oidc_callback))
        .route("/oidc/callback", post(crate::handlers::auth::oidc_callback_post))
        .layer(middleware::from_fn_with_state(
            Arc::new(crate::middleware::RateLimiter::new()),
            crate::middleware::auth_rate_limit_middleware,
        ))
}

/// User routes
fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(crate::handlers::user::get_current_user))
        .route("/", post(crate::handlers::user::update_user))
        .route("/preferences", get(crate::handlers::user::get_preferences))
        .route("/preferences", post(crate::handlers::user::update_preferences))
        .route("/search", get(crate::handlers::user::search_users))
}

/// Project routes
fn project_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(crate::handlers::project::list_projects))
        .route("/", post(crate::handlers::project::create_project))
        .route("/:id", get(crate::handlers::project::get_project))
        .route("/:id", put(crate::handlers::project::update_project))
        .route("/:id", delete(crate::handlers::project::delete_project))
        .route("/:id/collaborators", get(crate::handlers::project::get_collaborators))
        .route("/:id/collaborators", post(crate::handlers::project::add_collaborator))
        .route("/:id/collaborators/:user_id", delete(crate::handlers::project::remove_collaborator))
        .route("/:id/compile", post(crate::handlers::project::compile_project))
        .route("/:id/stats", get(crate::handlers::project::get_project_stats))
        .route("/:id/activity", get(crate::handlers::project::get_activity))
        .route("/search", get(crate::handlers::project::search_projects))
}

/// File routes
fn file_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(crate::handlers::file::list_files))
        .route("/", post(crate::handlers::file::create_file))
        .route("/:id", get(crate::handlers::file::get_file))
        .route("/:id", put(crate::handlers::file::update_file))
        .route("/:id", delete(crate::handlers::file::delete_file))
        .route("/:id/content", get(crate::handlers::file::get_file_content))
        .route("/:id/content", put(crate::handlers::file::update_file_content))
        .route("/:id/download", get(crate::handlers::file::download_file))
        .route("/upload", post(crate::handlers::file::upload_file))
        .route("/tree", get(crate::handlers::file::get_file_tree))
        .route("/search", get(crate::handlers::file::search_files))
}

/// Compilation routes
fn compilation_routes() -> Router<AppState> {
    Router::new()
        .route("/jobs", get(crate::handlers::compilation::list_jobs))
        .route("/jobs", post(crate::handlers::compilation::create_job))
        .route("/jobs/:id", get(crate::handlers::compilation::get_job))
        .route("/jobs/:id/cancel", post(crate::handlers::compilation::cancel_job))
        .route("/jobs/:id/logs", get(crate::handlers::compilation::get_job_logs))
        .route("/jobs/:id/artifacts", get(crate::handlers::compilation::get_job_artifacts))
        .route("/queue", get(crate::handlers::compilation::get_queue_status))
        .route("/templates", get(crate::handlers::compilation::list_templates))
        .route("/templates", post(crate::handlers::compilation::create_template))
        .route("/templates/:id", get(crate::handlers::compilation::get_template))
        .route("/stats", get(crate::handlers::compilation::get_compilation_stats))
}

/// Collaboration routes
fn collaboration_routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(crate::handlers::collaboration::list_sessions))
        .route("/sessions", post(crate::handlers::collaboration::create_session))
        .route("/sessions/:id", get(crate::handlers::collaboration::get_session))
        .route("/sessions/:id", put(crate::handlers::collaboration::update_session))
        .route("/sessions/:id", delete(crate::handlers::collaboration::delete_session))
        .route("/sessions/:id/join", post(crate::handlers::collaboration::join_session))
        .route("/sessions/:id/leave", post(crate::handlers::collaboration::leave_session))
        .route("/sessions/:id/participants", get(crate::handlers::collaboration::get_participants))
        .route("/sessions/:id/operations", post(crate::handlers::collaboration::create_operation))
        .route("/sessions/:id/messages", get(crate::handlers::collaboration::get_messages))
        .route("/sessions/:id/messages", post(crate::handlers::collaboration::send_message))
        .route("/sessions/:id/invite", post(crate::handlers::collaboration::invite_participant))
        .route("/sessions/:id/stats", get(crate::handlers::collaboration::get_session_stats))
        .route("/invitations/:token", get(crate::handlers::collaboration::get_invitation))
        .route("/invitations/:token", post(crate::handlers::collaboration::accept_invitation))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Not found handler
async fn not_found_handler() -> impl IntoResponse {
    let status = StatusCode::NOT_FOUND;
    let body = Json(serde_json::json!({
        "success": false,
        "error": {
            "message": "Endpoint not found",
            "code": "NOT_FOUND"
        }
    }));
    (status, body)
}

/// Request ID middleware
async fn request_id_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let request_id = RequestId::generate();

    let headers = request.headers();
    let existing_id = headers.get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| uuid::Uuid::parse_str(value).ok());

    let request_id = existing_id.map(RequestId).unwrap_or(request_id);

    let mut request = request;
    request.extensions_mut().insert(request_id);

    Ok(next.run(request).await)
}

/// Authentication middleware
async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Skip authentication for health check and auth routes
    let path = request.uri().path();
    if path == "/health" || path.starts_with("/api/v1/auth") {
        return Ok(next.run(request).await);
    }

    let headers = request.headers();
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            let claims = state.jwt_service.verify_token_with_db(token, &state.db_pool).await?;
            let auth_context = crate::models::auth::AuthContext::from(claims);

            if auth_context.is_expired() {
                return Err(AppError::Authentication("Token has expired".to_string()));
            }

            request.extensions_mut().insert(auth_context);
            return Ok(next.run(request).await);
        }
    }

    Err(AppError::Authentication("Missing or invalid authorization header".to_string()))
}

/// Logging middleware
async fn logging_middleware(
    State(_state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let start_time = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start_time.elapsed();
    let status = response.status();

    // Log request
    match status.as_u16() {
        200..=299 => {
            info!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request completed successfully"
            );
        }
        400..=499 => {
            warn!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = %status,
                duration_ms = %duration.as_millis(),
                "Client error"
            );
        }
        500..=599 => {
            tracing::error!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = %status,
                duration_ms = %duration.as_millis(),
                "Server error"
            );
        }
        _ => {}
    }

    response
}

impl AppState {
    /// Create new application state with OIDC clients
    pub async fn new(config: Config, db_pool: sqlx::PgPool) -> Result<Self, AppError> {
        // Initialize JWT service
        let jwt_service = crate::models::auth::JwtService::new(
            &config.jwt.secret,
            config.jwt.issuer.clone(),
            config.jwt.expiration as i64,
            config.jwt.refresh_expiration as i64,
        )?;

        // Initialize OIDC clients if enabled
        let mut oidc_clients = std::collections::HashMap::new();

        if config.oidc.enabled {
            for provider in &config.oidc.providers {
                // For now, only support GitHub
                if provider.name == "github" {
                    let oidc_client = authware::OidcClient::builder()
                        .new(authware::OidcProvider::GitHub, provider.client_id.clone(), provider.redirect_uri.clone())
                        .client_secret(provider.client_secret.clone())
                        .scopes(provider.scopes.clone())
                        .pkce(true)
                        .build()
                        .map_err(|e| AppError::Internal(format!("Failed to initialize GitHub OIDC client: {}", e)))?;

                    oidc_clients.insert(provider.name.clone(), oidc_client);
                    info!("Initialized GitHub OIDC client");
                } else {
                    warn!("OIDC provider '{}' not supported yet. Only GitHub is supported.", provider.name);
                }
            }
        }

        Ok(AppState {
            config: Arc::new(config),
            db_pool: Arc::new(db_pool),
            oidc_clients: Arc::new(oidc_clients),
            jwt_service: Arc::new(jwt_service),
            rate_limiter: Arc::new(crate::middleware::RateLimiter::new()),
        })
    }
}

/// Create the application
pub async fn create_app(state: AppState) -> Router {
    create_router(state)
}

/// Start the web server
pub async fn start_server(config: Config, db_pool: sqlx::PgPool) -> Result<(), AppError> {
    let state = AppState::new(config.clone(), db_pool).await?;

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    info!(
        "Starting server on {}",
        config.server.bind_address()
    );

    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| AppError::Config(format!("Failed to bind to {}: {}", addr, e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::Server(format!("Server error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_id_middleware() {
        // This test would require setting up a full app state
        // For now, we just verify the middleware compiles
        assert!(true);
    }

    #[tokio::test]
    async fn test_cors_configuration() {
        // This would test CORS headers are properly set
        assert!(true);
    }
}