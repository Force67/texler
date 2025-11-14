//! Texler Backend - A high-performance collaborative LaTeX editor API
//!
//! This crate provides the main backend functionality for Texler, including:
//! - RESTful API with Axum
//! - PostgreSQL database with SQLx
//! - WebSocket real-time collaboration
//! - JWT authentication
//! - LaTeX compilation services
//!
//! # Example
//!
//! ```rust,no_run
//! use texler_backend::server;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = server::create_app(app_state).await?;
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
//!
//!     axum::serve(listener, app).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod admin_init;
pub mod config;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod migrate;
pub mod models;
pub mod server;
pub mod websocket;

// Re-export commonly used types
pub use error::{AppError, Result};
pub use models::*;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");