//! Admin user initialization functionality

use tracing::{info, warn};
use crate::models::user::{User, CreateUser};

/// Ensure that an admin user exists on startup
/// Creates an admin user with username "admin" and password "password" if it doesn't exist
pub async fn ensure_admin_user(db_pool: &sqlx::PgPool) -> Result<(), crate::error::AppError> {
    const ADMIN_USERNAME: &str = "admin";
    const ADMIN_PASSWORD: &str = "password";
    const ADMIN_EMAIL: &str = "admin@texler.local";
    const ADMIN_DISPLAY_NAME: &str = "Administrator";

    // Check if admin user already exists
    match User::find_by_username(db_pool, ADMIN_USERNAME).await? {
        Some(_) => {
            info!("Admin user '{}' already exists", ADMIN_USERNAME);
            Ok(())
        }
        None => {
            warn!("Admin user '{}' not found, creating with default credentials", ADMIN_USERNAME);

            // Create admin user
            let admin_user = CreateUser {
                username: ADMIN_USERNAME.to_string(),
                email: ADMIN_EMAIL.to_string(),
                password: ADMIN_PASSWORD.to_string(),
                display_name: ADMIN_DISPLAY_NAME.to_string(),
                avatar_url: None,
            };

            match User::create(db_pool, admin_user).await {
                Ok(user) => {
                    info!("Successfully created admin user '{}' with ID: {}", ADMIN_USERNAME, user.id);
                    warn!("SECURITY WARNING: Admin user created with default password '{}'. Please change this password immediately.", ADMIN_PASSWORD);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to create admin user: {}", e);
                    Err(e)
                }
            }
        }
    }
}