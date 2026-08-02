use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::{CoreError, Result};
use crate::event_exceptions;
use crate::occurrences::{self, EventOccurrence, OccurrenceScope};
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

pub async fn list_occurrences(
    state: &AppState,
    calendar_id: &str,
    range_start_ms: i64,
    range_end_ms: i64,
) -> Result<Vec<EventOccurrence>> {
    let mut conn = state.pool.acquire().await?;

    let candidates = sqlx::query_as!(
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
          AND (
            (rrule IS NULL AND start_ms < ?3 AND end_ms > ?2)
            OR rrule IS NOT NULL
          )
        "#,
        calendar_id,
        range_start_ms,
        range_end_ms
    )
    .fetch_all(&mut *conn)
    .await?;

    let event_ids: Vec<String> = candidates.iter().map(|e| e.id.clone()).collect();
    let exceptions = event_exceptions::list_for_events_conn(&mut conn, &event_ids).await?;

    let mut by_event: std::collections::HashMap<String, Vec<event_exceptions::EventException>> =
        std::collections::HashMap::new();
    for exc in exceptions {
        by_event.entry(exc.event_id.clone()).or_default().push(exc);
    }

    let mut out = Vec::new();
    for event in candidates {
        if event.rrule.is_some() {
            let excs = by_event.remove(&event.id).unwrap_or_default();
            out.extend(occurrences::expand_recurring(
                &event,
                range_start_ms,
                range_end_ms,
                &excs,
            )?);
        } else if event.start_ms < range_end_ms && event.end_ms > range_start_ms {
            out.push(occurrences::single_occurrence(&event));
        }
    }

    out.sort_by_key(|o| (o.start_ms, o.title.clone()));
    Ok(out)
}

pub async fn create_event(
    state: &AppState,
    calendar_id: &str,
    title: &str,
    description: Option<&str>,
    start_ms: i64,
    end_ms: i64,
    all_day: bool,
    rrule: Option<&str>,
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
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL, NULL, ?10, ?11, NULL)
        "#,
        id,
        calendar_id,
        title,
        description,
        start_ms,
        end_ms,
        all_day_i,
        rrule,
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
        rrule: rrule.map(str::to_string),
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
    rrule: Option<&str>,
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
            rrule = ?6,
            updated_at = ?7
        WHERE id = ?8 AND deleted_at IS NULL
        "#,
        title,
        description,
        start_ms,
        end_ms,
        all_day_i,
        rrule,
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

pub async fn move_occurrence(
    state: &AppState,
    event_id: &str,
    original_start_ms: i64,
    new_start_ms: i64,
    new_end_ms: i64,
    scope: OccurrenceScope,
) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let event = fetch_by_id_conn(&mut tx, event_id)
        .await?
        .ok_or_else(|| CoreError::Message("event not found".into()))?;
    let all_day = event.all_day != 0;
    event_exceptions::validate_override_range(new_start_ms, new_end_ms, all_day)?;

    match scope {
        OccurrenceScope::This => {
            if event.rrule.is_none() {
                update_event_in_tx(
                    &mut tx,
                    &event,
                    &event.title,
                    event.description.as_deref(),
                    new_start_ms,
                    new_end_ms,
                    all_day,
                    None,
                )
                .await?;
            } else {
                event_exceptions::upsert_exception_conn(
                    &mut tx,
                    event_id,
                    original_start_ms,
                    Some(new_start_ms),
                    Some(new_end_ms),
                    false,
                )
                .await?;
            }
        }
        OccurrenceScope::All => {
            update_event_in_tx(
                &mut tx,
                &event,
                &event.title,
                event.description.as_deref(),
                new_start_ms,
                new_end_ms,
                all_day,
                event.rrule.as_deref(),
            )
            .await?;
            event_exceptions::delete_exceptions_after_conn(&mut tx, event_id, 0).await?;
        }
        OccurrenceScope::ThisAndFollowing => {
            let rrule = event
                .rrule
                .as_deref()
                .ok_or_else(|| CoreError::Message("not a recurring event".into()))?;
            let until_ms = original_start_ms.saturating_sub(1);
            let truncated = occurrences::truncate_rrule_until(rrule, until_ms)?;
            update_event_in_tx(
                &mut tx,
                &event,
                &event.title,
                event.description.as_deref(),
                event.start_ms,
                event.end_ms,
                all_day,
                Some(&truncated),
            )
            .await?;
            event_exceptions::delete_exceptions_after_conn(
                &mut tx,
                event_id,
                original_start_ms,
            )
            .await?;

            let duration = new_end_ms.saturating_sub(new_start_ms);
            create_event_in_tx(
                &mut tx,
                &event.calendar_id,
                &event.title,
                event.description.as_deref(),
                new_start_ms,
                new_start_ms.saturating_add(duration),
                all_day,
                Some(rrule),
            )
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_occurrence(
    state: &AppState,
    event_id: &str,
    original_start_ms: i64,
    scope: OccurrenceScope,
) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let event = fetch_by_id_conn(&mut tx, event_id)
        .await?
        .ok_or_else(|| CoreError::Message("event not found".into()))?;

    match scope {
        OccurrenceScope::This => {
            if event.rrule.is_none() {
                soft_delete_event_in_tx(&mut tx, &event).await?;
            } else {
                event_exceptions::upsert_exception_conn(
                    &mut tx,
                    event_id,
                    original_start_ms,
                    None,
                    None,
                    true,
                )
                .await?;
            }
        }
        OccurrenceScope::All => {
            soft_delete_event_in_tx(&mut tx, &event).await?;
        }
        OccurrenceScope::ThisAndFollowing => {
            let rrule = event
                .rrule
                .as_deref()
                .ok_or_else(|| CoreError::Message("not a recurring event".into()))?;
            let until_ms = original_start_ms.saturating_sub(1);
            let truncated = occurrences::truncate_rrule_until(rrule, until_ms)?;
            update_event_in_tx(
                &mut tx,
                &event,
                &event.title,
                event.description.as_deref(),
                event.start_ms,
                event.end_ms,
                event.all_day != 0,
                Some(&truncated),
            )
            .await?;
            event_exceptions::delete_exceptions_after_conn(
                &mut tx,
                event_id,
                original_start_ms,
            )
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn update_event_in_tx(
    conn: &mut SqliteConnection,
    event: &Event,
    title: &str,
    description: Option<&str>,
    start_ms: i64,
    end_ms: i64,
    all_day: bool,
    rrule: Option<&str>,
) -> Result<Event> {
    validate_range(start_ms, end_ms, all_day)?;
    let now = envelope::now_ms();
    let all_day_i: i64 = if all_day { 1 } else { 0 };

    sqlx::query!(
        r#"
        UPDATE events SET
            title = ?1,
            description = ?2,
            start_ms = ?3,
            end_ms = ?4,
            all_day = ?5,
            rrule = ?6,
            updated_at = ?7
        WHERE id = ?8 AND deleted_at IS NULL
        "#,
        title,
        description,
        start_ms,
        end_ms,
        all_day_i,
        rrule,
        now,
        event.id
    )
    .execute(&mut *conn)
    .await?;

    let updated = fetch_by_id_conn(conn, &event.id)
        .await?
        .ok_or_else(|| CoreError::Message("event not found".into()))?;
    let payload = serde_json::to_string(&updated)?;
    record_change_conn(
        conn,
        ChangeOp::Update,
        "event",
        &event.id,
        Some(&payload),
    )
    .await?;
    Ok(updated)
}

async fn create_event_in_tx(
    conn: &mut SqliteConnection,
    calendar_id: &str,
    title: &str,
    description: Option<&str>,
    start_ms: i64,
    end_ms: i64,
    all_day: bool,
    rrule: Option<&str>,
) -> Result<Event> {
    validate_range(start_ms, end_ms, all_day)?;
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
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL, NULL, ?10, ?11, NULL)
        "#,
        id,
        calendar_id,
        title,
        description,
        start_ms,
        end_ms,
        all_day_i,
        rrule,
        source,
        now,
        now
    )
    .execute(&mut *conn)
    .await?;

    let event = Event {
        id,
        calendar_id: calendar_id.to_string(),
        title: title.to_string(),
        description: description.map(str::to_string),
        start_ms,
        end_ms,
        all_day: all_day_i,
        rrule: rrule.map(str::to_string),
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
        conn,
        ChangeOp::Insert,
        "event",
        &event.id,
        Some(&payload),
    )
    .await?;
    Ok(event)
}

async fn soft_delete_event_in_tx(conn: &mut SqliteConnection, event: &Event) -> Result<()> {
    let now = envelope::now_ms();
    sqlx::query!(
        r#"
        UPDATE events SET deleted_at = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        "#,
        now,
        now,
        event.id
    )
    .execute(&mut *conn)
    .await?;

    let deleted = fetch_by_id_conn(conn, &event.id)
        .await?
        .ok_or_else(|| CoreError::Message("event not found".into()))?;
    let payload = serde_json::to_string(&deleted)?;
    record_change_conn(
        conn,
        ChangeOp::Delete,
        "event",
        &event.id,
        Some(&payload),
    )
    .await?;
    Ok(())
}
