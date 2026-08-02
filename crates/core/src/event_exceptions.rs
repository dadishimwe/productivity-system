use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::envelope;
use crate::error::{CoreError, Result};
use crate::outbox::{record_change_conn, ChangeOp};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventException {
    pub id: String,
    pub event_id: String,
    pub original_start_ms: i64,
    pub override_start_ms: Option<i64>,
    pub override_end_ms: Option<i64>,
    pub cancelled: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn list_for_events_conn(
    conn: &mut SqliteConnection,
    event_ids: &[String],
) -> Result<Vec<EventException>> {
    let mut all = Vec::new();
    for event_id in event_ids {
        let rows = sqlx::query_as!(
            EventException,
            r#"
            SELECT
                id as "id!",
                event_id as "event_id!",
                original_start_ms as "original_start_ms!",
                override_start_ms,
                override_end_ms,
                cancelled as "cancelled!",
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM event_exceptions
            WHERE event_id = ?1
            "#,
            event_id
        )
        .fetch_all(&mut *conn)
        .await?;
        all.extend(rows);
    }
    Ok(all)
}

pub async fn upsert_exception_conn(
    conn: &mut SqliteConnection,
    event_id: &str,
    original_start_ms: i64,
    override_start_ms: Option<i64>,
    override_end_ms: Option<i64>,
    cancelled: bool,
) -> Result<EventException> {
    let now = envelope::now_ms();
    let cancelled_i: i64 = if cancelled { 1 } else { 0 };

    let existing = sqlx::query_as!(
        EventException,
        r#"
        SELECT
            id as "id!",
            event_id as "event_id!",
            original_start_ms as "original_start_ms!",
            override_start_ms,
            override_end_ms,
            cancelled as "cancelled!",
            created_at as "created_at!",
            updated_at as "updated_at!"
        FROM event_exceptions
        WHERE event_id = ?1 AND original_start_ms = ?2
        "#,
        event_id,
        original_start_ms
    )
    .fetch_optional(&mut *conn)
    .await?;

    let is_update = existing.is_some();

    let row = if let Some(prev) = existing {
        sqlx::query!(
            r#"
            UPDATE event_exceptions SET
                override_start_ms = ?1,
                override_end_ms = ?2,
                cancelled = ?3,
                updated_at = ?4
            WHERE id = ?5
            "#,
            override_start_ms,
            override_end_ms,
            cancelled_i,
            now,
            prev.id
        )
        .execute(&mut *conn)
        .await?;

        EventException {
            id: prev.id,
            event_id: event_id.to_string(),
            original_start_ms,
            override_start_ms,
            override_end_ms,
            cancelled: cancelled_i,
            created_at: prev.created_at,
            updated_at: now,
        }
    } else {
        let id = envelope::new_id();
        sqlx::query!(
            r#"
            INSERT INTO event_exceptions (
                id, event_id, original_start_ms, override_start_ms, override_end_ms,
                cancelled, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            id,
            event_id,
            original_start_ms,
            override_start_ms,
            override_end_ms,
            cancelled_i,
            now,
            now
        )
        .execute(&mut *conn)
        .await?;

        EventException {
            id,
            event_id: event_id.to_string(),
            original_start_ms,
            override_start_ms,
            override_end_ms,
            cancelled: cancelled_i,
            created_at: now,
            updated_at: now,
        }
    };

    let payload = serde_json::to_string(&row)?;
    let op = if is_update {
        ChangeOp::Update
    } else {
        ChangeOp::Insert
    };
    record_change_conn(
        conn,
        op,
        "event_exception",
        &row.id,
        Some(&payload),
    )
    .await?;

    Ok(row)
}

pub async fn delete_exceptions_after_conn(
    conn: &mut SqliteConnection,
    event_id: &str,
    from_original_start_ms: i64,
) -> Result<()> {
    let rows = sqlx::query_as!(
        EventException,
        r#"
        SELECT
            id as "id!",
            event_id as "event_id!",
            original_start_ms as "original_start_ms!",
            override_start_ms,
            override_end_ms,
            cancelled as "cancelled!",
            created_at as "created_at!",
            updated_at as "updated_at!"
        FROM event_exceptions
        WHERE event_id = ?1 AND original_start_ms >= ?2
        "#,
        event_id,
        from_original_start_ms
    )
    .fetch_all(&mut *conn)
    .await?;

    for row in rows {
        sqlx::query!(r#"DELETE FROM event_exceptions WHERE id = ?1"#, row.id)
            .execute(&mut *conn)
            .await?;
        let payload = serde_json::to_string(&row)?;
        record_change_conn(
            conn,
            ChangeOp::Delete,
            "event_exception",
            &row.id,
            Some(&payload),
        )
        .await?;
    }
    Ok(())
}

pub fn exception_map(exceptions: &[EventException]) -> std::collections::HashMap<(String, i64), &EventException> {
    exceptions
        .iter()
        .map(|e| ((e.event_id.clone(), e.original_start_ms), e))
        .collect()
}

pub fn apply_exception(
    exc: &EventException,
    duration_ms: i64,
    default_start_ms: i64,
) -> Result<Option<(i64, i64)>> {
    if exc.cancelled != 0 {
        return Ok(None);
    }
    let start = exc.override_start_ms.unwrap_or(default_start_ms);
    let end = exc
        .override_end_ms
        .unwrap_or(start.saturating_add(duration_ms));
    Ok(Some((start, end)))
}

pub fn validate_override_range(start_ms: i64, end_ms: i64, all_day: bool) -> Result<()> {
    if all_day {
        if end_ms < start_ms {
            return Err(CoreError::Message("all-day override end before start".into()));
        }
    } else if end_ms <= start_ms {
        return Err(CoreError::Message("override end must be after start".into()));
    }
    Ok(())
}
