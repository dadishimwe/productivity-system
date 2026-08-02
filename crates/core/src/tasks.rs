use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::Result;
use crate::outbox::{record_change_conn, ChangeOp};
use crate::positioning;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: f64,
    pub due_date: Option<i64>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn sibling_positions(
    conn: &mut SqliteConnection,
    column_id: &str,
) -> Result<Vec<f64>> {
    let rows = sqlx::query!(
        r#"
        SELECT position as "position!: f64"
        FROM tasks
        WHERE column_id = ?1 AND deleted_at IS NULL
        ORDER BY position ASC
        "#,
        column_id
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(|r| r.position).collect())
}

async fn siblings(conn: &mut SqliteConnection, column_id: &str) -> Result<Vec<(String, f64)>> {
    let rows = sqlx::query!(
        r#"
        SELECT id as "id!", position as "position!: f64"
        FROM tasks
        WHERE column_id = ?1 AND deleted_at IS NULL
        ORDER BY position ASC
        "#,
        column_id
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(|r| (r.id, r.position)).collect())
}

async fn rebalance(conn: &mut SqliteConnection, column_id: &str) -> Result<Vec<f64>> {
    let ordered = siblings(conn, column_id).await?;
    let positions = positioning::rebalance_positions(ordered.len());
    let now = envelope::now_ms();
    for ((id, _), new_pos) in ordered.iter().zip(positions.iter()) {
        sqlx::query!(
            r#"UPDATE tasks SET position = ?1, updated_at = ?2 WHERE id = ?3"#,
            new_pos,
            now,
            id
        )
        .execute(&mut *conn)
        .await?;
        let task = fetch_by_id_conn(conn, id).await?.expect("task missing");
        let payload = serde_json::to_string(&task)?;
        record_change_conn(conn, ChangeOp::Update, "task", id, Some(&payload)).await?;
    }
    Ok(positions)
}

async fn fetch_by_id_conn(conn: &mut SqliteConnection, id: &str) -> Result<Option<Task>> {
    sqlx::query_as!(
        Task,
        r#"
        SELECT
            id as "id!",
            column_id as "column_id!",
            title as "title!",
            description,
            position as "position!",
            due_date,
            status as "status!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM tasks
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

async fn insert_task(
    conn: &mut SqliteConnection,
    column_id: &str,
    title: &str,
    position: f64,
) -> Result<Task> {
    let id = envelope::new_id();
    let now = envelope::now_ms();
    let status = "open";

    sqlx::query!(
        r#"
        INSERT INTO tasks (
            id, column_id, title, description, position, due_date, status,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, NULL, ?4, NULL, ?5, ?6, ?7, NULL)
        "#,
        id,
        column_id,
        title,
        position,
        status,
        now,
        now
    )
    .execute(&mut *conn)
    .await?;

    Ok(Task {
        id,
        column_id: column_id.to_string(),
        title: title.to_string(),
        description: None,
        position,
        due_date: None,
        status: status.to_string(),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    })
}

pub async fn create(state: &AppState, column_id: &str, title: &str) -> Result<Task> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let positions = sibling_positions(&mut tx, column_id).await?;
    let position = positioning::position_at_end(&positions);
    let task = insert_task(&mut tx, column_id, title, position).await?;

    let payload = serde_json::to_string(&task)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "task",
        &task.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(task)
}

pub async fn list(state: &AppState, column_id: &str) -> Result<Vec<Task>> {
    let tasks = sqlx::query_as!(
        Task,
        r#"
        SELECT
            id as "id!",
            column_id as "column_id!",
            title as "title!",
            description,
            position as "position!",
            due_date,
            status as "status!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM tasks
        WHERE column_id = ?1 AND deleted_at IS NULL
        ORDER BY position ASC
        "#,
        column_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(tasks)
}

pub async fn move_task(
    state: &AppState,
    task_id: &str,
    new_column_id: &str,
    new_position: f64,
) -> Result<Task> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        UPDATE tasks
        SET column_id = ?1, position = ?2, updated_at = ?3
        WHERE id = ?4 AND deleted_at IS NULL
        "#,
        new_column_id,
        new_position,
        now,
        task_id
    )
    .execute(&mut *tx)
    .await?;

    let task = fetch_by_id_conn(&mut tx, task_id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("task not found".into()))?;

    let payload = serde_json::to_string(&task)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "task",
        task_id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(task)
}

pub async fn insert_between(
    state: &AppState,
    column_id: &str,
    title: &str,
    before: f64,
    after: f64,
) -> Result<Task> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let position = match positioning::try_position_between(before, after) {
        Ok(p) => p,
        Err(positioning::RebalanceRequired) => {
            rebalance(&mut tx, column_id).await?;
            let positions = sibling_positions(&mut tx, column_id).await?;
            positioning::position_from_anchors(&positions, before, after)
        }
    };

    let task = insert_task(&mut tx, column_id, title, position).await?;
    let payload = serde_json::to_string(&task)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "task",
        &task.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(task)
}

pub async fn rebalance_tasks_for_test(
    conn: &mut SqliteConnection,
    column_id: &str,
) -> Result<Vec<f64>> {
    rebalance(conn, column_id).await
}
