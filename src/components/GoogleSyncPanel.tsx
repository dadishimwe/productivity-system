import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { confirmDialog } from "../lib/dialogs";

type GoogleAccount = { id: string; email: string };

export function GoogleSyncPanel({ onError }: { onError: (msg: string) => void }) {
  const [accounts, setAccounts] = useState<GoogleAccount[]>([]);
  const [connecting, setConnecting] = useState(false);

  const load = useCallback(async () => {
    const rows = await invoke<GoogleAccount[]>("list_google_accounts_cmd");
    setAccounts(rows);
  }, []);

  useEffect(() => {
    load().catch((e) => onError(String(e)));
  }, [load, onError]);

  async function connect() {
    setConnecting(true);
    try {
      await invoke("connect_google_oauth_cmd");
      await load();
    } catch (e) {
      onError(String(e));
    } finally {
      setConnecting(false);
    }
  }

  async function disconnect(id: string, email: string) {
    if (!(await confirmDialog(`Disconnect ${email}?`))) return;
    await invoke("disconnect_google_account_cmd", { id });
    await load();
  }

  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-medium text-zinc-200">Google Calendar</h3>
          <p className="text-xs text-zinc-500">
            Connect an account (refresh token stored in the OS keychain). Set{" "}
            <code className="text-zinc-400">GOOGLE_OAUTH_CLIENT_ID</code> and{" "}
            <code className="text-zinc-400">GOOGLE_OAUTH_CLIENT_SECRET</code>{" "}
            before connecting. Calendar sync pull/push is the next step.
          </p>
        </div>
        <button
          type="button"
          disabled={connecting}
          className="rounded bg-zinc-100 px-3 py-1.5 text-sm text-zinc-900 disabled:opacity-50"
          onClick={() => connect().catch((e) => onError(String(e)))}
        >
          {connecting ? "Waiting for Google…" : "Connect Google"}
        </button>
      </div>
      {accounts.length > 0 && (
        <ul className="mt-3 space-y-1 border-t border-zinc-800 pt-2 text-sm">
          {accounts.map((a) => (
            <li
              key={a.id}
              className="flex items-center justify-between gap-2 text-zinc-300"
            >
              <span>{a.email}</span>
              <span className="rounded bg-emerald-950/50 px-1.5 text-xs text-emerald-400">
                Connected
              </span>
              <button
                type="button"
                className="text-xs text-zinc-500 hover:text-red-400"
                onClick={() =>
                  disconnect(a.id, a.email).catch((e) => onError(String(e)))
                }
              >
                Disconnect
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
