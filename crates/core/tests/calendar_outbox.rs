use productivity_core::test_support::{change_count, test_state};

#[tokio::test]
async fn create_event_writes_outbox() {
    let state = test_state().await;
    let calendar = productivity_core::calendars::ensure_default_calendar(&state)
        .await
        .unwrap();
    let start = 1_700_000_000_000_i64;
    let end = start + 3_600_000;
    let event = productivity_core::events::create_event(
        &state,
        &calendar.id,
        "Meet",
        None,
        start,
        end,
        false,
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        change_count(&state, "event", &event.id).await,
        1,
        "insert should write one change"
    );
}

#[tokio::test]
async fn delete_calendar_cascades_events_to_outbox() {
    let state = test_state().await;
    let calendar = productivity_core::calendars::create_calendar(&state, "Temp", None)
        .await
        .unwrap();
    let event = productivity_core::events::create_event(
        &state,
        &calendar.id,
        "X",
        None,
        1_700_000_000_000,
        1_700_000_360_000,
        false,
        None,
    )
    .await
    .unwrap();

    productivity_core::calendars::delete_calendar(&state, &calendar.id)
        .await
        .unwrap();

    assert!(
        change_count(&state, "calendar", &calendar.id).await >= 1,
        "calendar delete recorded"
    );
    assert!(
        change_count(&state, "event", &event.id).await >= 2,
        "event insert + delete"
    );
}
