use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqliteConnection};

use crate::envelope;
use crate::error::{CoreError, Result};
use crate::outbox::{record_change_conn, ChangeOp};
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GoogleAccount {
    pub id: String,
    pub email: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

async fn fetch_by_id_conn(conn: &mut SqliteConnection, id: &str) -> Result<Option<GoogleAccount>> {
    sqlx::query_as!(
        GoogleAccount,
        r#"
        SELECT
            id as "id!",
            email as "email!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM google_accounts
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Into::into)
}

pub async fn upsert_account(state: &AppState, email: &str) -> Result<GoogleAccount> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;

    let existing = sqlx::query_as!(
        GoogleAccount,
        r#"
        SELECT
            id as "id!",
            email as "email!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM google_accounts
        WHERE email = ?1 AND deleted_at IS NULL
        "#,
        email
    )
    .fetch_optional(&mut *tx)
    .await?;

    let now = envelope::now_ms();
    let is_update = existing.is_some();

    let account = if let Some(row) = existing {
        sqlx::query!(
            r#"UPDATE google_accounts SET updated_at = ?1 WHERE id = ?2"#,
            now,
            row.id
        )
        .execute(&mut *tx)
        .await?;
        fetch_by_id_conn(&mut tx, &row.id)
            .await?
            .ok_or_else(|| CoreError::Message("account missing".into()))?
    } else {
        let id = envelope::new_id();
        sqlx::query!(
            r#"
            INSERT INTO google_accounts (id, email, created_at, updated_at, deleted_at)
            VALUES (?1, ?2, ?3, ?4, NULL)
            "#,
            id,
            email,
            now,
            now
        )
        .execute(&mut *tx)
        .await?;
        GoogleAccount {
            id,
            email: email.to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    };

    let payload = serde_json::to_string(&account)?;
    let op = if is_update {
        ChangeOp::Update
    } else {
        ChangeOp::Insert
    };
    record_change_conn(
        &mut tx,
        op,
        "google_account",
        &account.id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(account)
}

pub async fn list_accounts(state: &AppState) -> Result<Vec<GoogleAccount>> {
    let rows = sqlx::query_as!(
        GoogleAccount,
        r#"
        SELECT
            id as "id!",
            email as "email!",
            created_at as "created_at!",
            updated_at as "updated_at!",
            deleted_at
        FROM google_accounts
        WHERE deleted_at IS NULL
        ORDER BY email ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(rows)
}

pub async fn disconnect_account(state: &AppState, id: &str) -> Result<()> {
    let mut conn = state.pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let now = envelope::now_ms();

    let updated = sqlx::query!(
        r#"
        UPDATE google_accounts SET deleted_at = ?1, updated_at = ?2
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
        return Err(CoreError::Message("google account not found".into()));
    }

    let account = fetch_by_id_conn(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::Message("google account not found".into()))?;
    let payload = serde_json::to_string(&account)?;
    record_change_conn(
        &mut tx,
        ChangeOp::Delete,
        "google_account",
        id,
        Some(&payload),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}
