use sqlx::postgres::PgPoolOptions;
use texler_backend::{config, server};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Starting Texler backend server...");

    let config = config::Config::load()?;

    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string())
        .await
        .map_err(|e| {
            error!("Failed to connect to database: {}", e);
            e
        })?;

    server::start_server(config, db_pool)
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            Box::<dyn std::error::Error>::from(e)
        })
}
