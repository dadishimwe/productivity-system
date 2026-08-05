import { invoke } from "@tauri-apps/api/core";
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import {
  addDays,
  formatDate,
  formatDayLabel,
  formatTime,
  formatWeekday,
  isSameMonth,
  monthWindow,
  startOfDay,
  weekWindow,
} from "../lib/dates";
import {
  chooseOccurrenceScope,
  confirmDelete,
  promptRename,
} from "../lib/dialogs";
import {
  RECURRENCE_LABELS,
  rruleFromPreset,
  type RecurrencePreset,
} from "../lib/recurrence";
import { IconButton } from "./IconButton";
import { GoogleSyncPanel } from "./GoogleSyncPanel";
import { Select } from "./Select";

type Calendar = { id: string; name: string; color: string | null };

export type Occurrence = {
  event_id: string;
  calendar_id: string;
  title: string;
  description: string | null;
  original_start_ms: number;
  start_ms: number;
  end_ms: number;
  all_day: boolean;
  recurring: boolean;
};

type ViewMode = "week" | "month";

function occKey(o: Occurrence): string {
  return `${o.event_id}:${o.original_start_ms}`;
}

function eventDayKey(ms: number): string {
  return startOfDay(new Date(ms)).toDateString();
}

function parseTimeOnDay(day: Date, hhmm: string): number {
  const [h, m] = hhmm.split(":").map((x) => parseInt(x, 10));
  const d = new Date(day);
  d.setHours(h || 0, m || 0, 0, 0);
  return d.getTime();
}

function msToTimeInput(ms: number): string {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export function CalendarView({ onError }: { onError: (msg: string) => void }) {
  const [viewMode, setViewMode] = useState<ViewMode>("week");
  const [anchor, setAnchor] = useState(() => new Date());
  const [calendars, setCalendars] = useState<Calendar[]>([]);
  const [calendarId, setCalendarId] = useState<string | null>(null);
  const [occurrences, setOccurrences] = useState<Occurrence[]>([]);
  const [editor, setEditor] = useState<Occurrence | "new" | null>(null);
  const [formTitle, setFormTitle] = useState("");
  const [formDay, setFormDay] = useState(() => new Date());
  const [formStart, setFormStart] = useState("09:00");
  const [formEnd, setFormEnd] = useState("10:00");
  const [formAllDay, setFormAllDay] = useState(false);
  const [formRecurrence, setFormRecurrence] = useState<RecurrencePreset>("none");

  const week = useMemo(() => weekWindow(anchor), [anchor]);
  const month = useMemo(() => monthWindow(anchor), [anchor]);
  const range =
    viewMode === "week"
      ? { startMs: week.startMs, endMs: week.endMs, days: week.days }
      : {
          startMs: month.startMs,
          endMs: month.endMs,
          days: month.weeks.flat(),
        };

  const currentCalendar = calendars.find((c) => c.id === calendarId);

  const loadCalendars = useCallback(async () => {
    await invoke("ensure_default_calendar_cmd");
    const all = await invoke<Calendar[]>("list_calendars_cmd");
    setCalendars(all);
    setCalendarId((prev) =>
      prev && all.some((c) => c.id === prev) ? prev : (all[0]?.id ?? null),
    );
  }, []);

  const loadOccurrences = useCallback(async () => {
    if (!calendarId) {
      setOccurrences([]);
      return;
    }
    const rows = await invoke<Occurrence[]>("list_occurrences_cmd", {
      calendarId,
      rangeStartMs: range.startMs,
      rangeEndMs: range.endMs,
    });
    setOccurrences(rows);
  }, [calendarId, range.endMs, range.startMs]);

  useEffect(() => {
    loadCalendars().catch((e) => onError(String(e)));
  }, [loadCalendars, onError]);

  useEffect(() => {
    loadOccurrences().catch((e) => onError(String(e)));
  }, [loadOccurrences, onError]);

  const byDay = useMemo(() => {
    const map = new Map<string, Occurrence[]>();
    for (const o of occurrences) {
      const key = eventDayKey(o.start_ms);
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(o);
    }
    for (const list of map.values()) {
      list.sort((a, b) => a.start_ms - b.start_ms);
    }
    return map;
  }, [occurrences]);

  function openNewEvent(day: Date) {
    setEditor("new");
    setFormTitle("");
    setFormDay(day);
    setFormStart("09:00");
    setFormEnd("10:00");
    setFormAllDay(false);
    setFormRecurrence("none");
  }

  function openEdit(o: Occurrence) {
    setEditor(o);
    setFormTitle(o.title);
    setFormDay(new Date(o.start_ms));
    setFormAllDay(o.all_day);
    setFormStart(msToTimeInput(o.start_ms));
    setFormEnd(msToTimeInput(o.end_ms));
    setFormRecurrence("none");
  }

  async function saveEditor(e: FormEvent) {
    e.preventDefault();
    if (!calendarId || !formTitle.trim()) return;
    let startMs: number;
    let endMs: number;
    if (formAllDay) {
      startMs = startOfDay(formDay).getTime();
      endMs = startOfDay(addDays(formDay, 1)).getTime();
    } else {
      startMs = parseTimeOnDay(formDay, formStart);
      endMs = parseTimeOnDay(formDay, formEnd);
      if (endMs <= startMs) endMs = startMs + 60 * 60 * 1000;
    }
    const rrule = rruleFromPreset(formRecurrence, formDay);

    if (editor === "new") {
      await invoke("create_event_cmd", {
        calendarId,
        title: formTitle.trim(),
        description: null,
        startMs,
        endMs,
        allDay: formAllDay,
        rrule,
      });
    } else if (editor) {
      await invoke("update_event_cmd", {
        id: editor.event_id,
        title: formTitle.trim(),
        description: editor.description,
        startMs,
        endMs,
        allDay: formAllDay,
        rrule: editor.recurring ? null : rrule,
      });
    }
    setEditor(null);
    await loadOccurrences();
  }

  async function removeOccurrence(o: Occurrence) {
    let scope: "this" | "this_and_following" | "all" = "this";
    if (o.recurring) {
      const chosen = await chooseOccurrenceScope("delete");
      if (!chosen) return;
      scope = chosen;
    } else if (!(await confirmDelete(`event “${o.title}”`))) {
      return;
    }
    await invoke("delete_occurrence_cmd", {
      eventId: o.event_id,
      originalStartMs: o.original_start_ms,
      scope,
    });
    await loadOccurrences();
  }

  async function dropOccurrence(
    o: Occurrence,
    targetDay: Date,
  ) {
    const duration = o.end_ms - o.start_ms;
    let newStart: number;
    let newEnd: number;
    if (o.all_day) {
      newStart = startOfDay(targetDay).getTime();
      newEnd = newStart + duration;
    } else {
      const src = new Date(o.start_ms);
      const dst = new Date(targetDay);
      dst.setHours(src.getHours(), src.getMinutes(), 0, 0);
      newStart = dst.getTime();
      newEnd = newStart + duration;
    }
    let scope: "this" | "this_and_following" | "all" = "this";
    if (o.recurring) {
      const chosen = await chooseOccurrenceScope("move");
      if (!chosen) return;
      scope = chosen;
    }
    await invoke("move_occurrence_cmd", {
      eventId: o.event_id,
      originalStartMs: o.original_start_ms,
      newStartMs: newStart,
      newEndMs: newEnd,
      scope,
    });
    await loadOccurrences();
  }

  function navPrev() {
    setAnchor((d) =>
      addDays(d, viewMode === "week" ? -7 : -30),
    );
  }

  function navNext() {
    setAnchor((d) => addDays(d, viewMode === "week" ? 7 : 30));
  }

  function renderDayCell(day: Date, compact: boolean) {
    const key = day.toDateString();
    const list = byDay.get(key) ?? [];
    const muted =
      viewMode === "month" && !isSameMonth(day, anchor) ? "opacity-40" : "";
    return (
      <div
        key={key}
        className={`flex min-h-[120px] flex-col rounded-lg border border-zinc-800 bg-zinc-900/40 p-2 ${muted}`}
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => {
          e.preventDefault();
          const raw = e.dataTransfer.getData("application/json");
          if (!raw) return;
          const o = JSON.parse(raw) as Occurrence;
          dropOccurrence(o, day).catch((err) => onError(String(err)));
        }}
      >
        <button
          type="button"
          className="mb-2 text-left text-xs text-zinc-400 hover:text-zinc-200"
          onClick={() => openNewEvent(day)}
        >
          {formatWeekday(day)}
          <div className="text-sm text-zinc-200">{formatDayLabel(day)}</div>
        </button>
        <ul className={`space-y-1 ${compact ? "text-[10px]" : "text-xs"}`}>
          {list.map((o) => (
            <li
              key={occKey(o)}
              draggable
              onDragStart={(e) =>
                e.dataTransfer.setData("application/json", JSON.stringify(o))
              }
              className="cursor-grab rounded border border-zinc-700/80 bg-zinc-800/90 px-1.5 py-1 active:cursor-grabbing"
            >
              <button
                type="button"
                className="w-full text-left"
                onClick={() => openEdit(o)}
              >
                <div className="truncate font-medium">{o.title}</div>
                {!compact && (
                  <div className="text-zinc-500">
                    {o.all_day
                      ? "All day"
                      : `${formatTime(o.start_ms)} – ${formatTime(o.end_ms)}`}
                    {o.recurring ? " ↻" : ""}
                  </div>
                )}
              </button>
              <div className="mt-0.5 flex justify-end">
                <IconButton
                  label="Delete"
                  onClick={() =>
                    removeOccurrence(o).catch((err) => onError(String(err)))
                  }
                />
              </div>
            </li>
          ))}
        </ul>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <GoogleSyncPanel onError={onError} />
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm text-zinc-400">Calendar</span>
        <div className="min-w-[10rem]">
          <Select
            aria-label="Calendar"
            value={calendarId ?? ""}
            options={calendars.map((c) => ({ value: c.id, label: c.name }))}
            placeholder="No calendars"
            onChange={(id) => setCalendarId(id || null)}
          />
        </div>
        {calendarId && (
          <>
            <IconButton
              label="Rename"
              onClick={() =>
                (async () => {
                  const name = await promptRename(currentCalendar!.name, "calendar");
                  if (!name) return;
                  await invoke("rename_calendar_cmd", { id: calendarId, name });
                  await loadCalendars();
                })().catch((err) => onError(String(err)))
              }
            />
            <IconButton
              label="Delete"
              onClick={() =>
                (async () => {
                  if (!(await confirmDelete(`calendar “${currentCalendar!.name}”`)))
                    return;
                  await invoke("delete_calendar_cmd", { id: calendarId });
                  await loadCalendars();
                })().catch((err) => onError(String(err)))
              }
            />
          </>
        )}
        <div className="ml-auto flex rounded-lg border border-zinc-800 p-0.5 text-sm">
          <button
            type="button"
            className={`rounded px-2 py-0.5 ${viewMode === "week" ? "bg-zinc-100 text-zinc-900" : "text-zinc-400"}`}
            onClick={() => setViewMode("week")}
          >
            Week
          </button>
          <button
            type="button"
            className={`rounded px-2 py-0.5 ${viewMode === "month" ? "bg-zinc-100 text-zinc-900" : "text-zinc-400"}`}
            onClick={() => setViewMode("month")}
          >
            Month
          </button>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="rounded border border-zinc-600 px-2 py-1 text-sm"
          onClick={navPrev}
        >
          ← Prev
        </button>
        <span className="text-sm text-zinc-300">
          {viewMode === "week"
            ? `${formatDayLabel(week.days[0])} – ${formatDayLabel(week.days[6])}`
            : month.label}
        </span>
        <button
          type="button"
          className="rounded border border-zinc-600 px-2 py-1 text-sm"
          onClick={navNext}
        >
          Next →
        </button>
        <button
          type="button"
          className="rounded border border-zinc-600 px-2 py-1 text-sm text-zinc-400"
          onClick={() => setAnchor(new Date())}
        >
          Today
        </button>
        {calendarId && (
          <button
            type="button"
            className="rounded bg-zinc-100 px-3 py-1 text-sm text-zinc-900"
            onClick={() => openNewEvent(new Date())}
          >
            New event
          </button>
        )}
      </div>

      {viewMode === "week" ? (
        <div className="grid grid-cols-7 gap-2">
          {week.days.map((day) => renderDayCell(day, false))}
        </div>
      ) : (
        <div className="space-y-2">
          <div className="grid grid-cols-7 gap-1 text-center text-xs text-zinc-500">
            {["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map((d) => (
              <span key={d}>{d}</span>
            ))}
          </div>
          {month.weeks.map((row) => (
            <div key={row[0].toISOString()} className="grid grid-cols-7 gap-1">
              {row.map((day) => renderDayCell(day, true))}
            </div>
          ))}
        </div>
      )}

      {editor && (
        <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 p-4">
          <form
            onSubmit={(e) => saveEditor(e).catch((err) => onError(String(err)))}
            className="w-full max-w-md space-y-3 rounded-lg border border-zinc-700 bg-zinc-900 p-4"
          >
            <h2 className="text-sm font-medium text-zinc-100">
              {editor === "new" ? "New event" : "Edit event"}
            </h2>
            {editor !== "new" && editor.recurring && (
              <p className="text-xs text-amber-200/80">
                Edits update the series anchor. Use drag + scope to move one
                instance.
              </p>
            )}
            <input
              className="w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
              placeholder="Title"
              value={formTitle}
              onChange={(e) => setFormTitle(e.target.value)}
              autoFocus
            />
            <input
              type="date"
              className="w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
              value={formatDate(formDay)}
              onChange={(e) => {
                const [y, m, d] = e.target.value.split("-").map(Number);
                if (y && m && d) setFormDay(new Date(y, m - 1, d));
              }}
            />
            <label className="flex items-center gap-2 text-sm text-zinc-400">
              <input
                type="checkbox"
                checked={formAllDay}
                onChange={(e) => setFormAllDay(e.target.checked)}
              />
              All day
            </label>
            {!formAllDay && (
              <div className="flex gap-2">
                <input
                  type="time"
                  className="flex-1 rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-sm"
                  value={formStart}
                  onChange={(e) => setFormStart(e.target.value)}
                />
                <input
                  type="time"
                  className="flex-1 rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-sm"
                  value={formEnd}
                  onChange={(e) => setFormEnd(e.target.value)}
                />
              </div>
            )}
            {editor === "new" && (
              <select
                className="w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-sm"
                value={formRecurrence}
                onChange={(e) =>
                  setFormRecurrence(e.target.value as RecurrencePreset)
                }
              >
                {(Object.keys(RECURRENCE_LABELS) as RecurrencePreset[]).map(
                  (k) => (
                    <option key={k} value={k}>
                      {RECURRENCE_LABELS[k]}
                    </option>
                  ),
                )}
              </select>
            )}
            <div className="flex justify-end gap-2 pt-2">
              <button
                type="button"
                className="rounded border border-zinc-600 px-3 py-1 text-sm"
                onClick={() => setEditor(null)}
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
        </div>
      )}
    </div>
  );
}
