use productivity_core::test_support::test_state;

#[tokio::test]
async fn log_habit_twice_upserts_single_row() {
    let state = test_state().await;
    let habit = productivity_core::habits::create_habit(&state, "Meditate", None, None)
        .await
        .unwrap();

    let first = productivity_core::habit_logs::log_habit(&state, &habit.id, "2026-01-15", 1)
        .await
        .unwrap();
    let second = productivity_core::habit_logs::log_habit(&state, &habit.id, "2026-01-15", 3)
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.value, 3);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM habit_logs WHERE habit_id = ?1 AND date = ?2",
    )
    .bind(&habit.id)
    .bind("2026-01-15")
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn log_and_unlog_writes_outbox() {
    let state = test_state().await;
    let habit = productivity_core::habits::create_habit(&state, "Read", None, None)
        .await
        .unwrap();
    let log = productivity_core::habit_logs::log_habit(&state, &habit.id, "2026-02-01", 1)
        .await
        .unwrap();
    productivity_core::habit_logs::unlog_habit(&state, &habit.id, "2026-02-01")
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM changes WHERE entity_type = 'habit_log' AND entity_id = ?1",
    )
    .bind(&log.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 2);
}
