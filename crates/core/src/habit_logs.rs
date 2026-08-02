use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::Result;
use crate::outbox::{record_change_conn, ChangeOp};
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HabitLog {
    pub id: String,
    pub habit_id: String,
    pub date: String,
    pub value: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn fetch_active(
    conn: &mut SqliteConnection,
    habit_id: &str,
    date: &str,
) -> Result<Option<HabitLog>> {
    sqlx::query_as!(
        HabitLog,
        r#"
        SELECT
            id as "id!",
            habit_id as "habit_id!",
            date as "date!",
            value as "value!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM habit_logs
        WHERE habit_id = ?1 AND date = ?2 AND deleted_at IS NULL
        "#,
        habit_id,
        date
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

async fn fetch_any(
    conn: &mut SqliteConnection,
    habit_id: &str,
    date: &str,
) -> Result<Option<HabitLog>> {
    sqlx::query_as!(
        HabitLog,
        r#"
        SELECT
            id as "id!",
            habit_id as "habit_id!",
            date as "date!",
            value as "value!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM habit_logs
        WHERE habit_id = ?1 AND date = ?2
        "#,
        habit_id,
        date
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

pub async fn log_habit(
    state: &AppState,
    habit_id: &str,
    date: &str,
    value: i64,
) -> Result<HabitLog> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let before = fetch_any(&mut tx, habit_id, date).await?;
    let op = if before.is_some() {
        ChangeOp::Update
    } else {
        ChangeOp::Insert
    };

    let id = before
        .as_ref()
        .map(|r| r.id.clone())
        .unwrap_or_else(envelope::new_id);
    let created_at = before.as_ref().map(|r| r.created_at).unwrap_or_else(envelope::now_ms);
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        INSERT INTO habit_logs (id, habit_id, date, value, created_at, updated_at, deleted_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)
        ON CONFLICT(habit_id, date) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at,
            deleted_at = NULL
        "#,
        id,
        habit_id,
        date,
        value,
        created_at,
        now
    )
    .execute(&mut *tx)
    .await?;

    let log = fetch_active(&mut tx, habit_id, date)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("habit log missing after upsert".into()))?;

    let payload = serde_json::to_string(&log)?;
    record_change_conn(
        &mut tx,
        op,
        "habit_log",
        &log.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(log)
}

pub async fn unlog_habit(state: &AppState, habit_id: &str, date: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let log = fetch_active(&mut tx, habit_id, date)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("habit log not found".into()))?;

    sqlx::query!(
        r#"
        UPDATE habit_logs SET deleted_at = ?1, updated_at = ?1
        WHERE habit_id = ?2 AND date = ?3 AND deleted_at IS NULL
        "#,
        now,
        habit_id,
        date
    )
    .execute(&mut *tx)
    .await?;

    let mut deleted = log.clone();
    deleted.deleted_at = Some(now);
    deleted.updated_at = now;
    let payload = serde_json::to_string(&deleted)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "habit_log",
        &log.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn list_habit_logs(
    state: &AppState,
    habit_id: &str,
    from_date: &str,
    to_date: &str,
) -> Result<Vec<HabitLog>> {
    let logs = sqlx::query_as!(
        HabitLog,
        r#"
        SELECT
            id as "id!",
            habit_id as "habit_id!",
            date as "date!",
            value as "value!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM habit_logs
        WHERE habit_id = ?1
          AND date >= ?2
          AND date <= ?3
          AND deleted_at IS NULL
        ORDER BY date ASC
        "#,
        habit_id,
        from_date,
        to_date
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(logs)
}
