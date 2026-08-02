# Productivity app

Local-first personal productivity (Tauri 2 + Rust core + SQLite + React).

## Setup

```bash
cd "/Users/HP/Documents/Projects/Productivity app"
source "$HOME/.cargo/env"

# Rust uses compile-time SQLx with the committed cache (no live DB needed to compile):
# crates/core/.cargo/config.toml sets SQLX_OFFLINE=true

npm install
npm test

cargo test --workspace   # from repo root
npm run tauri dev
```

If you see **`unable to open database file`** during `cargo build` / `cargo test`, either:

1. **`crates/core/.sqlx/` is missing** — regenerate and commit it:

```bash
cd crates/core
chmod +x scripts/prepare-sqlx.sh
./scripts/prepare-sqlx.sh
cd ../..
git add crates/core/.sqlx
```

2. **`SQLX_OFFLINE` was overridden** — ensure you did not `export SQLX_OFFLINE=false`, and that `crates/core/.cargo/config.toml` exists.

After changing SQL or migrations, run `./scripts/prepare-sqlx.sh` again and commit `.sqlx/`.

## Layout

- `crates/core` — business logic, migrations, outbox, positioning
- `src-tauri` — thin Tauri IPC wrappers
- `src` — React UI (board, habits, shopping)

## Phase 1

- **Board** — multiple boards, columns, drag-and-drop tasks (`dnd-kit`), fractional positions via `move_task_cmd`; rename/delete boards, columns, and tasks from the UI
- **Habits** — habit list + GitHub-style heatmap; click toggles `log_habit_cmd` / `unlog_habit_cmd`; rename/delete habits

## Phase 2

- **Shopping** — multiple lists with optional budget, line items (qty, unit, price in cents), check-off, running total vs budget, and outbox entries for sync
