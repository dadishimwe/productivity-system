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

export function startOfDay(d: Date): Date {
  const x = new Date(d);
  x.setHours(0, 0, 0, 0);
  return x;
}

export function weekWindow(anchor: Date): {
  startMs: number;
  endMs: number;
  days: Date[];
} {
  const start = startOfWeek(anchor);
  const days = Array.from({ length: 7 }, (_, i) => addDays(start, i));
  const end = addDays(start, 7);
  return { startMs: start.getTime(), endMs: end.getTime(), days };
}

export function formatWeekday(d: Date): string {
  return d.toLocaleDateString(undefined, { weekday: "short" });
}

export function formatDayLabel(d: Date): string {
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
  });
}

export function monthWindow(anchor: Date): {
  startMs: number;
  endMs: number;
  weeks: Date[][];
  label: string;
} {
  const first = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const start = startOfWeek(first);
  const weeks: Date[][] = [];
  let cursor = start;
  for (let w = 0; w < 6; w++) {
    weeks.push(Array.from({ length: 7 }, (_, i) => addDays(cursor, i)));
    cursor = addDays(cursor, 7);
  }
  const label = first.toLocaleDateString(undefined, {
    month: "long",
    year: "numeric",
  });
  return {
    startMs: start.getTime(),
    endMs: cursor.getTime(),
    weeks,
    label,
  };
}

export function isSameMonth(a: Date, monthAnchor: Date): boolean {
  return (
    a.getFullYear() === monthAnchor.getFullYear() &&
    a.getMonth() === monthAnchor.getMonth()
  );
}
