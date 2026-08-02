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
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { positionBetween } from "../lib/positioning";

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
  position: number;
  status: string;
};

function SortableTask({ task }: { task: Task }) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: task.id, data: { type: "task", task } });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      className="cursor-grab rounded border border-zinc-700 bg-zinc-800/80 px-2 py-1.5 text-sm active:cursor-grabbing"
    >
      {task.title}
    </div>
  );
}

function ColumnView({
  column,
  tasks,
  onAddTask,
}: {
  column: Column;
  tasks: Task[];
  onAddTask: (columnId: string, title: string) => Promise<void>;
}) {
  const [title, setTitle] = useState("");
  const ids = useMemo(() => tasks.map((t) => t.id), [tasks]);
  const { setNodeRef } = useDroppable({ id: column.id, data: { type: "column" } });

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!title.trim()) return;
    await onAddTask(column.id, title.trim());
    setTitle("");
  }

  return (
    <div
      ref={setNodeRef}
      className="flex w-64 shrink-0 flex-col rounded-lg border border-zinc-800 bg-zinc-900/50"
      data-column-id={column.id}
    >
      <h3 className="border-b border-zinc-800 px-3 py-2 text-sm font-medium">
        {column.name}
      </h3>
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        <div className="flex min-h-[120px] flex-col gap-2 p-2">
          {tasks.map((t) => (
            <SortableTask key={t.id} task={t} />
          ))}
        </div>
      </SortableContext>
      <form onSubmit={submit} className="border-t border-zinc-800 p-2">
        <input
          className="w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-xs"
          placeholder="Add task…"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
      </form>
    </div>
  );
}

export function BoardView({ onError }: { onError: (msg: string) => void }) {
  const [boards, setBoards] = useState<Board[]>([]);
  const [boardId, setBoardId] = useState<string | null>(null);
  const [columns, setColumns] = useState<Column[]>([]);
  const [tasksByColumn, setTasksByColumn] = useState<Record<string, Task[]>>({});
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const [newBoardName, setNewBoardName] = useState("");
  const [newColumnName, setNewColumnName] = useState("");

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

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

  const loadBoardData = useCallback(
    async (id: string) => {
      const cols = await invoke<Column[]>("list_columns_cmd", { boardId: id });
      setColumns(cols);
      const map: Record<string, Task[]> = {};
      for (const col of cols) {
        map[col.id] = await invoke<Task[]>("list_tasks_cmd", {
          columnId: col.id,
        });
      }
      setTasksByColumn(map);
    },
    [],
  );

  useEffect(() => {
    loadBoards().catch((e) => onError(String(e)));
  }, [loadBoards, onError]);

  useEffect(() => {
    if (!boardId) return;
    loadBoardData(boardId).catch((e) => onError(String(e)));
  }, [boardId, loadBoardData, onError]);

  async function createBoard() {
    if (!newBoardName.trim()) return;
    await invoke("create_board_cmd", { name: newBoardName.trim() });
    setNewBoardName("");
    await loadBoards();
  }

  async function createColumn() {
    if (!boardId || !newColumnName.trim()) return;
    await invoke("create_column_cmd", {
      boardId,
      name: newColumnName.trim(),
    });
    setNewColumnName("");
    await loadBoardData(boardId);
  }

  async function addTask(columnId: string, title: string) {
    await invoke("create_task_cmd", { columnId, title });
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
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm text-zinc-400">Board</span>
        <select
          className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          value={boardId ?? ""}
          onChange={(e) => setBoardId(e.target.value || null)}
        >
          {boards.length === 0 && <option value="">No boards</option>}
          {boards.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
            </option>
          ))}
        </select>
        <input
          className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          placeholder="New board"
          value={newBoardName}
          onChange={(e) => setNewBoardName(e.target.value)}
        />
        <button
          type="button"
          onClick={() => createBoard().catch((e) => onError(String(e)))}
          className="rounded bg-zinc-100 px-3 py-1 text-sm text-zinc-900"
        >
          Add board
        </button>
      </div>

      {boardId && (
        <div className="flex flex-wrap items-center gap-2">
          <input
            className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            placeholder="New column"
            value={newColumnName}
            onChange={(e) => setNewColumnName(e.target.value)}
          />
          <button
            type="button"
            onClick={() => createColumn().catch((e) => onError(String(e)))}
            className="rounded border border-zinc-600 px-3 py-1 text-sm"
          >
            Add column
          </button>
        </div>
      )}

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
                onAddTask={addTask}
              />
            </div>
          ))}
        </div>
        <DragOverlay>
          {activeTask ? (
            <div className="rounded border border-zinc-500 bg-zinc-800 px-2 py-1.5 text-sm shadow-lg">
              {activeTask.title}
            </div>
          ) : null}
        </DragOverlay>
      </DndContext>
    </div>
  );
}
