import { describe, expect, it, vi, beforeEach } from "vitest";
import { positionBetween } from "./positioning";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

describe("board workflow", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("persists move_task with fractional position between neighbors", async () => {
    const board = { id: "b1", name: "Work", position: 0 };
    const column = { id: "c1", board_id: "b1", name: "Todo", position: 0 };
    const tasks = [
      { id: "t1", column_id: "c1", title: "A", position: 0, status: "open" },
      { id: "t2", column_id: "c1", title: "B", position: 1, status: "open" },
    ];

    vi.mocked(invoke).mockImplementation(async (cmd, args) => {
      if (cmd === "list_boards_cmd") return [board];
      if (cmd === "list_columns_cmd") return [column];
      if (cmd === "list_tasks_cmd") return tasks;
      if (cmd === "move_task_cmd") return { ...tasks[1], position: args?.newPosition };
      return null;
    });

    const newPos = positionBetween(tasks[0].position, tasks[1].position);
    await invoke("move_task_cmd", {
      taskId: "t2",
      newColumnId: "c1",
      newPosition: newPos,
    });

    expect(invoke).toHaveBeenCalledWith("move_task_cmd", {
      taskId: "t2",
      newColumnId: "c1",
      newPosition: 0.5,
    });

    vi.mocked(invoke).mockImplementation(async (cmd) => {
      if (cmd === "list_tasks_cmd")
        return [
          tasks[0],
          { ...tasks[1], position: 0.5 },
        ].sort((a, b) => a.position - b.position);
      return [];
    });

    const reloaded = (await invoke("list_tasks_cmd", {
      columnId: "c1",
    })) as typeof tasks;
    expect(reloaded.map((t) => t.title)).toEqual(["A", "B"]);
    expect(reloaded[1].position).toBe(0.5);
  });
});

describe("positionBetween", () => {
  it("matches rust midpoint", () => {
    expect(positionBetween(0, 10)).toBe(5);
  });
});
