use super::constants::LABEL_SENT_TO_PRINT;
use super::types::{EntryStatusMap, WorkflowStatus};
use crate::error::{AppError, AppResult};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use uuid::Uuid;

type DbPool = Pool<SqliteConnectionManager>;

const CANONICAL_STATUSES: [(&str, &str, i64); 6] = [
    ("Предпродакшен", "#64748b", 0),
    (LABEL_SENT_TO_PRINT, "#3b82f6", 1),
    ("Отпечатано", "#22c55e", 2),
    ("Загрунтовано", "#a855f7", 3),
    ("Брак", "#ef4444", 4),
    ("Перепечатать", "#f97316", 5),
];

pub struct WorkflowDb {
    pool: DbPool,
}

impl WorkflowDb {
    fn borrow(&self) -> AppResult<PooledConnection<SqliteConnectionManager>> {
        self.pool
            .get()
            .map_err(|e| AppError::Database(format!("pool: {e}")))
    }

    pub fn open() -> AppResult<Self> {
        let path = db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)
            .map_err(|e| AppError::Database(format!("open: {e}")))?;
        migrate_connection(&conn)?;
        ensure_canonical_defaults_on(&conn)?;
        let manager = SqliteConnectionManager::file(&path);
        let pool = Pool::builder()
            .max_size(4)
            .build(manager)
            .map_err(|e| AppError::Database(format!("pool build: {e}")))?;
        Ok(Self { pool })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .map_err(|e| AppError::Database(format!("pool build: {e}")))?;
        let conn = pool
            .get()
            .map_err(|e| AppError::Database(format!("pool: {e}")))?;
        migrate_connection(&conn)?;
        ensure_canonical_defaults_on(&conn)?;
        Ok(Self { pool })
    }

    fn ensure_canonical_defaults(&self) -> AppResult<()> {
        let conn = self.borrow()?;
        ensure_canonical_defaults_on(&conn)
    }
}

fn migrate_connection(conn: &Connection) -> AppResult<()> {
    let sql = include_str!("../../../migrations/local/001_init.sql");
    conn.execute_batch(sql)
        .map_err(|e| AppError::Database(format!("migrate: {e}")))?;
    migrate_uuid_schema(conn)?;
    migrate_action_log(conn)?;
    Ok(())
}

fn migrate_action_log(conn: &Connection) -> AppResult<()> {
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .ok();
    if version.as_deref() == Some("3") {
        return Ok(());
    }
    let sql = include_str!("../../../migrations/local/003_action_log.sql");
    conn.execute_batch(sql)
        .map_err(|e| AppError::Database(format!("migrate action_log: {e}")))?;
    Ok(())
}

fn status_uuid_v5(label: &str) -> String {
    Uuid::new_v5(
        &Uuid::NAMESPACE_DNS,
        format!("hehel-zip:status:{label}").as_bytes(),
    )
    .to_string()
}

fn migrate_uuid_schema(conn: &Connection) -> AppResult<()> {
        let version: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .ok();
        if version.as_deref() == Some("2") {
            return Ok(());
        }
        let id_type: String = conn
            .query_row(
                "SELECT type FROM pragma_table_info('workflow_statuses') WHERE name = 'id'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "INTEGER".into());
        if id_type.to_uppercase() != "INTEGER" {
            conn
                .execute(
                    "INSERT INTO app_settings (key, value) VALUES ('schema_version', '2')
                     ON CONFLICT(key) DO UPDATE SET value = '2'",
                    [],
                )
                .ok();
            return Ok(());
        }

        if let Some(parent) = db_path()?.parent() {
            let _ = std::fs::copy(db_path()?, parent.join("data.db.bak"));
        }

        let sql = include_str!("../../../migrations/local/002_uuid_statuses.sql");
        conn
            .execute_batch(sql)
            .map_err(|e| AppError::Database(format!("migrate uuid: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, label, color, sort_order, is_default, remote_id FROM workflow_statuses",
        ).map_err(|e| AppError::Database(e.to_string()))?;
        let rows: Vec<(i64, String, String, i64, i64, Option<String>)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let mut id_map: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
        for (_old, label, color, sort_order, is_default, remote_id) in &rows {
            let new_id = status_uuid_v5(label);
            id_map.insert(*_old, new_id.clone());
            conn.execute(
                "INSERT INTO workflow_statuses_new (id, label, color, sort_order, is_default, remote_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![new_id, label, color, sort_order, is_default, remote_id],
            ).map_err(|e| AppError::Database(e.to_string()))?;
        }

        let mut estmt = conn.prepare(
            "SELECT archive_path, entry_path, status_id FROM entry_statuses",
        ).map_err(|e| AppError::Database(e.to_string()))?;
        let entries: Vec<(String, String, i64)> = estmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| AppError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        for (ap, ep, sid) in entries {
            if let Some(new_id) = id_map.get(&sid) {
                conn.execute(
                    "INSERT INTO entry_statuses_new (archive_path, entry_path, status_id, updated_at, local_dirty)
                     SELECT ?1, ?2, ?3, updated_at, 0 FROM entry_statuses WHERE archive_path = ?1 AND entry_path = ?2",
                    params![ap, ep, new_id],
                ).map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        conn.execute_batch(
            "DROP TABLE entry_statuses;
             ALTER TABLE entry_statuses_new RENAME TO entry_statuses;
             DROP TABLE sync_queue;
             CREATE TABLE sync_queue (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               archive_id TEXT NOT NULL,
               entry_path TEXT NOT NULL,
               status_id TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               synced INTEGER NOT NULL DEFAULT 0
             );
             DROP TABLE workflow_statuses;
             ALTER TABLE workflow_statuses_new RENAME TO workflow_statuses;
             INSERT INTO app_settings (key, value) VALUES ('schema_version', '2')
             ON CONFLICT(key) DO UPDATE SET value = '2';",
        ).map_err(|e| AppError::Database(format!("swap uuid tables: {e}")))?;

        Ok(())
}

fn ensure_canonical_defaults_on(conn: &Connection) -> AppResult<()> {
    for (label, color, sort_order) in CANONICAL_STATUSES {
        let id = status_uuid_v5(label);
        conn
            .execute(
                "INSERT INTO workflow_statuses (id, label, color, sort_order, is_default) VALUES (?1, ?2, ?3, ?4, 1)
                 ON CONFLICT(label) DO UPDATE SET
                   color = excluded.color,
                   sort_order = excluded.sort_order,
                   is_default = 1",
                params![id, label, color, sort_order],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
    }
    Ok(())
}

impl WorkflowDb {
    pub fn list_statuses(&self) -> AppResult<Vec<WorkflowStatus>> {
        let conn = self.borrow()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, label, color, sort_order, is_default FROM workflow_statuses ORDER BY sort_order, id",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], map_status)
            .map_err(|e| AppError::Database(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn create_status(&self, label: &str, color: &str) -> AppResult<WorkflowStatus> {
        let conn = self.borrow()?;
        let id = Uuid::new_v4().to_string();
        let sort_order: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM workflow_statuses",
                [],
                |r| r.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn
            .execute(
                "INSERT INTO workflow_statuses (id, label, color, sort_order, is_default) VALUES (?1, ?2, ?3, ?4, 0)",
                params![id, label, color, sort_order],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        self.get_status(&id)
    }

    pub fn update_status(
        &self,
        id: &str,
        label: &str,
        color: &str,
        sort_order: i64,
    ) -> AppResult<WorkflowStatus> {
        let conn = self.borrow()?;
        conn
            .execute(
                "UPDATE workflow_statuses SET label = ?1, color = ?2, sort_order = ?3 WHERE id = ?4",
                params![label, color, sort_order, id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        self.get_status(id)
    }

    pub fn delete_status(&self, id: &str) -> AppResult<()> {
        let conn = self.borrow()?;
        conn
            .execute("DELETE FROM workflow_statuses WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    fn get_status(&self, id: &str) -> AppResult<WorkflowStatus> {
        let conn = self.borrow()?;
        conn
            .query_row(
                "SELECT id, label, color, sort_order, is_default FROM workflow_statuses WHERE id = ?1",
                params![id],
                map_status,
            )
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn get_entry_statuses(&self, archive_path: &str) -> AppResult<EntryStatusMap> {
        let conn = self.borrow()?;
        let mut stmt = conn
            .prepare(
                "SELECT entry_path, status_id FROM entry_statuses WHERE archive_path = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params![archive_path], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut map = EntryStatusMap::new();
        for row in rows {
            let (path, status_id) = row.map_err(|e| AppError::Database(e.to_string()))?;
            map.insert(path, status_id);
        }
        Ok(map)
    }

    pub fn set_entry_status(
        &self,
        archive_path: &str,
        entry_path: &str,
        status_id: Option<&str>,
    ) -> AppResult<()> {
        let conn = self.borrow()?;
        let now = chrono::Utc::now().to_rfc3339();
        match status_id {
            Some(sid) => {
                conn
                    .execute(
                        "INSERT INTO entry_statuses (archive_path, entry_path, status_id, updated_at) VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(archive_path, entry_path) DO UPDATE SET status_id = excluded.status_id, updated_at = excluded.updated_at",
                        params![archive_path, entry_path, sid, now],
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
            None => {
                conn
                    .execute(
                        "DELETE FROM entry_statuses WHERE archive_path = ?1 AND entry_path = ?2",
                        params![archive_path, entry_path],
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn touch_recent(&self, archive_path: &str) -> AppResult<()> {
        let conn = self.borrow()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn
            .execute(
                "INSERT INTO recent_archives (archive_path, last_opened_at) VALUES (?1, ?2)
                 ON CONFLICT(archive_path) DO UPDATE SET last_opened_at = excluded.last_opened_at",
                params![archive_path, now],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn list_recent(&self) -> AppResult<Vec<String>> {
        let conn = self.borrow()?;
        let mut stmt = conn
            .prepare(
                "SELECT archive_path FROM recent_archives ORDER BY last_opened_at DESC LIMIT 20",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn register_archive(&self, archive_path: &str, archive_id: &str, sidecar_path: Option<&str>) -> AppResult<()> {
        let conn = self.borrow()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn
            .execute(
                "INSERT INTO archive_registry (archive_path, archive_id, sidecar_path, created_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(archive_path) DO UPDATE SET archive_id = excluded.archive_id, sidecar_path = excluded.sidecar_path",
                params![archive_path, archive_id, sidecar_path, now],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_archive_id(&self, archive_path: &str) -> AppResult<String> {
        let conn = self.borrow()?;
        conn
            .query_row(
                "SELECT archive_id FROM archive_registry WHERE archive_path = ?1",
                params![archive_path],
                |r| r.get(0),
            )
            .map_err(|_| AppError::Validation("archive_id не найден".into()))
    }

    pub fn relink_archive_path(&self, old_path: &str, new_path: &str) -> AppResult<u32> {
        let conn = self.borrow()?;
        let updated = conn
            .execute(
                "UPDATE entry_statuses SET archive_path = ?2 WHERE archive_path = ?1",
                params![old_path, new_path],
            )
            .map_err(|e| AppError::Database(e.to_string()))? as u32;
        conn
            .execute(
                "UPDATE archive_registry SET archive_path = ?2 WHERE archive_path = ?1",
                params![old_path, new_path],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn
            .execute(
                "UPDATE recent_archives SET archive_path = ?2 WHERE archive_path = ?1",
                params![old_path, new_path],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn
            .execute(
                "UPDATE action_log SET archive_path = ?2 WHERE archive_path = ?1",
                params![old_path, new_path],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(updated)
    }

    pub fn find_path_by_archive_id(&self, archive_id: &str) -> AppResult<Option<String>> {
        let conn = self.borrow()?;
        let result = conn.query_row(
            "SELECT archive_path FROM archive_registry WHERE archive_id = ?1 LIMIT 1",
            params![archive_id],
            |r| r.get(0),
        );
        match result {
            Ok(path) => Ok(Some(path)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn set_status_remote_id(&self, status_id: &str, remote_id: &str) -> AppResult<()> {
        let conn = self.borrow()?;
        conn
            .execute(
                "UPDATE workflow_statuses SET remote_id = ?1 WHERE id = ?2",
                params![remote_id, status_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn find_status_id_by_remote_id(&self, remote_id: &str) -> AppResult<Option<String>> {
        let conn = self.borrow()?;
        let result = conn.query_row(
            "SELECT id FROM workflow_statuses WHERE remote_id = ?1",
            params![remote_id],
            |r| r.get(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn enqueue_sync(&self, archive_id: &str, entry_path: &str, status_id: &str) -> AppResult<()> {
        let conn = self.borrow()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn
            .execute(
                "INSERT INTO sync_queue (archive_id, entry_path, status_id, updated_at, synced) VALUES (?1, ?2, ?3, ?4, 0)",
                params![archive_id, entry_path, status_id, now],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn list_pending_sync(&self) -> AppResult<Vec<crate::sync::SyncQueueItem>> {
        let conn = self.borrow()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, archive_id, entry_path, status_id, updated_at FROM sync_queue WHERE synced = 0 ORDER BY id",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| {
                Ok(crate::sync::SyncQueueItem {
                    id: r.get(0)?,
                    archive_id: r.get(1)?,
                    entry_path: r.get(2)?,
                    status_id: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn mark_synced(&self, items: &[crate::sync::SyncQueueItem]) -> AppResult<()> {
        let conn = self.borrow()?;
        for item in items {
            conn
                .execute(
                    "UPDATE sync_queue SET synced = 1 WHERE id = ?1",
                    params![item.id],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    pub fn get_sync_config(&self) -> AppResult<crate::sync::SyncConfig> {
        let conn = self.borrow()?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'sync_config'",
                [],
                |r| r.get(0),
            )
            .ok();
        match raw {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Database(format!("sync_config parse: {e}"))),
            None => Ok(crate::sync::SyncConfig::default()),
        }
    }

    pub fn save_sync_config(&self, config: &crate::sync::SyncConfig) -> AppResult<()> {
        let conn = self.borrow()?;
        let json = serde_json::to_string(config)
            .map_err(|e| AppError::Database(format!("sync_config serialize: {e}")))?;
        conn
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('sync_config', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![json],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn resolve_remote_status_id(&self, local_id: &str) -> AppResult<String> {
        let conn = self.borrow()?;
        conn
            .query_row(
                "SELECT COALESCE(remote_id, id) FROM workflow_statuses WHERE id = ?1",
                params![local_id],
                |r| r.get(0),
            )
            .map_err(|_| AppError::Sync(format!("локальный статус {local_id} не найден")))
    }

    pub fn sync_remote_ids_from_labels(
        &self,
        remote_statuses: &[crate::sync::RemoteWorkflowStatus],
    ) -> AppResult<()> {
        let conn = self.borrow()?;
        for remote in remote_statuses {
            conn
                .execute(
                    "UPDATE workflow_statuses SET remote_id = ?1 WHERE label = ?2",
                    params![remote.id, remote.label],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    pub fn merge_remote_statuses(
        &self,
        archive_path: &str,
        links: &[crate::sync::RemoteArchiveLink],
    ) -> AppResult<u32> {
        let mut count = 0u32;
        for link in links {
            let local_id = self
                .find_status_id_by_remote_id(&link.workflow_status_id)?
                .ok_or_else(|| {
                    AppError::Sync(format!(
                        "Локальный статус для remote {} не найден",
                        link.workflow_status_id
                    ))
                })?;
            self.set_entry_status(archive_path, &link.entry_path, Some(local_id.as_str()))?;
            count += 1;
        }
        Ok(count)
    }

    pub fn get_entry_status(
        &self,
        archive_path: &str,
        entry_path: &str,
    ) -> AppResult<Option<String>> {
        let conn = self.borrow()?;
        let result = conn.query_row(
            "SELECT status_id FROM entry_statuses WHERE archive_path = ?1 AND entry_path = ?2",
            params![archive_path, entry_path],
            |r| r.get(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn log_action(
        &self,
        archive_id: &str,
        archive_path: Option<&str>,
        action_type: &str,
        entry_path: Option<&str>,
        from_status_id: Option<&str>,
        to_status_id: Option<&str>,
        detail: Option<&str>,
    ) -> AppResult<()> {
        let conn = self.borrow()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn
            .execute(
                "INSERT INTO action_log (archive_id, archive_path, action_type, entry_path, from_status_id, to_status_id, detail, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    archive_id,
                    archive_path,
                    action_type,
                    entry_path,
                    from_status_id,
                    to_status_id,
                    detail,
                    now,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_action_log(
        &self,
        archive_id: &str,
        limit: u32,
    ) -> AppResult<Vec<super::types::ActionLogEntry>> {
        let conn = self.borrow()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, archive_id, archive_path, action_type, entry_path, from_status_id, to_status_id, detail, created_at
                 FROM action_log WHERE archive_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params![archive_id, limit], |r| {
                Ok(super::types::ActionLogEntry {
                    id: r.get(0)?,
                    archive_id: r.get(1)?,
                    archive_path: r.get(2)?,
                    action_type: r.get(3)?,
                    entry_path: r.get(4)?,
                    from_status_id: r.get(5)?,
                    to_status_id: r.get(6)?,
                    detail: r.get(7)?,
                    created_at: r.get(8)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

fn map_status(r: &rusqlite::Row<'_>) -> rusqlite::Result<WorkflowStatus> {
    Ok(WorkflowStatus {
        id: r.get(0)?,
        label: r.get(1)?,
        color: r.get(2)?,
        sort_order: r.get(3)?,
        is_default: r.get::<_, i64>(4)? != 0,
    })
}

pub fn db_path() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| AppError::Database("APPDATA недоступен".into()))?;
    Ok(base.join("Hehel-Zip").join("data.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_status_lookup_by_archive_path() {
        let db = WorkflowDb::open_in_memory().unwrap();
        let statuses = db.list_statuses().unwrap();
        let sid = statuses[0].id.clone();
        db.set_entry_status("D:\\a.zip", "part.stl", Some(sid.as_str()))
            .unwrap();
        let map = db.get_entry_statuses("D:\\a.zip").unwrap();
        assert_eq!(map.get("part.stl"), Some(&statuses[0].id));
    }

    #[test]
    fn ensure_canonical_defaults_has_six_statuses() {
        let db = WorkflowDb::open_in_memory().unwrap();
        let statuses = db.list_statuses().unwrap();
        assert_eq!(statuses.len(), 6);
        assert_eq!(statuses[0].label, "Предпродакшен");
        assert_eq!(statuses[5].label, "Перепечатать");

        db.ensure_canonical_defaults().unwrap();
        let again = db.list_statuses().unwrap();
        assert_eq!(again.len(), 6);
    }
}
