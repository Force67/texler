use texler_backend::{config, error, server};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "texler_backend=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Texler backend server...");

    // Load configuration
    let config = config::Config::load()?;
    info!("Configuration loaded successfully");

    // Initialize database connection pool
    let db_pool = sqlx::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string())
        .await
        .map_err(|e| {
            error!("Failed to connect to database: {}", e);
            e
        })?;

    info!("Database connection established");

    // Run database migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .map_err(|e| {
            error!("Failed to run database migrations: {}", e);
            e
        })?;

    info!("Database migrations completed");

    // Create application state
    let app_state = server::AppState::new(config.clone(), db_pool).await?;

    // Build the application
    let app = server::create_app(app_state).await?;

    // Start the server
    let listener = tokio::net::TcpListener::bind(&config.server.bind_address())
        .await
        .map_err(|e| {
            error!("Failed to bind to {}: {}", config.server.bind_address(), e);
            e
        })?;

    info!("Server listening on {}", listener.local_addr()?);

    // Start WebSocket server if enabled
    let ws_handle = if config.features.websocket {
        let ws_config = config.websocket.clone();
        Some(tokio::spawn(async move {
            if let Err(e) = server::websocket::start_websocket_server(ws_config).await {
                error!("WebSocket server error: {}", e);
            }
        }))
    } else {
        None
    };

    // Start background job processor if enabled
    let job_processor = if config.features.background_jobs {
        Some(tokio::spawn(async move {
            if let Err(e) = server::jobs::start_job_processor(db_pool).await {
                error!("Job processor error: {}", e);
            }
        }))
    } else {
        None
    };

    // Run the server
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
    }

    // Cleanup
    if let Some(handle) = ws_handle {
        handle.abort();
    }
    if let Some(handle) = job_processor {
        handle.abort();
    }

    info!("Server shutdown complete");
    Ok(())
}