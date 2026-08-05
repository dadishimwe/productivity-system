import { useState } from "react";
import { BoardView } from "./components/BoardView";
import { CalendarView } from "./components/CalendarView";
import { DialogHost } from "./components/DialogHost";
import { HabitsView } from "./components/HabitsView";
import { ShoppingView } from "./components/ShoppingView";
import { ToastHost } from "./components/ToastHost";

type Tab = "board" | "habits" | "shopping" | "calendar";

export default function App() {
  const [tab, setTab] = useState<Tab>("board");
  const [error, setError] = useState<string | null>(null);

  const tabClass = (t: Tab) =>
    `rounded px-3 py-1 text-sm ${tab === t ? "bg-zinc-100 text-zinc-900" : "text-zinc-400"}`;

  return (
    <main className="mx-auto max-w-6xl p-6">
      <header className="mb-6 flex items-center justify-between gap-4">
        <h1 className="text-2xl font-semibold tracking-tight">Productivity</h1>
        <nav className="flex gap-1 rounded-lg border border-zinc-800 p-1">
          <button type="button" className={tabClass("board")} onClick={() => setTab("board")}>
            Board
          </button>
          <button type="button" className={tabClass("habits")} onClick={() => setTab("habits")}>
            Habits
          </button>
          <button
            type="button"
            className={tabClass("shopping")}
            onClick={() => setTab("shopping")}
          >
            Shopping
          </button>
          <button
            type="button"
            className={tabClass("calendar")}
            onClick={() => setTab("calendar")}
          >
            Calendar
          </button>
        </nav>
      </header>

      <DialogHost />
      <ToastHost />

      {error && (
        <p className="mb-4 rounded border border-red-900/50 bg-red-950/40 px-3 py-2 text-sm text-red-200">
          {error}
        </p>
      )}

      {tab === "board" && <BoardView onError={setError} />}
      {tab === "habits" && <HabitsView onError={setError} />}
      {tab === "shopping" && <ShoppingView onError={setError} />}
      {tab === "calendar" && <CalendarView onError={setError} />}
    </main>
  );
}
