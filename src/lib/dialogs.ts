import { confirmDialog, promptDialog } from "../components/DialogHost";

export async function confirmDelete(label: string): Promise<boolean> {
  return confirmDialog(
    `Delete ${label}? This cannot be undone from the UI.`,
  );
}

export async function promptRename(
  current: string,
  kind: string,
): Promise<string | null> {
  const value = await promptDialog(`Rename ${kind}`, current);
  if (value === null) return null;
  const trimmed = value.trim();
  if (!trimmed || trimmed === current) return null;
  return trimmed;
}

export async function promptText(
  title: string,
  initial = "",
): Promise<string | null> {
  return promptDialog(title, initial);
}
