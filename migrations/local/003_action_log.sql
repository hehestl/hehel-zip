CREATE TABLE IF NOT EXISTS action_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  archive_id TEXT NOT NULL,
  archive_path TEXT,
  action_type TEXT NOT NULL,
  entry_path TEXT,
  from_status_id TEXT,
  to_status_id TEXT,
  detail TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_action_log_id ON action_log(archive_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_action_log_path ON action_log(archive_path, created_at DESC);

INSERT INTO app_settings (key, value) VALUES ('schema_version', '3')
ON CONFLICT(key) DO UPDATE SET value = '3';
