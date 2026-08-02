use productivity_core::test_support::test_state;

#[tokio::test]
async fn weekly_recurrence_expands_in_range() {
    let state = test_state().await;
    let calendar = productivity_core::calendars::ensure_default_calendar(&state)
        .await
        .unwrap();
    let start = 1_735_689_600_000_i64; // fixed local-ish Monday anchor for test
    productivity_core::events::create_event(
        &state,
        &calendar.id,
        "Standup",
        None,
        start,
        start + 3_600_000,
        false,
        Some("FREQ=WEEKLY;BYDAY=MO"),
    )
    .await
    .unwrap();

    let range_end = start + 22 * 24 * 3_600_000;
    let occs = productivity_core::events::list_occurrences(
        &state,
        &calendar.id,
        start,
        range_end,
    )
    .await
    .unwrap();

    assert!(occs.len() >= 3, "expected at least 3 weekly occurrences");
}

#[tokio::test]
async fn cancelled_exception_excludes_occurrence() {
    let state = test_state().await;
    let calendar = productivity_core::calendars::ensure_default_calendar(&state)
        .await
        .unwrap();
    let start = 1_735_689_600_000_i64;
    let event = productivity_core::events::create_event(
        &state,
        &calendar.id,
        "Daily",
        None,
        start,
        start + 3_600_000,
        false,
        Some("FREQ=DAILY"),
    )
    .await
    .unwrap();

    let occs_before = productivity_core::events::list_occurrences(
        &state,
        &calendar.id,
        start,
        start + 5 * 24 * 3_600_000,
    )
    .await
    .unwrap();
    assert!(occs_before.len() >= 2);
    let second_start = occs_before[1].original_start_ms;

    productivity_core::events::delete_occurrence(
        &state,
        &event.id,
        second_start,
        productivity_core::occurrences::OccurrenceScope::This,
    )
    .await
    .unwrap();

    let occs = productivity_core::events::list_occurrences(
        &state,
        &calendar.id,
        start,
        start + 5 * 24 * 3_600_000,
    )
    .await
    .unwrap();

    assert!(
        !occs.iter().any(|o| o.original_start_ms == second_start),
        "cancelled instance should not appear"
    );
}
