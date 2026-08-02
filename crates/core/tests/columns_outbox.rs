use productivity_core::test_support::{change_count, test_state};

#[tokio::test]
async fn rename_column_writes_outbox() {
    let state = test_state().await;
    let board = productivity_core::boards::create_board(&state, "B").await.unwrap();
    let col = productivity_core::columns::create_column(&state, &board.id, "Todo")
        .await
        .unwrap();
    productivity_core::columns::rename_column(&state, &col.id, "Doing")
        .await
        .unwrap();
    assert!(change_count(&state, "column", &col.id).await >= 2);
}

#[tokio::test]
async fn delete_column_writes_outbox() {
    let state = test_state().await;
    let board = productivity_core::boards::create_board(&state, "B").await.unwrap();
    let col = productivity_core::columns::create_column(&state, &board.id, "Todo")
        .await
        .unwrap();
    productivity_core::columns::delete_column(&state, &col.id)
        .await
        .unwrap();
    assert!(change_count(&state, "column", &col.id).await >= 2);
}
