-- Hehel Zip local SQLite schema (MVP + phase 2 hooks)

CREATE TABLE IF NOT EXISTS workflow_statuses (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  label TEXT NOT NULL UNIQUE,
  color TEXT NOT NULL DEFAULT '#64748b',
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_default INTEGER NOT NULL DEFAULT 0,
  remote_id TEXT
);

CREATE TABLE IF NOT EXISTS entry_statuses (
  archive_path TEXT NOT NULL,
  entry_path TEXT NOT NULL,
  status_id INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (archive_path, entry_path),
  FOREIGN KEY (status_id) REFERENCES workflow_statuses(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS recent_archives (
  archive_path TEXT PRIMARY KEY,
  last_opened_at TEXT NOT NULL
);

-- Phase 2: stable archive identity
CREATE TABLE IF NOT EXISTS archive_registry (
  archive_path TEXT PRIMARY KEY,
  archive_id TEXT NOT NULL UNIQUE,
  sidecar_path TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  archive_id TEXT NOT NULL,
  entry_path TEXT NOT NULL,
  status_id INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  synced INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entry_statuses_archive ON entry_statuses(archive_path);
CREATE INDEX IF NOT EXISTS idx_sync_queue_pending ON sync_queue(synced) WHERE synced = 0;
