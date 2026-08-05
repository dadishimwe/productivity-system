CREATE TABLE google_accounts (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER
);

ALTER TABLE calendars ADD COLUMN sync_token TEXT;
ALTER TABLE calendars ADD COLUMN external_calendar_id TEXT;
