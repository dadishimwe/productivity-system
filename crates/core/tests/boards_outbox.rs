use productivity_core::test_support::{change_count, test_state};

#[tokio::test]
async fn create_board_writes_outbox() {
    let state = test_state().await;
    let board = productivity_core::boards::create_board(&state, "Work").await.unwrap();
    assert_eq!(change_count(&state, "board", &board.id).await, 1);
}

#[tokio::test]
async fn rename_board_writes_outbox() {
    let state = test_state().await;
    let board = productivity_core::boards::create_board(&state, "Work").await.unwrap();
    productivity_core::boards::rename_board(&state, &board.id, "Personal")
        .await
        .unwrap();
    assert!(change_count(&state, "board", &board.id).await >= 2);
}

#[tokio::test]
async fn delete_board_writes_outbox() {
    let state = test_state().await;
    let board = productivity_core::boards::create_board(&state, "Work").await.unwrap();
    productivity_core::boards::delete_board(&state, &board.id)
        .await
        .unwrap();
    assert!(change_count(&state, "board", &board.id).await >= 2);
}

#[tokio::test]
async fn create_column_writes_outbox() {
    let state = test_state().await;
    let board = productivity_core::boards::create_board(&state, "B").await.unwrap();
    let col = productivity_core::columns::create_column(&state, &board.id, "Todo")
        .await
        .unwrap();
    assert_eq!(change_count(&state, "column", &col.id).await, 1);
}
