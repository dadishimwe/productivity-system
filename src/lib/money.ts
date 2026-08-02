/** Format integer cents for display only. */
export function formatMoney(cents: number): string {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.abs(cents);
  const dollars = Math.floor(abs / 100);
  const remainder = abs % 100;
  return `${sign}$${dollars}.${String(remainder).padStart(2, "0")}`;
}

/** Parse user input like "42.50" or "42" to cents; null if invalid/empty. */
export function parseMoneyToCents(input: string): number | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  const m = trimmed.match(/^(\d+)(?:\.(\d{1,2}))?$/);
  if (!m) return null;
  const dollars = parseInt(m[1], 10);
  const frac = (m[2] ?? "").padEnd(2, "0").slice(0, 2);
  return dollars * 100 + parseInt(frac, 10);
}
