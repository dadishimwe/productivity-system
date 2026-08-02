CREATE TABLE event_exceptions (
  id TEXT PRIMARY KEY,
  event_id TEXT NOT NULL REFERENCES events(id),
  original_start_ms INTEGER NOT NULL,
  override_start_ms INTEGER,
  override_end_ms INTEGER,
  cancelled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_event_exceptions_instance
  ON event_exceptions(event_id, original_start_ms);

CREATE INDEX idx_event_exceptions_event ON event_exceptions(event_id);
