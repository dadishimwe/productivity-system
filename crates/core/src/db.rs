use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

use crate::error::Result;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

pub async fn init_pool(db_path: &Path) -> Result<AppState> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let options = SqliteConnectOptions::from_str(&format!(
        "sqlite:{}?mode=rwc",
        db_path.display()
    ))?
    .foreign_keys(true)
    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
    .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    Ok(AppState { pool })
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./src/migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
pub async fn test_pool() -> Result<AppState> {
    let dir = std::env::temp_dir().join(format!(
        "productivity_core_test_{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("test.db");
    init_pool(&path).await
}
