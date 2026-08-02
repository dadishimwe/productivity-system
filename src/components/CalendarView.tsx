import { invoke } from "@tauri-apps/api/core";
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import {
  addDays,
  formatDayLabel,
  formatTime,
  formatWeekday,
  startOfDay,
  weekWindow,
} from "../lib/dates";
import { confirmDelete, promptRename } from "../lib/dialogs";
import { IconButton } from "./IconButton";

type Calendar = { id: string; name: string; color: string | null };
type EventRow = {
  id: string;
  calendar_id: string;
  title: string;
  description: string | null;
  start_ms: number;
  end_ms: number;
  all_day: boolean;
};

function eventDayKey(ms: number): string {
  return startOfDay(new Date(ms)).toDateString();
}

export function CalendarView({ onError }: { onError: (msg: string) => void }) {
  const [weekAnchor, setWeekAnchor] = useState(() => new Date());
  const [calendars, setCalendars] = useState<Calendar[]>([]);
  const [calendarId, setCalendarId] = useState<string | null>(null);
  const [events, setEvents] = useState<EventRow[]>([]);
  const [newCalendarName, setNewCalendarName] = useState("");
  const [title, setTitle] = useState("");
  const [dayIndex, setDayIndex] = useState(0);
  const [startTime, setStartTime] = useState("09:00");
  const [endTime, setEndTime] = useState("10:00");
  const [allDay, setAllDay] = useState(false);

  const window = useMemo(() => weekWindow(weekAnchor), [weekAnchor]);
  const currentCalendar = calendars.find((c) => c.id === calendarId);

  const loadCalendars = useCallback(async () => {
    await invoke("ensure_default_calendar_cmd");
    const all = await invoke<Calendar[]>("list_calendars_cmd");
    setCalendars(all);
    setCalendarId((prev) =>
      prev && all.some((c) => c.id === prev) ? prev : (all[0]?.id ?? null),
    );
  }, []);

  const loadEvents = useCallback(async () => {
    if (!calendarId) {
      setEvents([]);
      return;
    }
    const rows = await invoke<EventRow[]>("list_events_cmd", {
      calendarId,
      rangeStartMs: window.startMs,
      rangeEndMs: window.endMs,
    });
    setEvents(rows);
  }, [calendarId, window.endMs, window.startMs]);

  useEffect(() => {
    loadCalendars().catch((e) => onError(String(e)));
  }, [loadCalendars, onError]);

  useEffect(() => {
    loadEvents().catch((e) => onError(String(e)));
  }, [loadEvents, onError]);

  const eventsByDay = useMemo(() => {
    const map = new Map<string, EventRow[]>();
    for (const day of window.days) {
      map.set(day.toDateString(), []);
    }
    for (const ev of events) {
      const key = eventDayKey(ev.start_ms);
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(ev);
    }
    for (const list of map.values()) {
      list.sort((a, b) => a.start_ms - b.start_ms);
    }
    return map;
  }, [events, window.days]);

  async function createCalendar() {
    if (!newCalendarName.trim()) return;
    await invoke("create_calendar_cmd", {
      name: newCalendarName.trim(),
      color: null,
    });
    setNewCalendarName("");
    await loadCalendars();
  }

  async function renameCalendar() {
    if (!calendarId || !currentCalendar) return;
    const name = await promptRename(currentCalendar.name, "calendar");
    if (!name) return;
    await invoke("rename_calendar_cmd", { id: calendarId, name });
    await loadCalendars();
  }

  async function deleteCalendar() {
    if (!calendarId || !currentCalendar) return;
    if (!(await confirmDelete(`calendar “${currentCalendar.name}”`))) return;
    await invoke("delete_calendar_cmd", { id: calendarId });
    await loadCalendars();
  }

  function parseTimeOnDay(day: Date, hhmm: string): number {
    const [h, m] = hhmm.split(":").map((x) => parseInt(x, 10));
    const d = new Date(day);
    d.setHours(h || 0, m || 0, 0, 0);
    return d.getTime();
  }

  async function addEvent(e: FormEvent) {
    e.preventDefault();
    if (!calendarId || !title.trim()) return;
    const day = window.days[dayIndex] ?? window.days[0];
    let startMs: number;
    let endMs: number;
    if (allDay) {
      startMs = startOfDay(day).getTime();
      endMs = startOfDay(addDays(day, 1)).getTime();
    } else {
      startMs = parseTimeOnDay(day, startTime);
      endMs = parseTimeOnDay(day, endTime);
      if (endMs <= startMs) endMs = startMs + 60 * 60 * 1000;
    }
    await invoke("create_event_cmd", {
      calendarId,
      title: title.trim(),
      description: null,
      startMs,
      endMs,
      allDay,
    });
    setTitle("");
    await loadEvents();
  }

  async function removeEvent(ev: EventRow) {
    if (!(await confirmDelete(`event “${ev.title}”`))) return;
    await invoke("delete_event_cmd", { id: ev.id });
    await loadEvents();
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm text-zinc-400">Calendar</span>
        <select
          className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          value={calendarId ?? ""}
          onChange={(e) => setCalendarId(e.target.value || null)}
        >
          {calendars.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
        {calendarId && (
          <>
            <IconButton
              label="Rename"
              onClick={() =>
                renameCalendar().catch((err) => onError(String(err)))
              }
            />
            <IconButton
              label="Delete"
              onClick={() =>
                deleteCalendar().catch((err) => onError(String(err)))
              }
            />
          </>
        )}
        <input
          className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          placeholder="New calendar"
          value={newCalendarName}
          onChange={(e) => setNewCalendarName(e.target.value)}
        />
        <button
          type="button"
          className="rounded bg-zinc-100 px-3 py-1 text-sm text-zinc-900"
          onClick={() => createCalendar().catch((err) => onError(String(err)))}
        >
          Add calendar
        </button>
      </div>

      <div className="flex items-center justify-between gap-2">
        <button
          type="button"
          className="rounded border border-zinc-600 px-2 py-1 text-sm"
          onClick={() => setWeekAnchor(addDays(weekAnchor, -7))}
        >
          ← Prev
        </button>
        <span className="text-sm text-zinc-300">
          {formatDayLabel(window.days[0])} – {formatDayLabel(window.days[6])}
        </span>
        <button
          type="button"
          className="rounded border border-zinc-600 px-2 py-1 text-sm"
          onClick={() => setWeekAnchor(addDays(weekAnchor, 7))}
        >
          Next →
        </button>
        <button
          type="button"
          className="rounded border border-zinc-600 px-2 py-1 text-sm text-zinc-400"
          onClick={() => setWeekAnchor(new Date())}
        >
          Today
        </button>
      </div>

      <div className="grid grid-cols-7 gap-2">
        {window.days.map((day) => {
          const key = day.toDateString();
          const dayEvents = eventsByDay.get(key) ?? [];
          return (
            <div
              key={key}
              className="min-h-[140px] rounded-lg border border-zinc-800 bg-zinc-900/40 p-2"
            >
              <div className="mb-2 text-xs text-zinc-400">
                {formatWeekday(day)}
                <div className="text-sm text-zinc-200">{formatDayLabel(day)}</div>
              </div>
              <ul className="space-y-1">
                {dayEvents.map((ev) => (
                  <li
                    key={ev.id}
                    className="group flex items-start gap-1 rounded bg-zinc-800/80 px-1.5 py-1 text-xs"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="truncate font-medium">{ev.title}</div>
                      <div className="text-zinc-500">
                        {ev.all_day
                          ? "All day"
                          : `${formatTime(ev.start_ms)} – ${formatTime(ev.end_ms)}`}
                      </div>
                    </div>
                    <IconButton
                      label="Delete"
                      onClick={() =>
                        removeEvent(ev).catch((err) => onError(String(err)))
                      }
                    />
                  </li>
                ))}
              </ul>
            </div>
          );
        })}
      </div>

      {calendarId && (
        <form
          onSubmit={addEvent}
          className="flex flex-wrap items-end gap-2 border-t border-zinc-800 pt-4"
        >
          <input
            className="min-w-[10rem] flex-1 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            placeholder="Event title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
          <select
            className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            value={dayIndex}
            onChange={(e) => setDayIndex(Number(e.target.value))}
          >
            {window.days.map((day, i) => (
              <option key={day.toDateString()} value={i}>
                {formatWeekday(day)} {formatDayLabel(day)}
              </option>
            ))}
          </select>
          <label className="flex items-center gap-1 text-sm text-zinc-400">
            <input
              type="checkbox"
              checked={allDay}
              onChange={(e) => setAllDay(e.target.checked)}
            />
            All day
          </label>
          {!allDay && (
            <>
              <input
                type="time"
                className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
                value={startTime}
                onChange={(e) => setStartTime(e.target.value)}
              />
              <input
                type="time"
                className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
                value={endTime}
                onChange={(e) => setEndTime(e.target.value)}
              />
            </>
          )}
          <button
            type="submit"
            className="rounded border border-zinc-600 px-3 py-1 text-sm"
          >
            Add event
          </button>
        </form>
      )}
    </div>
  );
}
