use sqlx::SqliteConnection;

use crate::error::Result;

pub const POSITION_EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebalanceRequired;

pub fn position_between(before: f64, after: f64) -> f64 {
    (before + after) / 2.0
}

pub fn position_before_first(first: f64) -> f64 {
    first - 1.0
}

pub fn position_after_last(last: f64) -> f64 {
    last + 1.0
}

pub fn should_rebalance(before: f64, after: f64) -> bool {
    (after - before).abs() < POSITION_EPSILON
}

pub fn try_position_between(
    before: f64,
    after: f64,
) -> std::result::Result<f64, RebalanceRequired> {
    if should_rebalance(before, after) {
        return Err(RebalanceRequired);
    }
    Ok(position_between(before, after))
}

pub fn rebalance_positions(count: usize) -> Vec<f64> {
    (0..count).map(|i| i as f64).collect()
}

/// Next position when appending to an ordered sibling list.
pub fn position_at_end(sorted_positions: &[f64]) -> f64 {
    match sorted_positions.last() {
        Some(&last) => position_after_last(last),
        None => 0.0,
    }
}

/// After a rebalance (or when neighbors still fit), pick a position between anchors.
pub fn position_from_anchors(sorted_positions: &[f64], before: f64, after: f64) -> f64 {
    if let Ok(p) = try_position_between(before, after) {
        return p;
    }
    let idx = sorted_positions
        .iter()
        .position(|&p| p > before)
        .unwrap_or(sorted_positions.len());
    if idx == 0 {
        if sorted_positions.is_empty() {
            0.0
        } else {
            position_before_first(sorted_positions[0])
        }
    } else if idx >= sorted_positions.len() {
        position_after_last(*sorted_positions.last().unwrap())
    } else {
        position_between(sorted_positions[idx - 1], sorted_positions[idx])
    }
}

/// Resolve a target position between two neighbors, rebalance siblings when the gap is too small.
pub async fn resolve_position_between<F, Fut>(
    conn: &mut SqliteConnection,
    sorted_siblings: &[(String, f64)],
    before: f64,
    after: f64,
    mut rebalance: F,
) -> Result<f64>
where
    F: FnMut(&mut SqliteConnection, &[(String, f64)]) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<f64>>>,
{
    match try_position_between(before, after) {
        Ok(p) => Ok(p),
        Err(RebalanceRequired) => {
            let positions = rebalance(conn, sorted_siblings).await?;
            Ok(position_from_anchors(&positions, before, after))
        }
    }
}
