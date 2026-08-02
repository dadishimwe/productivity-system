import { invoke } from "@tauri-apps/api/core";
import { FormEvent, useCallback, useEffect, useState } from "react";
import { confirmDelete, promptRename, promptText } from "../lib/dialogs";
import { formatMoney, parseMoneyToCents } from "../lib/money";
import { IconButton } from "./IconButton";

type ShoppingList = {
  id: string;
  name: string;
  budget_limit: number | null;
};

type ShoppingItem = {
  id: string;
  list_id: string;
  name: string;
  qty: number;
  unit: string | null;
  unit_price: number | null;
  checked: boolean;
  category: string | null;
};

type ListSummary = {
  total_cents: number;
  item_count: number;
  checked_count: number;
};

function lineTotal(item: ShoppingItem): number | null {
  if (item.unit_price == null) return null;
  return Math.round(item.qty * item.unit_price);
}

export function ShoppingView({ onError }: { onError: (msg: string) => void }) {
  const [lists, setLists] = useState<ShoppingList[]>([]);
  const [listId, setListId] = useState<string | null>(null);
  const [items, setItems] = useState<ShoppingItem[]>([]);
  const [summary, setSummary] = useState<ListSummary | null>(null);
  const [newListName, setNewListName] = useState("");
  const [newBudget, setNewBudget] = useState("");
  const [itemName, setItemName] = useState("");
  const [itemQty, setItemQty] = useState("1");
  const [itemUnit, setItemUnit] = useState("");
  const [itemPrice, setItemPrice] = useState("");
  const [itemCategory, setItemCategory] = useState("");

  const currentList = lists.find((l) => l.id === listId);

  const loadLists = useCallback(async () => {
    const all = await invoke<ShoppingList[]>("list_shopping_lists_cmd");
    setLists(all);
    if (all.length === 0) {
      setListId(null);
      setItems([]);
      setSummary(null);
      return;
    }
    setListId((prev) =>
      prev && all.some((l) => l.id === prev) ? prev : all[0].id,
    );
  }, []);

  const loadListData = useCallback(async (id: string) => {
    const [rows, sum] = await Promise.all([
      invoke<ShoppingItem[]>("list_shopping_items_cmd", { listId: id }),
      invoke<ListSummary>("get_shopping_list_summary_cmd", { listId: id }),
    ]);
    setItems(rows);
    setSummary(sum);
  }, []);

  useEffect(() => {
    loadLists().catch((e) => onError(String(e)));
  }, [loadLists, onError]);

  useEffect(() => {
    if (!listId) return;
    loadListData(listId).catch((e) => onError(String(e)));
  }, [listId, loadListData, onError]);

  async function createList() {
    if (!newListName.trim()) return;
    const budget = parseMoneyToCents(newBudget);
    await invoke("create_shopping_list_cmd", {
      name: newListName.trim(),
      budgetLimit: budget,
    });
    setNewListName("");
    setNewBudget("");
    await loadLists();
  }

  async function renameList() {
    if (!listId || !currentList) return;
    const name = await promptRename(currentList.name, "list");
    if (!name) return;
    await invoke("rename_shopping_list_cmd", { id: listId, name });
    await loadLists();
  }

  async function deleteList() {
    if (!listId || !currentList) return;
    if (!(await confirmDelete(`list “${currentList.name}”`))) return;
    await invoke("delete_shopping_list_cmd", { id: listId });
    await loadLists();
  }

  async function setBudget() {
    if (!listId) return;
    const input = await promptText(
      "Budget limit (leave empty for no budget)",
      currentList?.budget_limit != null
        ? (currentList.budget_limit / 100).toFixed(2)
        : "",
    );
    if (input === null) return;
    const cents = input.trim() === "" ? null : parseMoneyToCents(input.trim());
    if (input.trim() !== "" && cents === null) {
      onError("Invalid budget amount");
      return;
    }
    await invoke("set_shopping_budget_cmd", { listId, budgetLimit: cents });
    await loadLists();
    await loadListData(listId);
  }

  async function addItem(e: FormEvent) {
    e.preventDefault();
    if (!listId || !itemName.trim()) return;
    const qty = parseFloat(itemQty) || 1;
    const price = itemPrice.trim()
      ? parseMoneyToCents(itemPrice.trim())
      : null;
    if (itemPrice.trim() && price === null) {
      onError("Invalid price");
      return;
    }
    await invoke("create_shopping_item_cmd", {
      listId,
      name: itemName.trim(),
      qty,
      unit: itemUnit.trim() || null,
      unitPrice: price,
      category: itemCategory.trim() || null,
    });
    setItemName("");
    setItemQty("1");
    setItemUnit("");
    setItemPrice("");
    setItemCategory("");
    await loadListData(listId);
  }

  async function toggleItem(item: ShoppingItem) {
    await invoke("toggle_shopping_item_cmd", { itemId: item.id });
    if (listId) await loadListData(listId);
  }

  async function deleteItem(item: ShoppingItem) {
    if (!(await confirmDelete(`item “${item.name}”`))) return;
    await invoke("delete_shopping_item_cmd", { itemId: item.id });
    if (listId) await loadListData(listId);
  }

  const budget = currentList?.budget_limit ?? null;
  const total = summary?.total_cents ?? 0;
  const overBudget = budget != null && total > budget;
  const progress =
    budget != null && budget > 0 ? Math.min(100, (total / budget) * 100) : 0;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm text-zinc-400">List</span>
        <select
          className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          value={listId ?? ""}
          onChange={(e) => setListId(e.target.value || null)}
        >
          {lists.length === 0 && <option value="">No lists</option>}
          {lists.map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
        </select>
        {listId && (
          <>
            <IconButton
              label="Rename"
              onClick={() => renameList().catch((e) => onError(String(e)))}
            />
            <button
              type="button"
              className="rounded px-1.5 py-0.5 text-xs text-zinc-400 hover:bg-zinc-800"
              onClick={() => setBudget().catch((e) => onError(String(e)))}
            >
              Budget
            </button>
            <IconButton
              label="Delete"
              onClick={() => deleteList().catch((e) => onError(String(e)))}
            />
          </>
        )}
        <input
          className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          placeholder="New list"
          value={newListName}
          onChange={(e) => setNewListName(e.target.value)}
        />
        <input
          className="w-24 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          placeholder="Budget"
          value={newBudget}
          onChange={(e) => setNewBudget(e.target.value)}
        />
        <button
          type="button"
          className="rounded bg-zinc-100 px-3 py-1 text-sm text-zinc-900"
          onClick={() => createList().catch((e) => onError(String(e)))}
        >
          Add list
        </button>
      </div>

      {listId && summary && (
        <div className="space-y-1">
          <div className="flex justify-between text-sm">
            <span>
              {formatMoney(total)}
              {budget != null ? ` / ${formatMoney(budget)}` : ""}
            </span>
            <span className="text-zinc-500">
              {summary.checked_count}/{summary.item_count} checked
            </span>
          </div>
          {budget != null && (
            <div className="h-2 overflow-hidden rounded-full bg-zinc-800">
              <div
                className={`h-full ${overBudget ? "bg-red-500" : "bg-emerald-500"}`}
                style={{ width: `${progress}%` }}
              />
            </div>
          )}
          {overBudget && (
            <p className="text-xs text-red-400">Over budget</p>
          )}
        </div>
      )}

      {listId && (
        <ul className="divide-y divide-zinc-800 rounded-lg border border-zinc-800">
          {items.map((item) => {
            const line = lineTotal(item);
            return (
              <li
                key={item.id}
                className="flex flex-wrap items-center gap-2 px-3 py-2 text-sm"
              >
                <input
                  type="checkbox"
                  checked={item.checked}
                  onChange={() =>
                    toggleItem(item).catch((e) => onError(String(e)))
                  }
                />
                <span
                  className={`min-w-[8rem] flex-1 ${item.checked ? "text-zinc-500 line-through" : ""}`}
                >
                  {item.name}
                </span>
                <span className="text-zinc-500">
                  {item.qty}
                  {item.unit ? ` ${item.unit}` : ""}
                </span>
                <span className="w-16 text-right tabular-nums">
                  {line != null ? formatMoney(line) : "—"}
                </span>
                {item.category && (
                  <span className="rounded bg-zinc-800 px-1.5 text-xs text-zinc-400">
                    {item.category}
                  </span>
                )}
                <IconButton
                  label="Delete"
                  onClick={() =>
                    deleteItem(item).catch((e) => onError(String(e)))
                  }
                />
              </li>
            );
          })}
        </ul>
      )}

      {listId && (
        <form
          onSubmit={addItem}
          className="flex flex-wrap gap-2 border-t border-zinc-800 pt-4"
        >
          <input
            className="min-w-[8rem] flex-1 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            placeholder="Add item…"
            value={itemName}
            onChange={(e) => setItemName(e.target.value)}
          />
          <input
            className="w-14 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            placeholder="Qty"
            value={itemQty}
            onChange={(e) => setItemQty(e.target.value)}
          />
          <input
            className="w-16 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            placeholder="Unit"
            value={itemUnit}
            onChange={(e) => setItemUnit(e.target.value)}
          />
          <input
            className="w-20 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            placeholder="Price"
            value={itemPrice}
            onChange={(e) => setItemPrice(e.target.value)}
          />
          <input
            className="w-24 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
            placeholder="Category"
            value={itemCategory}
            onChange={(e) => setItemCategory(e.target.value)}
          />
          <button
            type="submit"
            className="rounded border border-zinc-600 px-3 py-1 text-sm"
          >
            Add
          </button>
        </form>
      )}
    </div>
  );
}
