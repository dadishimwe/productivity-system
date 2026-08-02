use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::{CoreError, Result};
use crate::outbox::{record_change_conn, ChangeOp};
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub source: String,
    pub external_account_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn fetch_by_id_conn(conn: &mut SqliteConnection, id: &str) -> Result<Option<Calendar>> {
    sqlx::query_as!(
        Calendar,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            color,
            source as "source!",
            external_account_id,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM calendars
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

pub async fn ensure_default_calendar(state: &AppState) -> Result<Calendar> {
    if let Some(existing) = list_calendars(state).await?.into_iter().next() {
        return Ok(existing);
    }
    create_calendar(state, "Personal", Some("#3b82f6")).await
}

pub async fn create_calendar(
    state: &AppState,
    name: &str,
    color: Option<&str>,
) -> Result<Calendar> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let id = envelope::new_id();
    let now = envelope::now_ms();
    let source = "local";

    sqlx::query!(
        r#"
        INSERT INTO calendars (
            id, name, color, source, external_account_id,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, NULL)
        "#,
        id,
        name,
        color,
        source,
        now,
        now
    )
    .execute(&mut *tx)
    .await?;

    let calendar = Calendar {
        id,
        name: name.to_string(),
        color: color.map(str::to_string),
        source: source.to_string(),
        external_account_id: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    let payload = serde_json::to_string(&calendar)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "calendar",
        &calendar.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(calendar)
}

pub async fn list_calendars(state: &AppState) -> Result<Vec<Calendar>> {
    let rows = sqlx::query_as!(
        Calendar,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            color,
            source as "source!",
            external_account_id,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM calendars
        WHERE deleted_at IS NULL
        ORDER BY name ASC
        "#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(rows)
}

pub async fn rename_calendar(
    state: &AppState,
    id: &str,
    name: &str,
) -> Result<Calendar> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let updated = sqlx::query!(
        r#"
        UPDATE calendars SET name = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        "#,
        name,
        now,
        id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if updated == 0 {
        return Err(CoreError::Message("calendar not found".into()));
    }

    let calendar = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::Message("calendar not found".into()))?;

    let payload = serde_json::to_string(&calendar)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "calendar",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(calendar)
}

pub async fn delete_calendar(state: &AppState, id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let updated = sqlx::query!(
        r#"
        UPDATE calendars SET deleted_at = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        "#,
        now,
        now,
        id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if updated == 0 {
        return Err(CoreError::Message("calendar not found".into()));
    }

    let calendar = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::Message("calendar not found".into()))?;
    let payload = serde_json::to_string(&calendar)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "calendar",
        id,
        Some(&payload),
    )
    .await?;

    let event_ids = sqlx::query_scalar!(
        r#"SELECT id as "id!" FROM events WHERE calendar_id = ?1 AND deleted_at IS NULL"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;

    for event_id in event_ids {
        sqlx::query!(
            r#"UPDATE events SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3"#,
            now,
            now,
            event_id
        )
        .execute(&mut *tx)
        .await?;

        let event = crate::events::fetch_by_id_conn(&mut tx, &event_id)
            .await?
            .ok_or_else(|| CoreError::Message("event missing after delete".into()))?;
        let event_payload = serde_json::to_string(&event)?;
        record_change_conn(
            &mut tx,
            ChangeOp::Delete,
            "event",
            &event_id,
            Some(&event_payload),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
