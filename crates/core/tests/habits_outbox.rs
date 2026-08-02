use productivity_core::test_support::{change_count, test_state};

#[tokio::test]
async fn create_habit_writes_outbox() {
    let state = test_state().await;
    let habit = productivity_core::habits::create_habit(
        &state,
        "Run",
        Some("#22c55e"),
        Some("daily"),
    )
    .await
    .unwrap();
    assert_eq!(change_count(&state, "habit", &habit.id).await, 1);
}

#[tokio::test]
async fn update_habit_writes_outbox() {
    let state = test_state().await;
    let habit = productivity_core::habits::create_habit(&state, "Run", None, None)
        .await
        .unwrap();
    productivity_core::habits::update_habit(&state, &habit.id, "Jog", None, None)
        .await
        .unwrap();
    assert!(change_count(&state, "habit", &habit.id).await >= 2);
}

#[tokio::test]
async fn delete_habit_writes_outbox() {
    let state = test_state().await;
    let habit = productivity_core::habits::create_habit(&state, "Run", None, None)
        .await
        .unwrap();
    productivity_core::habits::delete_habit(&state, &habit.id)
        .await
        .unwrap();
    assert!(change_count(&state, "habit", &habit.id).await >= 2);
}
