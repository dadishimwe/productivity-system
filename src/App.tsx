import { useState } from "react";
import { BoardView } from "./components/BoardView";
import { HabitsView } from "./components/HabitsView";

type Tab = "board" | "habits";

export default function App() {
  const [tab, setTab] = useState<Tab>("board");
  const [error, setError] = useState<string | null>(null);

  return (
    <main className="mx-auto max-w-6xl p-6">
      <header className="mb-6 flex items-center justify-between gap-4">
        <h1 className="text-2xl font-semibold tracking-tight">Productivity</h1>
        <nav className="flex gap-1 rounded-lg border border-zinc-800 p-1">
          <button
            type="button"
            className={`rounded px-3 py-1 text-sm ${tab === "board" ? "bg-zinc-100 text-zinc-900" : "text-zinc-400"}`}
            onClick={() => setTab("board")}
          >
            Board
          </button>
          <button
            type="button"
            className={`rounded px-3 py-1 text-sm ${tab === "habits" ? "bg-zinc-100 text-zinc-900" : "text-zinc-400"}`}
            onClick={() => setTab("habits")}
          >
            Habits
          </button>
        </nav>
      </header>

      {error && (
        <p className="mb-4 rounded border border-red-900/50 bg-red-950/40 px-3 py-2 text-sm text-red-200">
          {error}
        </p>
      )}

      {tab === "board" ? (
        <BoardView onError={setError} />
      ) : (
        <HabitsView onError={setError} />
      )}
    </main>
  );
}
