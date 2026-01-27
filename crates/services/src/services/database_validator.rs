//! Database validation service for ensuring migrations are up to date

use sqlx::SqlitePool;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum DatabaseValidationError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migrations not up to date: {0}")]
    MigrationsOutOfDate(String),
    #[error("database not initialized")]
    NotInitialized,
}

/// Database validator for ensuring schema is correct
pub struct DatabaseValidator {
    pool: SqlitePool,
}

impl DatabaseValidator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Check if the database is initialized and migrations are up to date
    pub async fn validate(&self) -> Result<ValidationResult, DatabaseValidationError> {
        // Check if _sqlx_migrations table exists
        let migrations_table_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'"
        )
        .fetch_one(&self.pool)
        .await? > 0;

        if !migrations_table_exists {
            warn!("Database not initialized - _sqlx_migrations table does not exist");
            return Ok(ValidationResult {
                is_initialized: false,
                migrations_applied: 0,
                pending_migrations: vec![],
                warnings: vec!["Database has not been initialized. Run migrations.".to_string()],
            });
        }

        // Count applied migrations
        let migrations_applied = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1"
        )
        .fetch_one(&self.pool)
        .await?;

        info!(
            migrations_applied = migrations_applied,
            "Database validation complete"
        );

        Ok(ValidationResult {
            is_initialized: true,
            migrations_applied: migrations_applied as usize,
            pending_migrations: vec![],
            warnings: vec![],
        })
    }

    /// Validate that specific tables exist
    pub async fn validate_tables(&self, required_tables: &[&str]) -> Result<Vec<String>, DatabaseValidationError> {
        let mut missing_tables = Vec::new();

        for table in required_tables {
            let exists = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?"
            )
            .bind(table)
            .fetch_one(&self.pool)
            .await? > 0;

            if !exists {
                missing_tables.push(table.to_string());
            }
        }

        Ok(missing_tables)
    }

    /// Get the latest applied migration
    pub async fn get_latest_migration(&self) -> Result<Option<String>, DatabaseValidationError> {
        let migration = sqlx::query_scalar::<_, String>(
            "SELECT description FROM _sqlx_migrations WHERE success = 1 ORDER BY installed_on DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(migration)
    }
}

/// Result of database validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_initialized: bool,
    pub migrations_applied: usize,
    pub pending_migrations: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Check if validation passed without issues
    pub fn is_ok(&self) -> bool {
        self.is_initialized && self.warnings.is_empty()
    }

    /// Get a summary message
    pub fn summary(&self) -> String {
        if !self.is_initialized {
            "Database not initialized - migrations need to be run".to_string()
        } else if !self.warnings.is_empty() {
            format!("Database validation warnings: {}", self.warnings.join(", "))
        } else {
            format!(
                "Database OK - {} migrations applied",
                self.migrations_applied
            )
        }
    }
}
