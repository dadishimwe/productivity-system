use productivity_core::{create_task, init_pool, list_tasks, move_task, AppState};

async fn test_state() -> AppState {
    let dir = std::env::temp_dir().join(format!(
        "productivity_task_pos_{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    init_pool(&dir.join("test.db")).await.unwrap()
}

#[tokio::test]
async fn rebalance_preserves_task_order() {
    let state = test_state().await;
    let column_id = productivity_core::boards::ensure_default_column(&state)
        .await
        .unwrap();

    create_task(&state, &column_id, "A").await.unwrap();
    create_task(&state, &column_id, "B").await.unwrap();
    create_task(&state, &column_id, "C").await.unwrap();

    let before = list_tasks(&state, &column_id).await.unwrap();
    let titles_before: Vec<_> = before.iter().map(|t| t.title.as_str()).collect();

    let mut conn = state.pool.acquire().await.unwrap();
    productivity_core::tasks::rebalance_tasks_for_test(&mut conn, &column_id)
        .await
        .unwrap();
    drop(conn);

    let after = list_tasks(&state, &column_id).await.unwrap();
    assert_eq!(
        after.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
        titles_before
    );
}

#[tokio::test]
async fn insert_between_uses_shared_positioning() {
    let state = test_state().await;
    let column_id = productivity_core::boards::ensure_default_column(&state)
        .await
        .unwrap();
    let t1 = create_task(&state, &column_id, "first").await.unwrap();
    let t2 = create_task(&state, &column_id, "second").await.unwrap();
    let mid = productivity_core::tasks::insert_between(
        &state,
        &column_id,
        "middle",
        t1.position,
        t2.position,
    )
    .await
    .unwrap();
    assert!(mid.position > t1.position && mid.position < t2.position);
}

#[tokio::test]
async fn move_task_persists_order_after_reload() {
    let state = test_state().await;
    let column_id = productivity_core::boards::ensure_default_column(&state)
        .await
        .unwrap();
    let t1 = create_task(&state, &column_id, "A").await.unwrap();
    let t2 = create_task(&state, &column_id, "B").await.unwrap();
    let new_pos = (t1.position + t2.position) / 2.0;
    move_task(&state, &t2.id, &column_id, new_pos)
        .await
        .unwrap();
    let tasks = list_tasks(&state, &column_id).await.unwrap();
    assert_eq!(tasks[0].title, "A");
    assert_eq!(tasks[1].title, "B");
    assert!(tasks[1].position > tasks[0].position);
}
