import { choiceDialog, confirmDialog, promptDialog } from "../components/DialogHost";

export async function confirmDelete(label: string): Promise<boolean> {
  return confirmDialog(
    `Delete ${label}? This cannot be undone from the UI.`,
  );
}

export { confirmDialog };

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

export type OccurrenceScope = "this" | "this_and_following" | "all";

export async function chooseOccurrenceScope(
  action: "move" | "delete",
): Promise<OccurrenceScope | null> {
  const id = await choiceDialog(
    action === "move"
      ? "Apply move to which events?"
      : "Delete which events?",
    [
      { id: "this", label: "This event only" },
      { id: "this_and_following", label: "This and following" },
      { id: "all", label: "All events in the series" },
    ],
  );
  if (id === "this" || id === "this_and_following" || id === "all") return id;
  return null;
}

export async function promptText(
  title: string,
  initial = "",
): Promise<string | null> {
  return promptDialog(title, initial);
}

export { promptDialog };
