use sqlx::{Executor, SqliteConnection};

use crate::envelope;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeOp {
    Insert,
    Update,
    Delete,
}

impl ChangeOp {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeOp::Insert => "insert",
            ChangeOp::Update => "update",
            ChangeOp::Delete => "delete",
        }
    }
}

/// Records a change in the outbox within the current transaction.
pub async fn record_change<'e, E>(
    executor: E,
    op: ChangeOp,
    entity_type: &str,
    entity_id: &str,
    payload: Option<&str>,
) -> Result<()>
where
    E: Executor<'e, Database = sqlx::Sqlite>,
{
    let id = envelope::new_id();
    let created_at = envelope::now_ms();
    let op_str = op.as_str();

    sqlx::query!(
        r#"
        INSERT INTO changes (id, op, entity_type, entity_id, payload, created_at, synced_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)
        "#,
        id,
        op_str,
        entity_type,
        entity_id,
        payload,
        created_at,
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn record_change_conn(
    conn: &mut SqliteConnection,
    op: ChangeOp,
    entity_type: &str,
    entity_id: &str,
    payload: Option<&str>,
) -> Result<()> {
    record_change(&mut *conn, op, entity_type, entity_id, payload).await
}
