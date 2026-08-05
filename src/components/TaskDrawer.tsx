import { FormEvent, useState } from "react";
import { formatDate } from "../lib/dates";
import {
  TASK_STATUSES,
  TASK_STATUS_LABELS,
  type TaskStatus,
} from "../lib/taskStatus";
import type { Task } from "./BoardView";
import { Select } from "./Select";

type Props = {
  task: Task;
  onClose: () => void;
  onSave: (patch: {
    title: string;
    description: string | null;
    dueDate: number | null;
    status: TaskStatus;
  }) => Promise<void>;
  onDelete: () => Promise<void>;
};

function startOfDayMs(yyyyMmDd: string): number | null {
  const [y, m, d] = yyyyMmDd.split("-").map(Number);
  if (!y || !m || !d) return null;
  return new Date(y, m - 1, d).getTime();
}

export function TaskDrawer({ task, onClose, onSave, onDelete }: Props) {
  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description ?? "");
  const [status, setStatus] = useState<TaskStatus>(
    TASK_STATUSES.includes(task.status as TaskStatus)
      ? (task.status as TaskStatus)
      : "open",
  );
  const [dueDate, setDueDate] = useState(
    task.due_date != null ? formatDate(new Date(task.due_date)) : "",
  );

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!title.trim()) return;
    const dueMs = dueDate.trim() ? startOfDayMs(dueDate.trim()) : null;
    await onSave({
      title: title.trim(),
      description: description.trim() ? description.trim() : null,
      dueDate: dueMs,
      status,
    });
    onClose();
  }

  return (
    <div className="fixed inset-0 z-40 flex justify-end bg-black/50">
      <button
        type="button"
        className="flex-1"
        aria-label="Close"
        onClick={onClose}
      />
      <form
        onSubmit={(e) => {
          submit(e).catch(() => {});
        }}
        className="flex h-full w-full max-w-md flex-col border-l border-zinc-800 bg-zinc-950 shadow-xl"
      >
        <div className="flex items-center justify-between border-b border-zinc-800 px-4 py-3">
          <h2 className="text-sm font-medium text-zinc-100">Task</h2>
          <button
            type="button"
            className="text-zinc-400 hover:text-zinc-200"
            onClick={onClose}
          >
            ✕
          </button>
        </div>
        <div className="flex flex-1 flex-col gap-3 overflow-y-auto p-4">
          <label className="block text-xs text-zinc-500">
            Title
            <input
              className="mt-1 w-full rounded border border-zinc-700 bg-zinc-900 px-2 py-1.5 text-sm"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              autoFocus
            />
          </label>
          <label className="block text-xs text-zinc-500">
            Description
            <textarea
              className="mt-1 min-h-[8rem] w-full rounded border border-zinc-700 bg-zinc-900 px-2 py-1.5 text-sm"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </label>
          <label className="block text-xs text-zinc-500">
            Status
            <div className="mt-1">
              <Select
                aria-label="Status"
                value={status}
                options={TASK_STATUSES.map((s) => ({
                  value: s,
                  label: TASK_STATUS_LABELS[s],
                }))}
                onChange={(v) => setStatus(v as TaskStatus)}
              />
            </div>
          </label>
          <label className="block text-xs text-zinc-500">
            Due date
            <input
              type="date"
              className="mt-1 w-full rounded border border-zinc-700 bg-zinc-900 px-2 py-1.5 text-sm"
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
            />
          </label>
        </div>
        <div className="flex justify-between gap-2 border-t border-zinc-800 p-4">
          <button
            type="button"
            className="rounded border border-red-900/50 px-3 py-1.5 text-sm text-red-400"
            onClick={() => {
              void onDelete().then(onClose);
            }}
          >
            Delete
          </button>
          <div className="flex gap-2">
            <button
              type="button"
              className="rounded border border-zinc-600 px-3 py-1.5 text-sm"
              onClick={onClose}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="rounded bg-zinc-100 px-3 py-1.5 text-sm text-zinc-900"
            >
              Save
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
