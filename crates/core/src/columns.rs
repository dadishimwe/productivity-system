use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::Result;
use crate::outbox::{record_change_conn, ChangeOp};
use crate::positioning;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Column {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub position: f64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn sibling_positions(
    conn: &mut SqliteConnection,
    board_id: &str,
) -> Result<Vec<f64>> {
    let rows = sqlx::query!(
        r#"
        SELECT position as "position!: f64"
        FROM columns
        WHERE board_id = ?1 AND deleted_at IS NULL
        ORDER BY position ASC
        "#,
        board_id
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(|r| r.position).collect())
}

async fn siblings(conn: &mut SqliteConnection, board_id: &str) -> Result<Vec<(String, f64)>> {
    let rows = sqlx::query!(
        r#"
        SELECT id as "id!", position as "position!: f64"
        FROM columns
        WHERE board_id = ?1 AND deleted_at IS NULL
        ORDER BY position ASC
        "#,
        board_id
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(|r| (r.id, r.position)).collect())
}

async fn rebalance(conn: &mut SqliteConnection, board_id: &str) -> Result<Vec<f64>> {
    let ordered = siblings(conn, board_id).await?;
    let positions = positioning::rebalance_positions(ordered.len());
    let now = envelope::now_ms();
    for ((id, _), new_pos) in ordered.iter().zip(positions.iter()) {
        sqlx::query!(
            r#"UPDATE columns SET position = ?1, updated_at = ?2 WHERE id = ?3"#,
            new_pos,
            now,
            id
        )
        .execute(&mut *conn)
        .await?;
        let column = fetch_by_id_conn(conn, id).await?.expect("column missing");
        let payload = serde_json::to_string(&column)?;
        record_change_conn(
            conn,
            ChangeOp::Update,
            "column",
            id,
            Some(&payload),
        )
        .await?;
    }
    Ok(positions)
}

async fn fetch_by_id_conn(conn: &mut SqliteConnection, id: &str) -> Result<Option<Column>> {
    sqlx::query_as!(
        Column,
        r#"
        SELECT
            id as "id!",
            board_id as "board_id!",
            name as "name!",
            position as "position!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM columns
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

pub async fn create_column(state: &AppState, board_id: &str, name: &str) -> Result<Column> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let positions = sibling_positions(&mut tx, board_id).await?;
    let position = positioning::position_at_end(&positions);
    let column = insert_column(&mut tx, board_id, name, position).await?;

    let payload = serde_json::to_string(&column)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "column",
        &column.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(column)
}

async fn insert_column(
    conn: &mut SqliteConnection,
    board_id: &str,
    name: &str,
    position: f64,
) -> Result<Column> {
    let id = envelope::new_id();
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        INSERT INTO columns (id, board_id, name, position, created_at, updated_at, deleted_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)
        "#,
        id,
        board_id,
        name,
        position,
        now,
        now
    )
    .execute(&mut *conn)
    .await?;

    Ok(Column {
        id,
        board_id: board_id.to_string(),
        name: name.to_string(),
        position,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    })
}

pub async fn list_columns(state: &AppState, board_id: &str) -> Result<Vec<Column>> {
    let columns = sqlx::query_as!(
        Column,
        r#"
        SELECT
            id as "id!",
            board_id as "board_id!",
            name as "name!",
            position as "position!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM columns
        WHERE board_id = ?1 AND deleted_at IS NULL
        ORDER BY position ASC
        "#,
        board_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(columns)
}

pub async fn rename_column(state: &AppState, id: &str, name: &str) -> Result<Column> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"UPDATE columns SET name = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"#,
        name,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let column = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("column not found".into()))?;

    let payload = serde_json::to_string(&column)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "column",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(column)
}

pub async fn reorder_column(state: &AppState, id: &str, new_position: f64) -> Result<Column> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"UPDATE columns SET position = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"#,
        new_position,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let column = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("column not found".into()))?;

    let payload = serde_json::to_string(&column)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "column",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(column)
}

pub async fn delete_column(state: &AppState, id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let column = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("column not found".into()))?;

    sqlx::query!(
        r#"UPDATE columns SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2"#,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let mut deleted = column.clone();
    deleted.deleted_at = Some(now);
    deleted.updated_at = now;
    let payload = serde_json::to_string(&deleted)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "column",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn first_column_id(state: &AppState) -> Result<Option<String>> {
    let row = sqlx::query_scalar!(
        r#"
        SELECT id as "id!" FROM columns
        WHERE deleted_at IS NULL
        ORDER BY position ASC
        LIMIT 1
        "#
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(row)
}

pub async fn rebalance_columns_for_test(
    conn: &mut SqliteConnection,
    board_id: &str,
) -> Result<Vec<f64>> {
    rebalance(conn, board_id).await
}
