# Productivity app

Local-first personal productivity (Tauri 2 + Rust core + SQLite + React).

## Setup

```bash
# SQLx offline cache (required before Rust build — CI enforces with sqlx prepare --check)
cd crates/core
chmod +x scripts/prepare-sqlx.sh
./scripts/prepare-sqlx.sh
git add .sqlx

cd ../..
npm install
npm test
cargo test --workspace
npm run tauri dev
```

All SQL in `productivity-core` uses compile-time `query!` / `query_as!`. `crates/core/.sqlx/` must stay in sync; run `./scripts/prepare-sqlx.sh` after any query or migration change.

## Layout

- `crates/core` — business logic, migrations, outbox, positioning
- `src-tauri` — thin Tauri IPC wrappers
- `src` — React UI (board + habits)

## Phase 1

- **Board** — multiple boards, columns, drag-and-drop tasks (`dnd-kit`), fractional positions via `move_task_cmd`
- **Habits** — habit list + GitHub-style heatmap; click toggles `log_habit_cmd` / `unlog_habit_cmd`
