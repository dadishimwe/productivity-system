use productivity_core::test_support::{change_count, test_state};

#[tokio::test]
async fn create_list_writes_outbox() {
    let state = test_state().await;
    let list =
        productivity_core::shopping_lists::create_list(&state, "Groceries", Some(10_000))
            .await
            .unwrap();
    assert_eq!(change_count(&state, "shopping_list", &list.id).await, 1);
}

#[tokio::test]
async fn create_item_writes_outbox() {
    let state = test_state().await;
    let list = productivity_core::shopping_lists::create_list(&state, "Trip", None)
        .await
        .unwrap();
    let item = productivity_core::shopping_items::create_item(
        &state,
        &list.id,
        "Milk",
        1.0,
        Some("L"),
        Some(399),
        None,
    )
    .await
    .unwrap();
    assert_eq!(change_count(&state, "shopping_item", &item.id).await, 1);
}

#[tokio::test]
async fn delete_list_cascades_items_outbox() {
    let state = test_state().await;
    let list = productivity_core::shopping_lists::create_list(&state, "Trip", None)
        .await
        .unwrap();
    let item = productivity_core::shopping_items::create_item(
        &state,
        &list.id,
        "Bread",
        1.0,
        None,
        Some(250),
        None,
    )
    .await
    .unwrap();
    productivity_core::shopping_lists::delete_list(&state, &list.id)
        .await
        .unwrap();
    assert!(change_count(&state, "shopping_list", &list.id).await >= 2);
    assert!(change_count(&state, "shopping_item", &item.id).await >= 2);
}
