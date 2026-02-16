use super::normalized_active_index_after_remove;

#[test]
fn remove_only_tab_returns_none() {
    assert_eq!(normalized_active_index_after_remove(0, 1, 0), None);
}

#[test]
fn remove_tab_before_active_shifts_left() {
    assert_eq!(normalized_active_index_after_remove(3, 5, 1), Some(2));
}

#[test]
fn remove_active_tab_clamps_to_existing_index() {
    assert_eq!(normalized_active_index_after_remove(4, 5, 4), Some(3));
    assert_eq!(normalized_active_index_after_remove(1, 5, 1), Some(1));
}

#[test]
fn invalid_removed_index_returns_none() {
    assert_eq!(normalized_active_index_after_remove(0, 0, 0), None);
    assert_eq!(normalized_active_index_after_remove(0, 3, 3), None);
}
