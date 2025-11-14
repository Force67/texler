//! Database migration management

use sqlx::PgPool;
use tracing::{error, info};

/// Run all database migrations
pub async fn run_migrations(db_pool: &PgPool) -> Result<(), crate::error::AppError> {
    info!("Running database migrations...");

    // Create migration tracking table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version VARCHAR(255) PRIMARY KEY,
            applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        );
        "#
    )
    .execute(db_pool)
    .await
    .map_err(crate::error::AppError::Database)?;

    let migrations = get_migrations();

    for migration in migrations {
        // Check if migration was already applied
        let applied = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = $1)"
        )
        .bind(migration.version)
        .fetch_one(db_pool)
        .await
        .map_err(crate::error::AppError::Database)?;

        if applied {
            info!("Migration {} already applied", migration.version);
            continue;
        }

        info!("Applying migration: {}", migration.version);

        // Execute migration using simple SQL execution (not prepared statements)
        // This allows for DO blocks and other complex SQL constructs
        sqlx::raw_sql(migration.sql)
            .execute(db_pool)
            .await
            .map_err(|e| {
                error!("Failed to apply migration {}: {}", migration.version, e);
                crate::error::AppError::Database(e)
            })?;

        // Mark migration as applied
        sqlx::query(
            "INSERT INTO schema_migrations (version) VALUES ($1)"
        )
        .bind(migration.version)
        .execute(db_pool)
        .await
        .map_err(crate::error::AppError::Database)?;

        info!("Migration {} applied successfully", migration.version);
    }

    info!("All migrations completed successfully");
    Ok(())
}

struct Migration {
    version: &'static str,
    sql: &'static str,
}

fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: "001_initial_schema",
            sql: include_str!("../migrations/001_initial_schema.sql"),
        },
        Migration {
            version: "002_add_workspaces",
            sql: include_str!("../migrations/002_add_workspaces.sql"),
        },
        Migration {
            version: "003_add_blacklisted_tokens",
            sql: include_str!("../migrations/003_add_blacklisted_tokens.sql"),
        },
        Migration {
            version: "004_fix_latex_engine_type",
            sql: include_str!("../migrations/004_fix_latex_engine_type.sql"),
        },
        Migration {
            version: "005_create_functions",
            sql: include_str!("../migrations/005_create_functions.sql"),
        },
    ]
}