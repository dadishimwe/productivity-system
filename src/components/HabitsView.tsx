import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  addDays,
  formatDate,
  lastMonthsRange,
  startOfWeek,
} from "../lib/dates";

type Habit = {
  id: string;
  name: string;
  color: string | null;
  target_frequency: string | null;
};

type HabitLog = {
  id: string;
  habit_id: string;
  date: string;
  value: number;
};

function parseHex(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function cellColor(base: string, value: number, max: number): string {
  if (value <= 0) return "rgb(39 39 42)";
  const [r, g, b] = parseHex(base.startsWith("#") ? base : "#6366f1");
  const t = Math.min(1, value / Math.max(1, max));
  const mix = (c: number) => Math.round(39 + (c - 39) * t);
  return `rgb(${mix(r)} ${mix(g)} ${mix(b)})`;
}

function Heatmap({
  habit,
  logs,
  onToggle,
}: {
  habit: Habit;
  logs: Map<string, number>;
  onToggle: (date: string, logged: boolean) => void;
}) {
  const weeks = 53;
  const end = new Date();
  const gridStart = startOfWeek(addDays(end, -(weeks * 7 - 1)));

  const cells = useMemo(() => {
    const out: { date: string; week: number; day: number }[] = [];
    for (let w = 0; w < weeks; w++) {
      for (let d = 0; d < 7; d++) {
        const date = addDays(gridStart, w * 7 + d);
        if (date > end) continue;
        out.push({ date: formatDate(date), week: w, day: d });
      }
    }
    return out;
  }, [gridStart, end]);

  const maxVal = useMemo(
    () => Math.max(1, ...Array.from(logs.values())),
    [logs],
  );
  const base = habit.color ?? "#6366f1";

  return (
    <div className="overflow-x-auto">
      <div
        className="inline-grid gap-[3px]"
        style={{
          gridTemplateColumns: `repeat(${weeks}, 12px)`,
          gridTemplateRows: "repeat(7, 12px)",
        }}
      >
        {cells.map(({ date, week, day }) => {
          const value = logs.get(date) ?? 0;
          const logged = value > 0;
          return (
            <button
              key={date}
              type="button"
              title={`${date}${logged ? ` · ${value}` : ""}`}
              aria-label={`${date}${logged ? ", logged" : ", empty"}`}
              onClick={() => onToggle(date, logged)}
              className="h-3 w-3 rounded-sm border border-zinc-800/50 p-0"
              style={{
                gridColumn: week + 1,
                gridRow: day + 1,
                backgroundColor: cellColor(base, value, maxVal),
              }}
            />
          );
        })}
      </div>
    </div>
  );
}

export function HabitsView({ onError }: { onError: (msg: string) => void }) {
  const [habits, setHabits] = useState<Habit[]>([]);
  const [logsByHabit, setLogsByHabit] = useState<
    Record<string, Map<string, number>>
  >({});
  const [newName, setNewName] = useState("");
  const range = lastMonthsRange(12);

  const load = useCallback(async () => {
    const list = await invoke<Habit[]>("list_habits_cmd");
    setHabits(list);
    const next: Record<string, Map<string, number>> = {};
    for (const h of list) {
      const logs = await invoke<HabitLog[]>("list_habit_logs_cmd", {
        habitId: h.id,
        fromDate: range.from,
        toDate: range.to,
      });
      next[h.id] = new Map(logs.map((l) => [l.date, l.value]));
    }
    setLogsByHabit(next);
  }, [range.from, range.to]);

  useEffect(() => {
    load().catch((e) => onError(String(e)));
  }, [load, onError]);

  async function createHabit() {
    if (!newName.trim()) return;
    await invoke("create_habit_cmd", {
      name: newName.trim(),
      color: "#22c55e",
      targetFrequency: null,
    });
    setNewName("");
    await load();
  }

  async function toggleDay(habitId: string, date: string, logged: boolean) {
    if (logged) {
      await invoke("unlog_habit_cmd", { habitId, date });
    } else {
      await invoke("log_habit_cmd", { habitId, date, value: 1 });
    }
    await load();
  }

  return (
    <div className="space-y-8">
      <div className="flex gap-2">
        <input
          className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          placeholder="New habit"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <button
          type="button"
          className="rounded bg-zinc-100 px-3 py-1 text-sm text-zinc-900"
          onClick={() => createHabit().catch((e) => onError(String(e)))}
        >
          Add habit
        </button>
      </div>

      {habits.length === 0 && (
        <p className="text-sm text-zinc-500">No habits yet.</p>
      )}

      {habits.map((h) => (
        <section key={h.id} className="space-y-2">
          <h3 className="text-sm font-medium" style={{ color: h.color ?? undefined }}>
            {h.name}
          </h3>
          <Heatmap
            habit={h}
            logs={logsByHabit[h.id] ?? new Map()}
            onToggle={(date, logged) =>
              toggleDay(h.id, date, logged).catch((e) => onError(String(e)))
            }
          />
        </section>
      ))}
    </div>
  );
}
