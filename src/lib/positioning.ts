/** Mirrors `crates/core/src/positioning.rs` for client-side drop targets. */
export function positionBetween(
  before: number | null,
  after: number | null,
): number {
  if (before === null && after === null) return 0;
  if (before === null) return after! - 1;
  if (after === null) return before + 1;
  return (before + after) / 2;
}

export function positionAtEnd(positions: number[]): number {
  if (positions.length === 0) return 0;
  return positions[positions.length - 1] + 1;
}
