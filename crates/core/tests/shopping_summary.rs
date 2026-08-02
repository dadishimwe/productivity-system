use productivity_core::shopping_items::{get_list_summary, line_total_cents};
use productivity_core::test_support::test_state;

#[test]
fn line_total_rounds_per_line_before_sum() {
    assert_eq!(line_total_cents(3.0, 33), 99);
    assert_eq!(line_total_cents(1.0, 10) + line_total_cents(1.0, 20), 30);
}

#[tokio::test]
async fn summary_sums_per_line_totals() {
    let state = test_state().await;
    let list = productivity_core::shopping_lists::create_list(&state, "Test", None)
        .await
        .unwrap();
    productivity_core::shopping_items::create_item(
        &state,
        &list.id,
        "A",
        3.0,
        None,
        Some(33),
        None,
    )
    .await
    .unwrap();
    productivity_core::shopping_items::create_item(
        &state,
        &list.id,
        "B",
        1.0,
        None,
        Some(10),
        None,
    )
    .await
    .unwrap();
    productivity_core::shopping_items::create_item(
        &state,
        &list.id,
        "C",
        1.0,
        None,
        Some(20),
        None,
    )
    .await
    .unwrap();

    let summary = get_list_summary(&state, &list.id).await.unwrap();
    assert_eq!(summary.total_cents, 99 + 10 + 20);
    assert_eq!(summary.item_count, 3);
    assert_eq!(summary.checked_count, 0);
}

#[tokio::test]
async fn toggle_checked_updates_summary_and_outbox() {
    let state = test_state().await;
    let list = productivity_core::shopping_lists::create_list(&state, "Test", None)
        .await
        .unwrap();
    let item = productivity_core::shopping_items::create_item(
        &state,
        &list.id,
        "X",
        1.0,
        None,
        Some(100),
        None,
    )
    .await
    .unwrap();

    productivity_core::shopping_items::toggle_checked(&state, &item.id)
        .await
        .unwrap();
    let summary = get_list_summary(&state, &list.id).await.unwrap();
    assert_eq!(summary.checked_count, 1);
    assert_eq!(summary.total_cents, 100);

    let changes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM changes WHERE entity_type = 'shopping_item' AND entity_id = ?1",
    )
    .bind(&item.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(changes, 2);
}
