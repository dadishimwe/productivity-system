export function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export function addDays(d: Date, n: number): Date {
  const x = new Date(d);
  x.setDate(x.getDate() + n);
  return x;
}

/** Inclusive range of YYYY-MM-DD strings. */
export function dateRange(from: Date, to: Date): string[] {
  const out: string[] = [];
  let cur = new Date(from);
  while (cur <= to) {
    out.push(formatDate(cur));
    cur = addDays(cur, 1);
  }
  return out;
}

export function lastMonthsRange(months: number): { from: string; to: string } {
  const to = new Date();
  const from = new Date(to);
  from.setMonth(from.getMonth() - months);
  return { from: formatDate(from), to: formatDate(to) };
}

/** Sunday-based week start for heatmap columns. */
export function startOfWeek(d: Date): Date {
  const x = new Date(d);
  x.setHours(0, 0, 0, 0);
  x.setDate(x.getDate() - x.getDay());
  return x;
}
