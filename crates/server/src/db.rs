use rusqlite::{params, Connection};
use serde_json::Value;
use std::sync::Mutex;

/// Thread-safe wrapper around a SQLite connection for the sync server.
pub struct SyncDb {
    conn: Mutex<Connection>,
}

/// A single change returned by `get_changes_since`.
#[derive(Debug)]
pub struct ChangeEntry {
    pub id: String,
    pub data: Value,
    pub updated_at: String,
    pub deleted: bool,
}

/// Status information returned by `get_status`.
#[derive(Debug)]
pub struct StatusInfo {
    pub total_tasks: i64,
    pub last_modified: Option<String>,
    pub devices: Vec<DeviceInfo>,
}

/// A device record from the devices table.
#[derive(Debug)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_sync: String,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    data TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted INTEGER DEFAULT 0,
    updated_by TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    data TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted INTEGER DEFAULT 0,
    updated_by TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    last_sync TEXT NOT NULL
);
";

impl SyncDb {
    /// Open (or create) the database at `path` and initialize the schema.
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(SyncDb {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(SyncDb {
            conn: Mutex::new(conn),
        })
    }

    /// Upsert a task. Returns `Some(old_updated_at)` if the server version is
    /// newer than the client version (conflict), or `None` if the upsert
    /// succeeded.
    pub fn upsert_task(
        &self,
        id: &str,
        data_json: &str,
        updated_at: &str,
        device_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        // Check existing row
        let existing: Option<(String, i64)> = conn
            .query_row(
                "SELECT updated_at, deleted FROM tasks WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((server_updated_at, _deleted)) = &existing {
            if updated_at < server_updated_at.as_str() {
                // Server is newer — conflict
                return Ok(Some(server_updated_at.clone()));
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO tasks (id, data, updated_at, deleted, updated_by) VALUES (?1, ?2, ?3, 0, ?4)",
            params![id, data_json, updated_at, device_id],
        )?;
        Ok(None)
    }

    /// Soft-delete a task by id. Returns `Some(old_updated_at)` on conflict.
    pub fn soft_delete_task(
        &self,
        id: &str,
        updated_at: &str,
        device_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<String> = conn
            .query_row(
                "SELECT updated_at FROM tasks WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok();

        if let Some(server_updated_at) = &existing {
            if updated_at < server_updated_at.as_str() {
                return Ok(Some(server_updated_at.clone()));
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO tasks (id, data, updated_at, deleted, updated_by) VALUES (?1, '{}', ?2, 1, ?3)",
            params![id, updated_at, device_id],
        )?;
        Ok(None)
    }

    /// Upsert a project. Returns `Some(old_updated_at)` on conflict.
    pub fn upsert_project(
        &self,
        id: &str,
        data_json: &str,
        updated_at: &str,
        device_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<(String, i64)> = conn
            .query_row(
                "SELECT updated_at, deleted FROM projects WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((server_updated_at, _deleted)) = &existing {
            if updated_at < server_updated_at.as_str() {
                return Ok(Some(server_updated_at.clone()));
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO projects (id, data, updated_at, deleted, updated_by) VALUES (?1, ?2, ?3, 0, ?4)",
            params![id, data_json, updated_at, device_id],
        )?;
        Ok(None)
    }

    /// Soft-delete a project by id. Returns `Some(old_updated_at)` on conflict.
    pub fn soft_delete_project(
        &self,
        id: &str,
        updated_at: &str,
        device_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<String> = conn
            .query_row(
                "SELECT updated_at FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok();

        if let Some(server_updated_at) = &existing {
            if updated_at < server_updated_at.as_str() {
                return Ok(Some(server_updated_at.clone()));
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO projects (id, data, updated_at, deleted, updated_by) VALUES (?1, '{}', ?2, 1, ?3)",
            params![id, updated_at, device_id],
        )?;
        Ok(None)
    }

    /// Get all task and project changes since a given timestamp.
    /// Returns active entries (deleted=0) with full data, and deleted entries
    /// (deleted=1) with just the id.
    pub fn get_changes_since(&self, since_timestamp: &str) -> Result<Vec<ChangeEntry>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut entries = Vec::new();

        // Active tasks
        {
            let mut stmt = conn.prepare(
                "SELECT id, data, updated_at FROM tasks WHERE updated_at > ?1 AND deleted = 0",
            )?;
            let rows = stmt.query_map(params![since_timestamp], |row| {
                let id: String = row.get(0)?;
                let data: String = row.get(1)?;
                let updated_at: String = row.get(2)?;
                Ok(ChangeEntry {
                    id,
                    data: serde_json::from_str(&data).unwrap_or(Value::Null),
                    updated_at,
                    deleted: false,
                })
            })?;
            for row in rows {
                entries.push(row?);
            }
        }

        // Deleted tasks
        {
            let mut stmt = conn.prepare(
                "SELECT id, updated_at FROM tasks WHERE updated_at > ?1 AND deleted = 1",
            )?;
            let rows = stmt.query_map(params![since_timestamp], |row| {
                let id: String = row.get(0)?;
                let updated_at: String = row.get(1)?;
                Ok(ChangeEntry {
                    id,
                    data: Value::Null,
                    updated_at,
                    deleted: true,
                })
            })?;
            for row in rows {
                entries.push(row?);
            }
        }

        // Active projects
        {
            let mut stmt = conn.prepare(
                "SELECT id, data, updated_at FROM projects WHERE updated_at > ?1 AND deleted = 0",
            )?;
            let rows = stmt.query_map(params![since_timestamp], |row| {
                let id: String = row.get(0)?;
                let data: String = row.get(1)?;
                let updated_at: String = row.get(2)?;
                Ok(ChangeEntry {
                    id,
                    data: serde_json::from_str(&data).unwrap_or(Value::Null),
                    updated_at,
                    deleted: false,
                })
            })?;
            for row in rows {
                entries.push(row?);
            }
        }

        // Deleted projects
        {
            let mut stmt = conn.prepare(
                "SELECT id, updated_at FROM projects WHERE updated_at > ?1 AND deleted = 1",
            )?;
            let rows = stmt.query_map(params![since_timestamp], |row| {
                let id: String = row.get(0)?;
                let updated_at: String = row.get(1)?;
                Ok(ChangeEntry {
                    id,
                    data: Value::Null,
                    updated_at,
                    deleted: true,
                })
            })?;
            for row in rows {
                entries.push(row?);
            }
        }

        Ok(entries)
    }

    /// Record (or update) the last sync timestamp for a device.
    pub fn record_device_sync(
        &self,
        device_id: &str,
        timestamp: &str,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO devices (device_id, last_sync) VALUES (?1, ?2)",
            params![device_id, timestamp],
        )?;
        Ok(())
    }

    /// Get server status: total active tasks, last modified timestamp, device list.
    pub fn get_status(&self) -> Result<StatusInfo, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();

        let total_tasks: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE deleted = 0",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let last_modified: Option<String> = conn
            .query_row(
                "SELECT MAX(updated_at) FROM (SELECT updated_at FROM tasks UNION ALL SELECT updated_at FROM projects)",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let mut devices = Vec::new();
        {
            let mut stmt = conn.prepare("SELECT device_id, last_sync FROM devices")?;
            let rows = stmt.query_map([], |row| {
                Ok(DeviceInfo {
                    device_id: row.get(0)?,
                    last_sync: row.get(1)?,
                })
            })?;
            for row in rows {
                devices.push(row?);
            }
        }

        Ok(StatusInfo {
            total_tasks,
            last_modified,
            devices,
        })
    }

    /// Get a single task's data by id. Used for conflict resolution.
    pub fn get_task_data(&self, id: &str) -> Result<Option<(Value, String)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT data, updated_at FROM tasks WHERE id = ?1",
                params![id],
                |row| {
                    let data: String = row.get(0)?;
                    let updated_at: String = row.get(1)?;
                    Ok((
                        serde_json::from_str(&data).unwrap_or(Value::Null),
                        updated_at,
                    ))
                },
            )
            .ok();
        Ok(result)
    }

    /// Get a single project's data by id. Used for conflict resolution.
    pub fn get_project_data(&self, id: &str) -> Result<Option<(Value, String)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT data, updated_at FROM projects WHERE id = ?1",
                params![id],
                |row| {
                    let data: String = row.get(0)?;
                    let updated_at: String = row.get(1)?;
                    Ok((
                        serde_json::from_str(&data).unwrap_or(Value::Null),
                        updated_at,
                    ))
                },
            )
            .ok();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_creates_tables() {
        let db = SyncDb::open_in_memory().unwrap();
        let status = db.get_status().unwrap();
        assert_eq!(status.total_tasks, 0);
        assert!(status.last_modified.is_none());
        assert!(status.devices.is_empty());
    }

    #[test]
    fn upsert_task_inserts_new() {
        let db = SyncDb::open_in_memory().unwrap();
        let result = db.upsert_task("t1", r#"{"id":"t1"}"#, "2026-01-01T00:00:00Z", "dev1");
        assert!(result.unwrap().is_none());

        let data = db.get_task_data("t1").unwrap().unwrap();
        assert_eq!(data.0["id"], "t1");
    }

    #[test]
    fn upsert_task_updates_when_newer() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_task("t1", r#"{"id":"t1","v":1}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        let conflict = db.upsert_task("t1", r#"{"id":"t1","v":2}"#, "2026-01-02T00:00:00Z", "dev2").unwrap();
        assert!(conflict.is_none());

        let data = db.get_task_data("t1").unwrap().unwrap();
        assert_eq!(data.0["v"], 2);
    }

    #[test]
    fn upsert_task_conflict_when_older() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_task("t1", r#"{"id":"t1","v":2}"#, "2026-01-02T00:00:00Z", "dev1").unwrap();
        let conflict = db.upsert_task("t1", r#"{"id":"t1","v":1}"#, "2026-01-01T00:00:00Z", "dev2").unwrap();
        assert_eq!(conflict, Some("2026-01-02T00:00:00Z".to_string()));

        // Server version unchanged
        let data = db.get_task_data("t1").unwrap().unwrap();
        assert_eq!(data.0["v"], 2);
    }

    #[test]
    fn soft_delete_task() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_task("t1", r#"{"id":"t1"}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        let conflict = db.soft_delete_task("t1", "2026-01-02T00:00:00Z", "dev1").unwrap();
        assert!(conflict.is_none());

        let status = db.get_status().unwrap();
        assert_eq!(status.total_tasks, 0);
    }

    #[test]
    fn soft_delete_task_conflict() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_task("t1", r#"{"id":"t1"}"#, "2026-01-02T00:00:00Z", "dev1").unwrap();
        let conflict = db.soft_delete_task("t1", "2026-01-01T00:00:00Z", "dev1").unwrap();
        assert_eq!(conflict, Some("2026-01-02T00:00:00Z".to_string()));
    }

    #[test]
    fn upsert_project_works() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_project("p1", r#"{"id":"p1","name":"test"}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        let data = db.get_project_data("p1").unwrap().unwrap();
        assert_eq!(data.0["name"], "test");
    }

    #[test]
    fn upsert_project_conflict() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_project("p1", r#"{"id":"p1"}"#, "2026-01-02T00:00:00Z", "dev1").unwrap();
        let conflict = db.upsert_project("p1", r#"{"id":"p1"}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        assert_eq!(conflict, Some("2026-01-02T00:00:00Z".to_string()));
    }

    #[test]
    fn soft_delete_project() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_project("p1", r#"{"id":"p1"}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        let conflict = db.soft_delete_project("p1", "2026-01-02T00:00:00Z", "dev1").unwrap();
        assert!(conflict.is_none());
    }

    #[test]
    fn get_changes_since_returns_active_and_deleted() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_task("t1", r#"{"id":"t1"}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        db.upsert_task("t2", r#"{"id":"t2"}"#, "2026-01-02T00:00:00Z", "dev1").unwrap();
        db.soft_delete_task("t2", "2026-01-03T00:00:00Z", "dev1").unwrap();

        let changes = db.get_changes_since("2025-12-31T00:00:00Z").unwrap();

        let active_tasks: Vec<_> = changes
            .iter()
            .filter(|c| !c.deleted && c.data != Value::Null)
            .collect();
        assert_eq!(active_tasks.len(), 1); // only t1 (t2 is deleted)
        assert_eq!(active_tasks[0].id, "t1");

        let deleted: Vec<_> = changes.iter().filter(|c| c.deleted).collect();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, "t2");
    }

    #[test]
    fn record_and_get_device_sync() {
        let db = SyncDb::open_in_memory().unwrap();
        db.record_device_sync("device-a", "2026-01-01T00:00:00Z").unwrap();

        let status = db.get_status().unwrap();
        assert_eq!(status.devices.len(), 1);
        assert_eq!(status.devices[0].device_id, "device-a");
        assert_eq!(status.devices[0].last_sync, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn get_status_counts_active_tasks() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_task("t1", r#"{"id":"t1"}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        db.upsert_task("t2", r#"{"id":"t2"}"#, "2026-01-02T00:00:00Z", "dev1").unwrap();
        db.soft_delete_task("t2", "2026-01-03T00:00:00Z", "dev1").unwrap();

        let status = db.get_status().unwrap();
        assert_eq!(status.total_tasks, 1);
        assert_eq!(status.last_modified, Some("2026-01-03T00:00:00Z".to_string()));
    }

    #[test]
    fn upsert_task_same_timestamp_overwrites() {
        let db = SyncDb::open_in_memory().unwrap();
        db.upsert_task("t1", r#"{"id":"t1","v":1}"#, "2026-01-01T00:00:00Z", "dev1").unwrap();
        // Same timestamp should still overwrite (>= check)
        let conflict = db.upsert_task("t1", r#"{"id":"t1","v":2}"#, "2026-01-01T00:00:00Z", "dev2").unwrap();
        assert!(conflict.is_none());

        let data = db.get_task_data("t1").unwrap().unwrap();
        assert_eq!(data.0["v"], 2);
    }
}
