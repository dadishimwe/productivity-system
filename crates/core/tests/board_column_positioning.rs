use productivity_core::{init_pool, AppState};

async fn test_state() -> AppState {
    let dir = std::env::temp_dir().join(format!(
        "productivity_board_pos_{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    init_pool(&dir.join("test.db")).await.unwrap()
}

#[tokio::test]
async fn board_rebalance_wired_through_module() {
    let state = test_state().await;
    productivity_core::boards::create_board(&state, "B1").await.unwrap();
    productivity_core::boards::create_board(&state, "B2").await.unwrap();

    let mut conn = state.pool.acquire().await.unwrap();
    let positions =
        productivity_core::boards::rebalance_boards_for_test(&mut conn)
            .await
            .unwrap();
    assert_eq!(positions, vec![0.0, 1.0]);
}

#[tokio::test]
async fn column_rebalance_wired_through_module() {
    let state = test_state().await;
    let board = productivity_core::boards::create_board(&state, "Board").await.unwrap();
    productivity_core::columns::create_column(&state, &board.id, "C1")
        .await
        .unwrap();
    productivity_core::columns::create_column(&state, &board.id, "C2")
        .await
        .unwrap();

    let mut conn = state.pool.acquire().await.unwrap();
    let positions = productivity_core::columns::rebalance_columns_for_test(&mut conn, &board.id)
        .await
        .unwrap();
    assert_eq!(positions, vec![0.0, 1.0]);
}
