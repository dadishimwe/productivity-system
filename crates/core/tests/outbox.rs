use productivity_core::test_support::test_state;
use productivity_core::{create_task, list_tasks};

#[tokio::test]
async fn create_task_writes_outbox_in_same_transaction() {
    let state = test_state().await;
    let column_id = productivity_core::boards::ensure_default_column(&state)
        .await
        .unwrap();

    let task = create_task(&state, &column_id, "Outbox me").await.unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM changes WHERE entity_type = 'task' AND entity_id = ?1",
    )
    .bind(&task.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();

    assert_eq!(count, 1);

    let list = list_tasks(&state, &column_id).await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn migrations_are_idempotent_on_rerun() {
    let state = test_state().await;
    productivity_core::db::run_migrations(&state.pool)
        .await
        .unwrap();
}
