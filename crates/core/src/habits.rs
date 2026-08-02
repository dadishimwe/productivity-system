use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::Result;
use crate::outbox::{record_change_conn, ChangeOp};
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Habit {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub target_frequency: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn fetch_by_id_conn(conn: &mut SqliteConnection, id: &str) -> Result<Option<Habit>> {
    sqlx::query_as!(
        Habit,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            color,
            target_frequency,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM habits
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

pub async fn create_habit(
    state: &AppState,
    name: &str,
    color: Option<&str>,
    target_frequency: Option<&str>,
) -> Result<Habit> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let id = envelope::new_id();
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        INSERT INTO habits (id, name, color, target_frequency, created_at, updated_at, deleted_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)
        "#,
        id,
        name,
        color,
        target_frequency,
        now,
        now
    )
    .execute(&mut *tx)
    .await?;

    let habit = Habit {
        id,
        name: name.to_string(),
        color: color.map(str::to_string),
        target_frequency: target_frequency.map(str::to_string),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    let payload = serde_json::to_string(&habit)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "habit",
        &habit.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(habit)
}

pub async fn list_habits(state: &AppState) -> Result<Vec<Habit>> {
    let habits = sqlx::query_as!(
        Habit,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            color,
            target_frequency,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM habits
        WHERE deleted_at IS NULL
        ORDER BY name ASC
        "#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(habits)
}

pub async fn update_habit(
    state: &AppState,
    id: &str,
    name: &str,
    color: Option<&str>,
    target_frequency: Option<&str>,
) -> Result<Habit> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        UPDATE habits
        SET name = ?1, color = ?2, target_frequency = ?3, updated_at = ?4
        WHERE id = ?5 AND deleted_at IS NULL
        "#,
        name,
        color,
        target_frequency,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let habit = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("habit not found".into()))?;

    let payload = serde_json::to_string(&habit)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "habit",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(habit)
}

pub async fn delete_habit(state: &AppState, id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let habit = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("habit not found".into()))?;

    sqlx::query!(
        r#"UPDATE habits SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2"#,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let mut deleted = habit.clone();
    deleted.deleted_at = Some(now);
    deleted.updated_at = now;
    let payload = serde_json::to_string(&deleted)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "habit",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}
