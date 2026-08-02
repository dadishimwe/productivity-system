use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::Result;
use crate::outbox::{record_change_conn, ChangeOp};
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ShoppingList {
    pub id: String,
    pub name: String,
    pub budget_limit: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn fetch_by_id_conn(conn: &mut SqliteConnection, id: &str) -> Result<Option<ShoppingList>> {
    sqlx::query_as!(
        ShoppingList,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            budget_limit,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM shopping_lists
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

pub async fn create_list(
    state: &AppState,
    name: &str,
    budget_limit: Option<i64>,
) -> Result<ShoppingList> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let id = envelope::new_id();
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        INSERT INTO shopping_lists (id, name, budget_limit, created_at, updated_at, deleted_at)
        VALUES (?1, ?2, ?3, ?4, ?5, NULL)
        "#,
        id,
        name,
        budget_limit,
        now,
        now
    )
    .execute(&mut *tx)
    .await?;

    let list = ShoppingList {
        id,
        name: name.to_string(),
        budget_limit,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    let payload = serde_json::to_string(&list)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "shopping_list",
        &list.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(list)
}

pub async fn list_lists(state: &AppState) -> Result<Vec<ShoppingList>> {
    let lists = sqlx::query_as!(
        ShoppingList,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            budget_limit,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM shopping_lists
        WHERE deleted_at IS NULL
        ORDER BY name ASC
        "#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(lists)
}

pub async fn rename_list(state: &AppState, id: &str, name: &str) -> Result<ShoppingList> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"UPDATE shopping_lists SET name = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"#,
        name,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let list = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("list not found".into()))?;

    let payload = serde_json::to_string(&list)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "shopping_list",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(list)
}

pub async fn set_budget(
    state: &AppState,
    list_id: &str,
    budget_limit: Option<i64>,
) -> Result<ShoppingList> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"UPDATE shopping_lists SET budget_limit = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"#,
        budget_limit,
        now,
        list_id
    )
    .execute(&mut *tx)
    .await?;

    let list = fetch_by_id_conn(&mut tx, list_id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("list not found".into()))?;

    let payload = serde_json::to_string(&list)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "shopping_list",
        list_id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(list)
}

pub async fn delete_list(state: &AppState, id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let list = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("list not found".into()))?;

    sqlx::query!(
        r#"UPDATE shopping_lists SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2"#,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let mut deleted_list = list.clone();
    deleted_list.deleted_at = Some(now);
    deleted_list.updated_at = now;
    let payload = serde_json::to_string(&deleted_list)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "shopping_list",
        id,
        Some(&payload),
    )
    .await?;

    let items = sqlx::query!(
        r#"
        SELECT id as "id!" FROM shopping_items
        WHERE list_id = ?1 AND deleted_at IS NULL
        "#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;

    for row in items {
        sqlx::query!(
            r#"UPDATE shopping_items SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2"#,
            now,
            row.id
        )
        .execute(&mut *tx)
        .await?;

        let item = crate::shopping_items::fetch_by_id_conn(&mut tx, &row.id)
            .await?
            .ok_or_else(|| crate::error::CoreError::Message("item missing".into()))?;
        let item_payload = serde_json::to_string(&item)?;
        record_change_conn(
            &mut tx,
            ChangeOp::Delete,
            "shopping_item",
            &row.id,
            Some(&item_payload),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
