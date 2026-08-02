use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::{CoreError, Result};
use crate::outbox::{record_change_conn, ChangeOp};
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub all_day: i64,
    pub rrule: Option<String>,
    pub source: String,
    pub external_event_id: Option<String>,
    pub external_calendar_id: Option<String>,
    pub last_synced_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

pub(crate) async fn fetch_by_id_conn(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Event>> {
    sqlx::query_as!(
        Event,
        r#"
        SELECT
            id as "id!",
            calendar_id as "calendar_id!",
            title as "title!",
            description,
            start_ms as "start_ms!",
            end_ms as "end_ms!",
            all_day as "all_day!",
            rrule,
            source as "source!",
            external_event_id,
            external_calendar_id,
            last_synced_at,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM events
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

fn validate_range(start_ms: i64, end_ms: i64, all_day: bool) -> Result<()> {
    if all_day {
        if end_ms < start_ms {
            return Err(CoreError::Message("all-day event end before start".into()));
        }
    } else if end_ms <= start_ms {
        return Err(CoreError::Message("event end must be after start".into()));
    }
    Ok(())
}

pub async fn list_events_in_range(
    state: &AppState,
    calendar_id: &str,
    range_start_ms: i64,
    range_end_ms: i64,
) -> Result<Vec<Event>> {
    let events = sqlx::query_as!(
        Event,
        r#"
        SELECT
            id as "id!",
            calendar_id as "calendar_id!",
            title as "title!",
            description,
            start_ms as "start_ms!",
            end_ms as "end_ms!",
            all_day as "all_day!",
            rrule,
            source as "source!",
            external_event_id,
            external_calendar_id,
            last_synced_at,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM events
        WHERE calendar_id = ?1
          AND deleted_at IS NULL
          AND start_ms < ?3
          AND end_ms > ?2
        ORDER BY start_ms ASC, end_ms ASC
        "#,
        calendar_id,
        range_start_ms,
        range_end_ms
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(events)
}

pub async fn create_event(
    state: &AppState,
    calendar_id: &str,
    title: &str,
    description: Option<&str>,
    start_ms: i64,
    end_ms: i64,
    all_day: bool,
) -> Result<Event> {
    validate_range(start_ms, end_ms, all_day)?;

    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let id = envelope::new_id();
    let now = envelope::now_ms();
    let source = "local";
    let all_day_i: i64 = if all_day { 1 } else { 0 };

    sqlx::query!(
        r#"
        INSERT INTO events (
            id, calendar_id, title, description, start_ms, end_ms, all_day,
            rrule, source, external_event_id, external_calendar_id, last_synced_at,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, NULL, NULL, NULL, ?9, ?10, NULL)
        "#,
        id,
        calendar_id,
        title,
        description,
        start_ms,
        end_ms,
        all_day_i,
        source,
        now,
        now
    )
    .execute(&mut *tx)
    .await?;

    let event = Event {
        id,
        calendar_id: calendar_id.to_string(),
        title: title.to_string(),
        description: description.map(str::to_string),
        start_ms,
        end_ms,
        all_day: all_day_i,
        rrule: None,
        source: source.to_string(),
        external_event_id: None,
        external_calendar_id: None,
        last_synced_at: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    let payload = serde_json::to_string(&event)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "event",
        &event.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(event)
}

pub async fn update_event(
    state: &AppState,
    id: &str,
    title: &str,
    description: Option<&str>,
    start_ms: i64,
    end_ms: i64,
    all_day: bool,
) -> Result<Event> {
    validate_range(start_ms, end_ms, all_day)?;

    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();
    let all_day_i: i64 = if all_day { 1 } else { 0 };

    let updated = sqlx::query!(
        r#"
        UPDATE events SET
            title = ?1,
            description = ?2,
            start_ms = ?3,
            end_ms = ?4,
            all_day = ?5,
            updated_at = ?6
        WHERE id = ?7 AND deleted_at IS NULL
        "#,
        title,
        description,
        start_ms,
        end_ms,
        all_day_i,
        now,
        id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if updated == 0 {
        return Err(CoreError::Message("event not found".into()));
    }

    let event = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::Message("event not found".into()))?;

    let payload = serde_json::to_string(&event)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "event",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(event)
}

pub async fn delete_event(state: &AppState, id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let updated = sqlx::query!(
        r#"
        UPDATE events SET deleted_at = ?1, updated_at = ?2
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
        return Err(CoreError::Message("event not found".into()));
    }

    let event = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::Message("event not found".into()))?;
    let payload = serde_json::to_string(&event)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "event",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}
