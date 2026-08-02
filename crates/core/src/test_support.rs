use crate::{init_pool, AppState};

pub async fn test_state() -> AppState {
    let dir = std::env::temp_dir().join(format!(
        "productivity_test_{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    init_pool(&dir.join("test.db")).await.unwrap()
}

pub async fn change_count(state: &AppState, entity_type: &str, entity_id: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM changes WHERE entity_type = ?1 AND entity_id = ?2",
    )
    .bind(entity_type)
    .bind(entity_id)
    .fetch_one(&state.pool)
    .await
    .unwrap()
}
