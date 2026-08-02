#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export DATABASE_URL="${DATABASE_URL:-sqlite://${PWD}/productivity.db}"
unset SQLX_OFFLINE
cargo install sqlx-cli --no-default-features --features sqlite --locked 2>/dev/null || true
rm -f productivity.db
sqlx database create
sqlx migrate run --source src/migrations
cargo sqlx prepare
echo "Commit crates/core/.sqlx/ after this script succeeds."
