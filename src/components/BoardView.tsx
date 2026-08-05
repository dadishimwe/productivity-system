import {
  DndContext,
  DragEndEvent,
  DragOverlay,
  DragStartEvent,
  PointerSensor,
  useDroppable,
  useSensor,
  useSensors,
  closestCorners,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { invoke } from "@tauri-apps/api/core";
import {
  FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { confirmDelete, promptDialog, promptRename } from "../lib/dialogs";
import { positionBetween } from "../lib/positioning";
import { statusLabel } from "../lib/taskStatus";
import { showToast } from "./ToastHost";
import { MenuButton } from "./Menu";
import { TaskDrawer } from "./TaskDrawer";

export type Board = { id: string; name: string; position: number };
export type Column = {
  id: string;
  board_id: string;
  name: string;
  position: number;
};
export type Task = {
  id: string;
  column_id: string;
  title: string;
  description: string | null;
  position: number;
  due_date: number | null;
  status: string;
};

function taskMatchesQuery(task: Task, q: string): boolean {
  const needle = q.toLowerCase();
  if (task.title.toLowerCase().includes(needle)) return true;
  if (task.description?.toLowerCase().includes(needle)) return true;
  return false;
}

function SortableTask({
  task,
  dimmed,
  onOpen,
  onDelete,
}: {
  task: Task;
  dimmed?: boolean;
  onOpen: () => void;
  onDelete: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: task.id, data: { type: "task", task } });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : dimmed ? 0.35 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="rounded-md border border-zinc-600/80 bg-[#2D2D35] px-2 py-2 text-sm shadow-sm"
    >
      <div className="flex items-start gap-1">
        <button
          type="button"
          className="cursor-grab touch-none px-0.5 text-zinc-500 hover:text-zinc-300 active:cursor-grabbing"
          aria-label="Drag"
          {...attributes}
          {...listeners}
        >
          ⠿
        </button>
        <button
          type="button"
          className="min-w-0 flex-1 text-left"
          onClick={onOpen}
        >
          <div className="truncate font-medium text-zinc-100">{task.title}</div>
          {task.status !== "open" && (
            <span className="mt-1 inline-block rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-zinc-400">
              {statusLabel(task.status)}
            </span>
          )}
        </button>
        <MenuButton
          items={[
            { label: "Open", onClick: onOpen },
            { label: "Delete", onClick: onDelete, danger: true },
          ]}
        />
      </div>
    </div>
  );
}

function ColumnView({
  column,
  tasks,
  searchQuery,
  onAddTask,
  onRename,
  onDelete,
  onOpenTask,
  onDeleteTask,
  onError,
}: {
  column: Column;
  tasks: Task[];
  searchQuery: string;
  onAddTask: (columnId: string, title: string) => Promise<void>;
  onRename: (id: string, name: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onOpenTask: (task: Task) => void;
  onDeleteTask: (id: string) => Promise<void>;
  onError: (msg: string) => void;
}) {
  const [title, setTitle] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const ids = useMemo(() => tasks.map((t) => t.id), [tasks]);
  const { setNodeRef, isOver } = useDroppable({
    id: column.id,
    data: { type: "column" },
  });

  const q = searchQuery.trim().toLowerCase();
  const hasFilter = q.length > 0;

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!title.trim()) return;
    await onAddTask(column.id, title.trim());
    setTitle("");
    inputRef.current?.focus();
  }

  return (
    <div
      ref={setNodeRef}
      className={`flex w-72 shrink-0 flex-col rounded-lg border bg-[#1E1E24] ${
        isOver ? "border-emerald-500/60 ring-1 ring-emerald-500/40" : "border-zinc-700/80"
      }`}
      data-column-id={column.id}
    >
      <div className="flex items-center gap-1 border-b border-zinc-700/60 px-2 py-2">
        <h3 className="min-w-0 flex-1 truncate text-sm font-medium text-zinc-100">
          {column.name}
          <span className="ml-1 text-zinc-500">({tasks.length})</span>
        </h3>
        <MenuButton
          items={[
            {
              label: "Rename",
              onClick: () => {
                void (async () => {
                  const name = await promptRename(column.name, "column");
                  if (name) await onRename(column.id, name);
                })().catch((e) => onError(String(e)));
              },
            },
            {
              label: "Delete column",
              danger: true,
              onClick: () => {
                void (async () => {
                  if (await confirmDelete(`column “${column.name}”`)) {
                    await onDelete(column.id);
                  }
                })().catch((e) => onError(String(e)));
              },
            },
          ]}
        />
      </div>
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        <div className="flex min-h-[140px] flex-col gap-2 p-2">
          {tasks.map((t) => (
            <SortableTask
              key={t.id}
              task={t}
              dimmed={hasFilter && !taskMatchesQuery(t, q)}
              onOpen={() => onOpenTask(t)}
              onDelete={() => {
                void (async () => {
                  if (await confirmDelete(`task “${t.title}”`)) {
                    await onDeleteTask(t.id);
                  }
                })().catch((e) => onError(String(e)));
              }}
            />
          ))}
        </div>
      </SortableContext>
      <form onSubmit={submit} className="border-t border-zinc-700/60 p-2">
        <input
          ref={inputRef}
          className="w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-xs placeholder:text-zinc-500"
          placeholder="+ Add a task"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
      </form>
    </div>
  );
}

function AddColumnCard({
  onAdd,
  onError,
}: {
  onAdd: (name: string) => Promise<void>;
  onError: (msg: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    await onAdd(name.trim());
    setName("");
    setOpen(false);
  }

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex h-full min-h-[200px] w-72 shrink-0 items-center justify-center rounded-lg border border-dashed border-zinc-700 text-sm text-zinc-500 hover:border-zinc-500 hover:text-zinc-300"
      >
        + Column
      </button>
    );
  }

  return (
    <form
      onSubmit={(e) => {
        submit(e).catch((err) => onError(String(err)));
      }}
      className="flex w-72 shrink-0 flex-col gap-2 rounded-lg border border-zinc-700 bg-[#1E1E24] p-3"
    >
      <input
        autoFocus
        className="rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
        placeholder="Column name"
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <div className="flex gap-2">
        <button
          type="submit"
          className="rounded bg-zinc-100 px-2 py-1 text-xs text-zinc-900"
        >
          Add
        </button>
        <button
          type="button"
          className="rounded border border-zinc-600 px-2 py-1 text-xs"
          onClick={() => setOpen(false)}
        >
          Cancel
        </button>
      </div>
    </form>
  );
}

export function BoardView({ onError }: { onError: (msg: string) => void }) {
  const [boards, setBoards] = useState<Board[]>([]);
  const [boardId, setBoardId] = useState<string | null>(null);
  const [columns, setColumns] = useState<Column[]>([]);
  const [tasksByColumn, setTasksByColumn] = useState<Record<string, Task[]>>({});
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

  const currentBoard = boards.find((b) => b.id === boardId);

  const loadBoards = useCallback(async () => {
    const list = await invoke<Board[]>("list_boards_cmd");
    setBoards(list);
    if (list.length === 0) {
      setBoardId(null);
      setColumns([]);
      setTasksByColumn({});
      return;
    }
    setBoardId((prev) =>
      prev && list.some((b) => b.id === prev) ? prev : list[0].id,
    );
  }, []);

  const loadBoardData = useCallback(async (id: string) => {
    const cols = await invoke<Column[]>("list_columns_cmd", { boardId: id });
    setColumns(cols);
    const map: Record<string, Task[]> = {};
    for (const col of cols) {
      map[col.id] = await invoke<Task[]>("list_tasks_cmd", {
        columnId: col.id,
      });
    }
    setTasksByColumn(map);
  }, []);

  useEffect(() => {
    loadBoards().catch((e) => onError(String(e)));
  }, [loadBoards, onError]);

  useEffect(() => {
    if (!boardId) return;
    loadBoardData(boardId).catch((e) => onError(String(e)));
  }, [boardId, loadBoardData, onError]);

  async function createBoard(name: string) {
    await invoke("create_board_cmd", { name });
    await loadBoards();
  }

  async function createColumn(name: string) {
    if (!boardId) return;
    await invoke("create_column_cmd", { boardId, name });
    await loadBoardData(boardId);
  }

  async function addTask(columnId: string, title: string) {
    await invoke("create_task_cmd", { columnId, title });
    if (boardId) await loadBoardData(boardId);
  }

  async function renameColumn(id: string, name: string) {
    await invoke("rename_column_cmd", { id, name });
    if (boardId) await loadBoardData(boardId);
  }

  async function deleteColumn(id: string) {
    await invoke("delete_column_cmd", { id });
    if (boardId) await loadBoardData(boardId);
  }

  async function deleteTask(taskId: string) {
    if (!(await confirmDelete("this task"))) return;
    await invoke("delete_task_cmd", { taskId });
    showToast("Task deleted");
    if (boardId) await loadBoardData(boardId);
  }

  async function saveTask(
    task: Task,
    patch: {
      title: string;
      description: string | null;
      dueDate: number | null;
      status: string;
    },
  ) {
    await invoke("update_task_cmd", {
      id: task.id,
      title: patch.title,
      description: patch.description,
      dueDate: patch.dueDate,
      status: patch.status,
    });
    if (boardId) await loadBoardData(boardId);
  }

  function findTask(taskId: string): { task: Task; columnId: string } | null {
    for (const [columnId, list] of Object.entries(tasksByColumn)) {
      const task = list.find((t) => t.id === taskId);
      if (task) return { task, columnId };
    }
    return null;
  }

  function computeDropPosition(
    targetColumnId: string,
    overTaskId: string | null,
    activeTaskId: string,
  ): number {
    const list = (tasksByColumn[targetColumnId] ?? [])
      .filter((t) => t.id !== activeTaskId)
      .sort((a, b) => a.position - b.position);

    if (!overTaskId) {
      const last = list[list.length - 1];
      return last ? last.position + 1 : 0;
    }

    const idx = list.findIndex((t) => t.id === overTaskId);
    if (idx === -1) {
      const last = list[list.length - 1];
      return last ? last.position + 1 : 0;
    }
    const before = idx > 0 ? list[idx - 1].position : null;
    const after = list[idx].position;
    return positionBetween(before, after);
  }

  function onDragStart(event: DragStartEvent) {
    const found = findTask(String(event.active.id));
    if (found) setActiveTask(found.task);
  }

  async function onDragEnd(event: DragEndEvent) {
    setActiveTask(null);
    const { active, over } = event;
    if (!over) return;

    const activeId = String(active.id);
    const found = findTask(activeId);
    if (!found) return;

    let targetColumnId = found.columnId;
    let overTaskId: string | null = null;
    const overData = over.data.current;
    if (overData?.type === "task") {
      const overTask = overData.task as Task;
      targetColumnId = overTask.column_id;
      overTaskId = overTask.id;
    } else if (columns.some((c) => c.id === String(over.id))) {
      targetColumnId = String(over.id);
    }

    const newPosition = computeDropPosition(
      targetColumnId,
      overTaskId,
      activeId,
    );

    if (
      targetColumnId === found.columnId &&
      Math.abs(newPosition - found.task.position) < 1e-12
    ) {
      return;
    }

    const prevCopy = structuredClone(tasksByColumn);
    setTasksByColumn((current) => {
      const next: Record<string, Task[]> = {};
      for (const [cid, list] of Object.entries(current)) {
        next[cid] = list.filter((t) => t.id !== activeId);
      }
      const moved = {
        ...found.task,
        column_id: targetColumnId,
        position: newPosition,
      };
      next[targetColumnId] = [...(next[targetColumnId] ?? []), moved].sort(
        (a, b) => a.position - b.position,
      );
      return next;
    });

    try {
      await invoke("move_task_cmd", {
        taskId: activeId,
        newColumnId: targetColumnId,
        newPosition,
      });
    } catch (e) {
      setTasksByColumn(prevCopy);
      onError(String(e));
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-3 border-b border-zinc-800 pb-3">
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1">
          {boards.map((b) => (
            <div key={b.id} className="flex items-center">
              <button
                type="button"
                onClick={() => setBoardId(b.id)}
                className={`rounded-md px-3 py-1.5 text-sm ${
                  boardId === b.id
                    ? "bg-zinc-100 font-medium text-zinc-900"
                    : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
                }`}
              >
                {b.name}
              </button>
              {boardId === b.id && (
                <MenuButton
                  label="Board options"
                  items={[
                    {
                      label: "Rename board",
                      onClick: () => {
                        void (async () => {
                          const name = await promptRename(b.name, "board");
                          if (!name) return;
                          await invoke("rename_board_cmd", { id: b.id, name });
                          await loadBoards();
                        })().catch((e) => onError(String(e)));
                      },
                    },
                    {
                      label: "Delete board",
                      danger: true,
                      onClick: () => {
                        void (async () => {
                          if (!(await confirmDelete(`board “${b.name}”`)))
                            return;
                          await invoke("delete_board_cmd", { id: b.id });
                          await loadBoards();
                        })().catch((e) => onError(String(e)));
                      },
                    },
                  ]}
                />
              )}
            </div>
          ))}
          <button
            type="button"
            className="rounded-md border border-dashed border-zinc-600 px-2 py-1.5 text-sm text-zinc-400 hover:border-zinc-500 hover:text-zinc-200"
            onClick={() => {
              void (async () => {
                const name = await promptDialog("New board name");
                if (name?.trim()) await createBoard(name.trim());
              })().catch((e) => onError(String(e)));
            }}
          >
            + Board
          </button>
        </div>
        <input
          type="search"
          placeholder="Search tasks…"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full min-w-[12rem] max-w-xs rounded-md border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-sm placeholder:text-zinc-500 sm:w-64"
        />
      </div>

      {!boardId && (
        <p className="text-sm text-zinc-500">
          Create a board to start tracking tasks.
        </p>
      )}

      {boardId && currentBoard && (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCorners}
          onDragStart={onDragStart}
          onDragEnd={(e) => {
            onDragEnd(e).catch((err) => onError(String(err)));
          }}
        >
          <div className="flex gap-3 overflow-x-auto pb-4">
            {columns.map((col) => (
              <div key={col.id} data-column-id={col.id} id={col.id}>
                <ColumnView
                  column={col}
                  tasks={tasksByColumn[col.id] ?? []}
                  searchQuery={searchQuery}
                  onAddTask={addTask}
                  onRename={renameColumn}
                  onDelete={deleteColumn}
                  onOpenTask={setSelectedTask}
                  onDeleteTask={deleteTask}
                  onError={onError}
                />
              </div>
            ))}
            <AddColumnCard
              onAdd={createColumn}
              onError={onError}
            />
          </div>
          <DragOverlay dropAnimation={null}>
            {activeTask ? (
              <div className="w-64 rounded-md border border-emerald-500/50 bg-[#2D2D35] px-2 py-2 text-sm shadow-xl">
                {activeTask.title}
              </div>
            ) : null}
          </DragOverlay>
        </DndContext>
      )}

      {selectedTask && (
        <TaskDrawer
          task={selectedTask}
          onClose={() => setSelectedTask(null)}
          onSave={(patch) => saveTask(selectedTask, patch)}
          onDelete={() => deleteTask(selectedTask.id)}
        />
      )}
    </div>
  );
}
