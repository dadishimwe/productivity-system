use productivity_core::test_support::test_state;
use productivity_core::{create_task, delete_task, update_task};

#[tokio::test]
async fn delete_task_writes_outbox() {
    let state = test_state().await;
    let column_id = productivity_core::boards::ensure_default_column(&state)
        .await
        .unwrap();
    let task = create_task(&state, &column_id, "gone").await.unwrap();
    delete_task(&state, &task.id).await.unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM changes WHERE entity_type = 'task' AND entity_id = ?1",
    )
    .bind(&task.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn update_task_writes_outbox() {
    let state = test_state().await;
    let column_id = productivity_core::boards::ensure_default_column(&state)
        .await
        .unwrap();
    let task = create_task(&state, &column_id, "edit me").await.unwrap();
    update_task(
        &state,
        &task.id,
        "edited",
        Some("notes"),
        None,
        "in_progress",
    )
    .await
    .unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM changes WHERE entity_type = 'task' AND entity_id = ?1",
    )
    .bind(&task.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 2);
}
