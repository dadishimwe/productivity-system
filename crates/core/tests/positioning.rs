use productivity_core::positioning::{
    position_after_last, position_before_first, position_between, rebalance_positions,
    try_position_between, RebalanceRequired, POSITION_EPSILON,
};

#[test]
fn insert_between_two_positions_uses_midpoint() {
    let mid = position_between(0.0, 10.0);
    assert!((mid - 5.0).abs() < f64::EPSILON);
}

#[test]
fn insert_at_start_before_first() {
    let first = 2.0;
    let pos = position_before_first(first);
    assert!((pos - 1.0).abs() < f64::EPSILON);
    assert!(pos < first);
}

#[test]
fn insert_at_end_after_last() {
    let last = 5.0;
    let pos = position_after_last(last);
    assert!((pos - 6.0).abs() < f64::EPSILON);
    assert!(pos > last);
}

#[test]
fn repeated_midpoints_trigger_rebalance() {
    let mut low = 0.0_f64;
    let high = 1.0_f64;
    loop {
        match try_position_between(low, high) {
            Ok(mid) => {
                low = mid;
                if (high - low).abs() < POSITION_EPSILON {
                    break;
                }
            }
            Err(RebalanceRequired) => break,
        }
    }
    assert!(try_position_between(low, high).is_err());
}

#[test]
fn rebalance_assigns_integer_spacing() {
    assert_eq!(rebalance_positions(3), vec![0.0, 1.0, 2.0]);
}
