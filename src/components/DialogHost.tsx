import {
  FormEvent,
  useEffect,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";

type ConfirmDialog = {
  kind: "confirm";
  message: string;
  resolve: (value: boolean) => void;
};

type PromptDialog = {
  kind: "prompt";
  title: string;
  initial: string;
  resolve: (value: string | null) => void;
};

type DialogState = ConfirmDialog | PromptDialog | null;

let setDialogRef: Dispatch<SetStateAction<DialogState>> | null = null;

function openDialog(state: Exclude<DialogState, null>) {
  if (!setDialogRef) {
    console.error("DialogHost is not mounted");
    if (state.kind === "confirm") {
      state.resolve(false);
    } else {
      state.resolve(null);
    }
    return;
  }
  setDialogRef(state);
}

export function confirmDialog(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    openDialog({ kind: "confirm", message, resolve });
  });
}

export function promptDialog(
  title: string,
  initial = "",
): Promise<string | null> {
  return new Promise((resolve) => {
    openDialog({ kind: "prompt", title, initial, resolve });
  });
}

export function DialogHost() {
  const [dialog, setDialog] = useState<DialogState>(null);
  const [draft, setDraft] = useState("");

  useEffect(() => {
    setDialogRef = setDialog;
    return () => {
      setDialogRef = null;
    };
  }, []);

  useEffect(() => {
    if (dialog?.kind === "prompt") {
      setDraft(dialog.initial);
    }
  }, [dialog]);

  function closeConfirm(result: boolean) {
    if (dialog?.kind !== "confirm") return;
    dialog.resolve(result);
    setDialog(null);
  }

  function closePrompt(result: string | null) {
    if (dialog?.kind !== "prompt") return;
    dialog.resolve(result);
    setDialog(null);
  }

  function onPromptSubmit(e: FormEvent) {
    e.preventDefault();
    if (dialog?.kind !== "prompt") return;
    closePrompt(draft);
  }

  if (!dialog) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="presentation"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) {
          if (dialog.kind === "confirm") closeConfirm(false);
          else closePrompt(null);
        }
      }}
    >
      <div
        className="w-full max-w-md rounded-lg border border-zinc-700 bg-zinc-900 p-4 shadow-xl"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
      >
        {dialog.kind === "confirm" ? (
          <>
            <p id="dialog-title" className="text-sm text-zinc-100">
              {dialog.message}
            </p>
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                className="rounded border border-zinc-600 px-3 py-1 text-sm"
                onClick={() => closeConfirm(false)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="rounded bg-red-600 px-3 py-1 text-sm text-white"
                onClick={() => closeConfirm(true)}
              >
                Delete
              </button>
            </div>
          </>
        ) : (
          <form onSubmit={onPromptSubmit}>
            <label
              id="dialog-title"
              className="mb-2 block text-sm text-zinc-300"
            >
              {dialog.title}
            </label>
            <input
              autoFocus
              className="w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                className="rounded border border-zinc-600 px-3 py-1 text-sm"
                onClick={() => closePrompt(null)}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="rounded bg-zinc-100 px-3 py-1 text-sm text-zinc-900"
              >
                Save
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
