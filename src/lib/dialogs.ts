export function confirmDelete(label: string): boolean {
  return window.confirm(`Delete ${label}? This cannot be undone from the UI.`);
}

export function promptRename(current: string, kind: string): string | null {
  const next = window.prompt(`Rename ${kind}`, current);
  if (next === null) return null;
  const trimmed = next.trim();
  if (!trimmed || trimmed === current) return null;
  return trimmed;
}
