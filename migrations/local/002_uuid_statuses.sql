-- Hehel Zip local SQLite: INTEGER status ids -> UUID text

CREATE TABLE IF NOT EXISTS workflow_statuses_new (
  id TEXT PRIMARY KEY NOT NULL,
  label TEXT NOT NULL UNIQUE,
  color TEXT NOT NULL DEFAULT '#64748b',
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_default INTEGER NOT NULL DEFAULT 0,
  remote_id TEXT,
  server_updated_at TEXT,
  local_dirty INTEGER NOT NULL DEFAULT 0,
  deleted_at TEXT,
  sync_state TEXT NOT NULL DEFAULT 'synced'
);

CREATE TABLE IF NOT EXISTS entry_statuses_new (
  archive_path TEXT NOT NULL,
  entry_path TEXT NOT NULL,
  status_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  local_dirty INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (archive_path, entry_path),
  FOREIGN KEY (status_id) REFERENCES workflow_statuses_new(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sync_queue_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  archive_id TEXT NOT NULL,
  entry_path TEXT NOT NULL,
  status_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  synced INTEGER NOT NULL DEFAULT 0
);
