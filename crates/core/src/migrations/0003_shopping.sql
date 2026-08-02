CREATE TABLE shopping_lists (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  budget_limit INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER
);

CREATE TABLE shopping_items (
  id TEXT PRIMARY KEY,
  list_id TEXT NOT NULL REFERENCES shopping_lists(id),
  name TEXT NOT NULL,
  qty REAL NOT NULL DEFAULT 1,
  unit TEXT,
  unit_price INTEGER,
  checked INTEGER NOT NULL DEFAULT 0,
  category TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER
);

CREATE INDEX idx_shopping_items_list ON shopping_items(list_id);
