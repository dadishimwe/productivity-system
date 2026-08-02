use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::columns;
use crate::envelope;
use crate::error::Result;
use crate::outbox::{record_change_conn, ChangeOp};
use crate::positioning;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub position: f64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn sibling_positions(conn: &mut SqliteConnection) -> Result<Vec<f64>> {
    let rows = sqlx::query!(
        r#"
        SELECT position as "position!: f64"
        FROM boards
        WHERE deleted_at IS NULL
        ORDER BY position ASC
        "#
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(|r| r.position).collect())
}

async fn siblings(conn: &mut SqliteConnection) -> Result<Vec<(String, f64)>> {
    let rows = sqlx::query!(
        r#"
        SELECT id as "id!", position as "position!: f64"
        FROM boards
        WHERE deleted_at IS NULL
        ORDER BY position ASC
        "#
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(|r| (r.id, r.position)).collect())
}

async fn rebalance(conn: &mut SqliteConnection) -> Result<Vec<f64>> {
    let ordered = siblings(conn).await?;
    let positions = positioning::rebalance_positions(ordered.len());
    let now = envelope::now_ms();
    for ((id, _), new_pos) in ordered.iter().zip(positions.iter()) {
        sqlx::query!(
            r#"UPDATE boards SET position = ?1, updated_at = ?2 WHERE id = ?3"#,
            new_pos,
            now,
            id
        )
        .execute(&mut *conn)
        .await?;
        let board = fetch_by_id_conn(conn, id).await?.expect("board missing after rebalance");
        let payload = serde_json::to_string(&board)?;
        record_change_conn(
            conn,
            ChangeOp::Update,
            "board",
            id,
            Some(&payload),
        )
        .await?;
    }
    Ok(positions)
}

async fn fetch_by_id_conn(conn: &mut SqliteConnection, id: &str) -> Result<Option<Board>> {
    let row = sqlx::query_as!(
        Board,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            position as "position!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM boards
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row)
}

pub async fn create_board(state: &AppState, name: &str) -> Result<Board> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let positions = sibling_positions(&mut tx).await?;
    let position = positioning::position_at_end(&positions);
    let board = insert_board(&mut tx, name, position).await?;

    let payload = serde_json::to_string(&board)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "board",
        &board.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(board)
}

async fn insert_board(
    conn: &mut SqliteConnection,
    name: &str,
    position: f64,
) -> Result<Board> {
    let id = envelope::new_id();
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        INSERT INTO boards (id, name, position, created_at, updated_at, deleted_at)
        VALUES (?1, ?2, ?3, ?4, ?5, NULL)
        "#,
        id,
        name,
        position,
        now,
        now
    )
    .execute(&mut *conn)
    .await?;

    Ok(Board {
        id,
        name: name.to_string(),
        position,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    })
}

pub async fn list_boards(state: &AppState) -> Result<Vec<Board>> {
    let boards = sqlx::query_as!(
        Board,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            position as "position!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM boards
        WHERE deleted_at IS NULL
        ORDER BY position ASC
        "#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(boards)
}

pub async fn rename_board(state: &AppState, id: &str, name: &str) -> Result<Board> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"UPDATE boards SET name = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"#,
        name,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let board = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("board not found".into()))?;

    let payload = serde_json::to_string(&board)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "board",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(board)
}

pub async fn reorder_board(state: &AppState, id: &str, new_position: f64) -> Result<Board> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"UPDATE boards SET position = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"#,
        new_position,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let board = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("board not found".into()))?;

    let payload = serde_json::to_string(&board)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "board",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(board)
}

pub async fn delete_board(state: &AppState, id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let board = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("board not found".into()))?;

    sqlx::query!(
        r#"UPDATE boards SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2"#,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let mut deleted = board.clone();
    deleted.deleted_at = Some(now);
    deleted.updated_at = now;
    let payload = serde_json::to_string(&deleted)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "board",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn ensure_default_column(state: &AppState) -> Result<String> {
    if let Some(id) = columns::first_column_id(state).await? {
        return Ok(id);
    }
    let board = create_board(state, "Default board").await?;
    let column = columns::create_column(state, &board.id, "To do").await?;
    Ok(column.id)
}

pub async fn rebalance_boards_for_test(conn: &mut SqliteConnection) -> Result<Vec<f64>> {
    rebalance(conn).await
}
