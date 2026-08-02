use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::Result;
use crate::outbox::{record_change_conn, ChangeOp};
use crate::AppState;

/// Per-line total in cents: round(qty * unit_price) before summing.
pub fn line_total_cents(qty: f64, unit_price_cents: i64) -> i64 {
    (qty * unit_price_cents as f64).round() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSummary {
    pub total_cents: i64,
    pub item_count: i32,
    pub checked_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ShoppingItem {
    pub id: String,
    pub list_id: String,
    pub name: String,
    pub qty: f64,
    pub unit: Option<String>,
    pub unit_price: Option<i64>,
    pub checked: i64,
    pub category: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

pub(crate) async fn fetch_by_id_conn(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<ShoppingItem>> {
    sqlx::query_as!(
        ShoppingItem,
        r#"
        SELECT
            id as "id!",
            list_id as "list_id!",
            name as "name!",
            qty as "qty!",
            unit,
            unit_price,
            checked as "checked!",
            category,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM shopping_items
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

pub async fn create_item(
    state: &AppState,
    list_id: &str,
    name: &str,
    qty: f64,
    unit: Option<&str>,
    unit_price: Option<i64>,
    category: Option<&str>,
) -> Result<ShoppingItem> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let id = envelope::new_id();
    let now = envelope::now_ms();
    let checked = 0_i64;

    sqlx::query!(
        r#"
        INSERT INTO shopping_items (
            id, list_id, name, qty, unit, unit_price, checked, category,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)
        "#,
        id,
        list_id,
        name,
        qty,
        unit,
        unit_price,
        checked,
        category,
        now,
        now
    )
    .execute(&mut *tx)
    .await?;

    let item = ShoppingItem {
        id,
        list_id: list_id.to_string(),
        name: name.to_string(),
        qty,
        unit: unit.map(str::to_string),
        unit_price,
        checked,
        category: category.map(str::to_string),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    let payload = serde_json::to_string(&item)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Insert,
        "shopping_item",
        &item.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(item)
}

pub async fn list_items(state: &AppState, list_id: &str) -> Result<Vec<ShoppingItem>> {
    let items = sqlx::query_as!(
        ShoppingItem,
        r#"
        SELECT
            id as "id!",
            list_id as "list_id!",
            name as "name!",
            qty as "qty!",
            unit,
            unit_price,
            checked as "checked!",
            category,
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM shopping_items
        WHERE list_id = ?1 AND deleted_at IS NULL
        ORDER BY created_at ASC
        "#,
        list_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(items)
}

pub async fn update_item(
    state: &AppState,
    id: &str,
    name: &str,
    qty: f64,
    unit: Option<&str>,
    unit_price: Option<i64>,
    category: Option<&str>,
) -> Result<ShoppingItem> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    sqlx::query!(
        r#"
        UPDATE shopping_items
        SET name = ?1, qty = ?2, unit = ?3, unit_price = ?4, category = ?5, updated_at = ?6
        WHERE id = ?7 AND deleted_at IS NULL
        "#,
        name,
        qty,
        unit,
        unit_price,
        category,
        now,
        id
    )
    .execute(&mut *tx)
    .await?;

    let item = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("item not found".into()))?;

    let payload = serde_json::to_string(&item)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "shopping_item",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(item)
}

pub async fn toggle_checked(state: &AppState, item_id: &str) -> Result<ShoppingItem> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let current = fetch_by_id_conn(&mut tx, item_id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("item not found".into()))?;
    let new_checked = if current.checked != 0 { 0 } else { 1 };

    sqlx::query!(
        r#"UPDATE shopping_items SET checked = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"#,
        new_checked,
        now,
        item_id
    )
    .execute(&mut *tx)
    .await?;

    let item = fetch_by_id_conn(&mut tx, item_id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("item not found".into()))?;

    let payload = serde_json::to_string(&item)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Update,
        "shopping_item",
        item_id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(item)
}

pub async fn delete_item(state: &AppState, item_id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let item = fetch_by_id_conn(&mut tx, item_id)
        .await?
        .ok_or_else(|| crate::error::CoreError::Message("item not found".into()))?;

    sqlx::query!(
        r#"UPDATE shopping_items SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2"#,
        now,
        item_id
    )
    .execute(&mut *tx)
    .await?;

    let mut deleted = item.clone();
    deleted.deleted_at = Some(now);
    deleted.updated_at = now;
    let payload = serde_json::to_string(&deleted)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "shopping_item",
        item_id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_list_summary(state: &AppState, list_id: &str) -> Result<ListSummary> {
    let items = list_items(state, list_id).await?;
    let mut total_cents = 0_i64;
    let mut checked_count = 0_i32;
    for item in &items {
        if item.checked != 0 {
            checked_count += 1;
        }
        if let Some(price) = item.unit_price {
            total_cents += line_total_cents(item.qty, price);
        }
    }
    Ok(ListSummary {
        total_cents,
        item_count: items.len() as i32,
        checked_count,
    })
}
