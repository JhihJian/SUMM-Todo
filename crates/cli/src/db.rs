use std::env;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};

use crate::error::TodoError;
use crate::task::{Creator, Priority, Project, Status, Task};

// ---------------------------------------------------------------------------
// ProjectStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectStats {
    pub total: i64,
    pub pending: i64,
    pub in_progress: i64,
    pub blocked: i64,
    pub done: i64,
    pub cancelled: i64,
}

// ---------------------------------------------------------------------------
// TaskFilter
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct TaskFilter {
    pub status: Option<Vec<Status>>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<Priority>,
    pub parent_id: Option<String>,
    pub creator: Option<Creator>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub sort: Option<String>,
    pub overdue: bool,
    pub project_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

pub struct Database {
    pub(crate) conn: Connection,
}

// Column list for tasks (used in SELECT, INSERT, UPDATE)
const TASK_COLUMNS: &str = "id, title, creator, created_at, \
    priority, tags, parent_id, due, \
    status, assignee, blocked_reason, \
    result, artifacts, log, \
    started_at, finished_at, content, project_id, updated_at";

const PROJECT_COLUMNS: &str = "id, name, description, path, created_at, updated_at";

impl Database {
    /// Open (or create) the database at the default path and run migrations.
    pub fn open() -> Result<Self, TodoError> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        let mut db = Self { conn };
        db.init()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self, TodoError> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.init()?;
        Ok(db)
    }

    /// Resolve the database file path.
    /// Checks `TODO_DB_PATH` env var first, then falls back to `~/.todo/todo.db`.
    fn default_path() -> Result<PathBuf, TodoError> {
        if let Ok(p) = env::var("TODO_DB_PATH") {
            return Ok(PathBuf::from(p));
        }
        let home = dirs::home_dir().ok_or_else(|| {
            TodoError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not determine home directory",
            ))
        })?;
        Ok(home.join(".todo").join("todo.db"))
    }

    /// Set pragmas and run migrations.
    fn init(&mut self) -> Result<(), TodoError> {
        self.conn
            .execute_batch("PRAGMA journal_mode = WAL;")?;
        self.run_migrations()?;
        Ok(())
    }

    /// Run pending migrations based on PRAGMA user_version.
    fn run_migrations(&mut self) -> Result<(), TodoError> {
        let version: i32 =
            self.conn
                .query_row("PRAGMA user_version;", [], |row| row.get(0))?;

        if version < 1 {
            let sql = include_str!("../../../migrations/v1.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute_batch("PRAGMA user_version = 1;")?;
        }

        if version < 2 {
            let sql = include_str!("../../../migrations/v2.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute_batch("PRAGMA user_version = 2;")?;
        }

        if version < 3 {
            let sql = include_str!("../../../migrations/v3.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute_batch("PRAGMA user_version = 3;")?;
        }

        if version < 4 {
            let sql = include_str!("../../../migrations/v4.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute_batch("PRAGMA user_version = 4;")?;
        }

        if version < 5 {
            let sql = include_str!("../../../migrations/v5.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute_batch("PRAGMA user_version = 5;")?;
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // CRUD operations
    // -----------------------------------------------------------------------

    /// Insert a new task.
    pub fn insert_task(&self, task: &Task) -> Result<(), TodoError> {
        self.conn.execute(
            &format!("INSERT INTO tasks ({TASK_COLUMNS}) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8,
                ?9, ?10, ?11,
                ?12, ?13, ?14,
                ?15, ?16, ?17, ?18, ?19
            )"),
            params![
                task.id,
                task.title,
                task.creator.to_string(),
                task.created_at.to_rfc3339(),
                task.priority.to_string(),
                serde_json::to_string(&task.tags).unwrap_or_else(|_| "[]".into()),
                task.parent_id,
                task.due.map(|d| d.to_rfc3339()),
                task.status.to_string(),
                task.assignee.as_ref().map(|a| a.to_string()),
                task.blocked_reason,
                task.result,
                serde_json::to_string(&task.artifacts).unwrap_or_else(|_| "[]".into()),
                task.log,
                task.started_at.map(|d| d.to_rfc3339()),
                task.finished_at.map(|d| d.to_rfc3339()),
                task.content,
                task.project_id,
                task.updated_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Fetch a single task by ID.
    pub fn get_task(&self, id: &str) -> Result<Option<Task>, TodoError> {
        let mut stmt = self.conn.prepare(
            &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?1"),
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_task(row)?)),
            None => Ok(None),
        }
    }

    /// Update an existing task (full replacement).
    pub fn update_task(&self, task: &Task) -> Result<(), TodoError> {
        self.conn.execute(
            "UPDATE tasks SET
                title = ?2, creator = ?3, created_at = ?4,
                priority = ?5, tags = ?6, parent_id = ?7, due = ?8,
                status = ?9, assignee = ?10, blocked_reason = ?11,
                result = ?12, artifacts = ?13, log = ?14,
                started_at = ?15, finished_at = ?16, content = ?17, project_id = ?18,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?1",
            params![
                task.id,
                task.title,
                task.creator.to_string(),
                task.created_at.to_rfc3339(),
                task.priority.to_string(),
                serde_json::to_string(&task.tags).unwrap_or_else(|_| "[]".into()),
                task.parent_id,
                task.due.map(|d| d.to_rfc3339()),
                task.status.to_string(),
                task.assignee.as_ref().map(|a| a.to_string()),
                task.blocked_reason,
                task.result,
                serde_json::to_string(&task.artifacts).unwrap_or_else(|_| "[]".into()),
                task.log,
                task.started_at.map(|d| d.to_rfc3339()),
                task.finished_at.map(|d| d.to_rfc3339()),
                task.content,
                task.project_id,
            ],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Project operations
    // -----------------------------------------------------------------------

    /// Insert a new project.
    pub fn insert_project(&self, project: &Project) -> Result<(), TodoError> {
        self.conn.execute(
            &format!("INSERT INTO projects ({PROJECT_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
            params![
                project.id,
                project.name,
                project.description,
                project.path,
                project.created_at.to_rfc3339(),
                project.updated_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Fetch a project by ID.
    pub fn get_project(&self, id: &str) -> Result<Option<Project>, TodoError> {
        let mut stmt = self.conn.prepare(
            &format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id = ?1"),
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_project(row)?)),
            None => Ok(None),
        }
    }

    /// Fetch a project by name.
    pub fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, TodoError> {
        let mut stmt = self.conn.prepare(
            &format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE name = ?1"),
        )?;
        let mut rows = stmt.query(params![name])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_project(row)?)),
            None => Ok(None),
        }
    }

    /// List all projects.
    pub fn list_projects(&self) -> Result<Vec<Project>, TodoError> {
        let mut stmt = self.conn.prepare(
            &format!("SELECT {PROJECT_COLUMNS} FROM projects ORDER BY name ASC"),
        )?;
        let mut rows = stmt.query([])?;
        let mut projects = Vec::new();
        while let Some(row) = rows.next()? {
            projects.push(row_to_project(row)?);
        }
        Ok(projects)
    }

    /// Update a project.
    pub fn update_project(&self, project: &Project) -> Result<(), TodoError> {
        self.conn.execute(
            "UPDATE projects SET name = ?2, description = ?3, path = ?4, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?1",
            params![project.id, project.name, project.description, project.path,],
        )?;
        Ok(())
    }

    /// Delete a project. Returns error if project has tasks.
    pub fn delete_project(&self, id: &str) -> Result<(), TodoError> {
        // Check if project has tasks
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Err(TodoError::ProjectHasTasks(count));
        }
        self.conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Get task statistics for a project.
    pub fn get_project_stats(&self, project_id: &str) -> Result<ProjectStats, TodoError> {
        let mut stats = ProjectStats::default();

        stats.total = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )?;

        stats.pending = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'pending'",
            params![project_id],
            |row| row.get(0),
        )?;

        stats.in_progress = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'in_progress'",
            params![project_id],
            |row| row.get(0),
        )?;

        stats.blocked = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'blocked'",
            params![project_id],
            |row| row.get(0),
        )?;

        stats.done = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'done'",
            params![project_id],
            |row| row.get(0),
        )?;

        stats.cancelled = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'cancelled'",
            params![project_id],
            |row| row.get(0),
        )?;

        Ok(stats)
    }

    /// Get recent tasks for a project.
    pub fn get_project_recent_tasks(&self, project_id: &str, limit: i64) -> Result<Vec<Task>, TodoError> {
        let filter = TaskFilter {
            status: None,
            tags: None,
            priority: None,
            parent_id: None,
            creator: None,
            since: None,
            limit: Some(limit),
            sort: Some("created_at DESC".into()),
            overdue: false,
            project_id: Some(project_id.to_string()),
        };
        self.list_tasks(&filter)
    }

    /// List tasks with optional filters.
    pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>, TodoError> {
        let mut conditions: Vec<String> = vec!["1=1".into()];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1u32;

        // status IN (...)
        if let Some(ref statuses) = filter.status {
            if !statuses.is_empty() {
                let placeholders: Vec<String> = statuses
                    .iter()
                    .map(|s| {
                        let ph = format!("?{}", param_idx);
                        param_idx += 1;
                        param_values.push(Box::new(s.to_string()));
                        ph
                    })
                    .collect();
                conditions.push(format!("status IN ({})", placeholders.join(", ")));
            }
        }

        // tags LIKE for each tag (AND logic)
        if let Some(ref tags) = filter.tags {
            for tag in tags {
                conditions.push(format!("tags LIKE ?{}", param_idx));
                param_idx += 1;
                param_values.push(Box::new(format!("%\"{}\"%" , tag)));
            }
        }

        // priority
        if let Some(ref priority) = filter.priority {
            conditions.push(format!("priority = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(priority.to_string()));
        }

        // parent_id
        if let Some(ref parent_id) = filter.parent_id {
            conditions.push(format!("parent_id = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(parent_id.clone()));
        }

        // creator
        if let Some(ref creator) = filter.creator {
            conditions.push(format!("creator = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(creator.to_string()));
        }

        // since
        if let Some(ref since) = filter.since {
            conditions.push(format!("created_at >= ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(since.to_rfc3339()));
        }

        // overdue
        if filter.overdue {
            conditions.push(format!("due IS NOT NULL AND due < ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(Utc::now().to_rfc3339()));
        }

        // project_id
        if let Some(ref project_id) = filter.project_id {
            conditions.push(format!("project_id = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(project_id.clone()));
        }

        // sort
        let order = filter.sort.clone().unwrap_or_else(|| {
            "CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 WHEN 'low' THEN 2 END ASC, created_at ASC".into()
        });

        let limit = filter.limit.unwrap_or(20);

        let sql = format!(
            "SELECT {TASK_COLUMNS}
             FROM tasks
             WHERE {}
             ORDER BY {}
             LIMIT {}",
            conditions.join(" AND "),
            order,
            limit,
        );

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params_refs.as_slice())?;
        let mut tasks = Vec::new();
        while let Some(row) = rows.next()? {
            tasks.push(row_to_task(row)?);
        }
        Ok(tasks)
    }

    /// Get the next pending task, optionally filtered by tag and/or priority.
    pub fn get_next_task(
        &self,
        tag: Option<&str>,
        priority: Option<&str>,
    ) -> Result<Option<Task>, TodoError> {
        let mut filter = TaskFilter {
            status: Some(vec![Status::Pending]),
            limit: Some(1),
            ..Default::default()
        };

        if let Some(t) = tag {
            filter.tags = Some(vec![t.to_string()]);
        }

        if let Some(p) = priority {
            filter.priority = Some(p.parse::<Priority>()?);
        }

        let mut tasks = self.list_tasks(&filter)?;
        Ok(tasks.pop())
    }

    /// Search tasks by title (and optionally result/log), with optional regex support.
    pub fn search_tasks(&self, query: &str, use_regex: bool, filter: &TaskFilter) -> Result<Vec<Task>, TodoError> {
        let mut conditions: Vec<String> = vec!["1=1".into()];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1u32;

        // Search condition for title
        if use_regex {
            conditions.push(format!("title REGEXP ?{}", param_idx));
        } else {
            conditions.push(format!("title LIKE ?{}", param_idx));
        }
        param_idx += 1;

        let pattern = if use_regex {
            query.to_string()
        } else {
            format!("%{}%", query)
        };
        param_values.push(Box::new(pattern));

        // status IN (...)
        if let Some(ref statuses) = filter.status {
            if !statuses.is_empty() {
                let placeholders: Vec<String> = statuses
                    .iter()
                    .map(|s| {
                        let ph = format!("?{}", param_idx);
                        param_idx += 1;
                        param_values.push(Box::new(s.to_string()));
                        ph
                    })
                    .collect();
                conditions.push(format!("status IN ({})", placeholders.join(", ")));
            }
        }

        // tags LIKE for each tag (AND logic)
        if let Some(ref tags) = filter.tags {
            for tag in tags {
                conditions.push(format!("tags LIKE ?{}", param_idx));
                param_idx += 1;
                param_values.push(Box::new(format!("%\"{}\"%" , tag)));
            }
        }

        // priority
        if let Some(ref priority) = filter.priority {
            conditions.push(format!("priority = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(priority.to_string()));
        }

        // parent_id
        if let Some(ref parent_id) = filter.parent_id {
            conditions.push(format!("parent_id = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(parent_id.clone()));
        }

        // creator
        if let Some(ref creator) = filter.creator {
            conditions.push(format!("creator = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(creator.to_string()));
        }

        // since
        if let Some(ref since) = filter.since {
            conditions.push(format!("created_at >= ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(since.to_rfc3339()));
        }

        // overdue
        if filter.overdue {
            conditions.push(format!("due IS NOT NULL AND due < ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(Utc::now().to_rfc3339()));
        }

        // project_id
        if let Some(ref project_id) = filter.project_id {
            conditions.push(format!("project_id = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(project_id.clone()));
        }

        // sort
        let order = filter.sort.clone().unwrap_or_else(|| {
            "CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 WHEN 'low' THEN 2 END ASC, created_at ASC".into()
        });

        let limit = filter.limit.unwrap_or(20);

        let sql = format!(
            "SELECT {TASK_COLUMNS}
             FROM tasks
             WHERE {}
             ORDER BY {}
             LIMIT {}",
            conditions.join(" AND "),
            order,
            limit,
        );

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params_refs.as_slice())?;
        let mut tasks = Vec::new();
        while let Some(row) = rows.next()? {
            tasks.push(row_to_task(row)?);
        }
        Ok(tasks)
    }

    // -----------------------------------------------------------------------
    // Sync config operations
    // -----------------------------------------------------------------------

    /// Get a sync config value by key.
    pub fn get_sync_config(&self, key: &str) -> Result<Option<String>, TodoError> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM sync_config WHERE key = ?1",
        )?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// Set a sync config value (insert or replace).
    pub fn set_sync_config(&self, key: &str, value: &str) -> Result<(), TodoError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sync_config (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Clear all sync_log entries (after successful push).
    pub fn clear_sync_log(&self) -> Result<(), TodoError> {
        self.conn.execute("DELETE FROM sync_log", [])?;
        Ok(())
    }

    /// Get pending sync changes from sync_log, resolving to latest action per entity.
    /// Returns (upserted_tasks, deleted_task_ids, upserted_projects, deleted_project_ids).
    pub fn get_sync_pending(&self) -> Result<(Vec<Task>, Vec<String>, Vec<Project>, Vec<String>), TodoError> {
        // Resolve latest action per (entity_type, entity_id)
        let mut task_actions: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut project_actions: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        let mut stmt = self.conn.prepare(
            "SELECT entity_type, entity_id, action FROM sync_log ORDER BY recorded_at ASC",
        )?;
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let entity_type: String = row.get(0)?;
            let entity_id: String = row.get(1)?;
            let action: String = row.get(2)?;

            match entity_type.as_str() {
                "task" => { task_actions.insert(entity_id, action); }
                "project" => { project_actions.insert(entity_id, action); }
                _ => {}
            }
        }

        let mut tasks = Vec::new();
        let mut deleted_task_ids = Vec::new();

        for (id, action) in &task_actions {
            match action.as_str() {
                "upsert" => {
                    if let Some(task) = self.get_task(id)? {
                        tasks.push(task);
                    }
                }
                "delete" => {
                    deleted_task_ids.push(id.clone());
                }
                _ => {}
            }
        }

        let mut projects = Vec::new();
        let mut deleted_project_ids = Vec::new();

        for (id, action) in &project_actions {
            match action.as_str() {
                "upsert" => {
                    if let Some(project) = self.get_project(id)? {
                        projects.push(project);
                    }
                }
                "delete" => {
                    deleted_project_ids.push(id.clone());
                }
                _ => {}
            }
        }

        Ok((tasks, deleted_task_ids, projects, deleted_project_ids))
    }
}

// ---------------------------------------------------------------------------
// Row → Task helper
// ---------------------------------------------------------------------------

fn row_to_task(row: &Row<'_>) -> Result<Task, TodoError> {
    let creator_str: String = row.get(2)?;
    let created_at_str: String = row.get(3)?;
    let priority_str: String = row.get(4)?;
    let tags_str: String = row.get(5)?;
    let status_str: String = row.get(8)?;
    let assignee_str: Option<String> = row.get(9)?;
    let artifacts_str: String = row.get(12)?;

    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(16)?,
        project_id: row.get(17)?,
        creator: creator_str.parse::<Creator>()?,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| TodoError::ParseError(e.to_string()))?
            .with_timezone(&Utc),
        priority: priority_str.parse::<Priority>()?,
        tags: serde_json::from_str(&tags_str)
            .map_err(|e| TodoError::ParseError(e.to_string()))?,
        parent_id: row.get(6)?,
        due: parse_optional_datetime(row.get::<_, Option<String>>(7)?)?,
        status: status_str.parse::<Status>()?,
        assignee: match assignee_str {
            Some(s) => Some(s.parse::<Creator>()?),
            None => None,
        },
        blocked_reason: row.get(10)?,
        result: row.get(11)?,
        artifacts: serde_json::from_str(&artifacts_str)
            .map_err(|e| TodoError::ParseError(e.to_string()))?,
        log: row.get(13)?,
        started_at: parse_optional_datetime(row.get::<_, Option<String>>(14)?)?,
        finished_at: parse_optional_datetime(row.get::<_, Option<String>>(15)?)?,
        updated_at: parse_optional_datetime(row.get::<_, Option<String>>(18)?)?,
    })
}

fn parse_optional_datetime(
    s: Option<String>,
) -> Result<Option<DateTime<Utc>>, TodoError> {
    match s {
        None => Ok(None),
        Some(v) => {
            let dt = DateTime::parse_from_rfc3339(&v)
                .map_err(|e| TodoError::ParseError(e.to_string()))?;
            Ok(Some(dt.with_timezone(&Utc)))
        }
    }
}

fn row_to_project(row: &Row<'_>) -> Result<Project, TodoError> {
    let created_at_str: String = row.get(4)?;
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        path: row.get(3)?,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| TodoError::ParseError(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: parse_optional_datetime(row.get::<_, Option<String>>(5)?)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Creator, Priority, Status, Task};
    use chrono::Utc;

    fn test_db() -> Database {
        Database::open_in_memory().expect("failed to open in-memory db")
    }

    #[test]
    fn database_initializes_with_schema() {
        let db = test_db();
        db.conn
            .execute(
                "INSERT INTO tasks (id, title, updated_at) VALUES ('test1', 'hello', datetime('now'))",
                [],
            )
            .expect("insert should work after migration");
    }

    #[test]
    fn schema_version_is_set() {
        let db = test_db();
        let version: i32 = db
            .conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 5);
    }

    #[test]
    fn insert_and_get_task() {
        let db = test_db();
        let mut task = Task::new("abc1", "Write tests");
        task.creator = Creator::Agent;
        task.priority = Priority::High;
        task.tags = vec!["backend".into(), "urgent".into()];

        db.insert_task(&task).unwrap();

        let fetched = db.get_task("abc1").unwrap().expect("task should exist");
        assert_eq!(fetched.id, "abc1");
        assert_eq!(fetched.title, "Write tests");
        assert_eq!(fetched.creator, Creator::Agent);
        assert_eq!(fetched.priority, Priority::High);
        assert_eq!(fetched.tags, vec!["backend".to_string(), "urgent".to_string()]);
        assert_eq!(fetched.status, Status::Pending);
        assert!(fetched.updated_at.is_some());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = test_db();
        let result = db.get_task("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_task() {
        let db = test_db();
        let mut task = Task::new("upd1", "Original title");
        db.insert_task(&task).unwrap();

        task.status = Status::InProgress;
        task.assignee = Some(Creator::Agent);
        task.started_at = Some(Utc::now());
        db.update_task(&task).unwrap();

        let fetched = db.get_task("upd1").unwrap().expect("task should exist");
        assert_eq!(fetched.status, Status::InProgress);
        assert_eq!(fetched.assignee, Some(Creator::Agent));
        assert!(fetched.started_at.is_some());
        assert!(fetched.updated_at.is_some());
    }

    #[test]
    fn list_filters_by_status() {
        let db = test_db();

        let mut t1 = Task::new("s1", "Pending task");
        t1.status = Status::Pending;
        db.insert_task(&t1).unwrap();

        let mut t2 = Task::new("s2", "Done task");
        t2.status = Status::Done;
        t2.result = Some("finished".into());
        t2.finished_at = Some(Utc::now());
        db.insert_task(&t2).unwrap();

        let filter = TaskFilter {
            status: Some(vec![Status::Pending]),
            ..Default::default()
        };
        let results = db.list_tasks(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s1");

        let filter2 = TaskFilter {
            status: Some(vec![Status::Done]),
            ..Default::default()
        };
        let results2 = db.list_tasks(&filter2).unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].id, "s2");
    }

    #[test]
    fn list_filters_by_tag() {
        let db = test_db();

        let mut t1 = Task::new("t1", "Backend task");
        t1.tags = vec!["backend".into()];
        db.insert_task(&t1).unwrap();

        let mut t2 = Task::new("t2", "Frontend task");
        t2.tags = vec!["frontend".into()];
        db.insert_task(&t2).unwrap();

        let filter = TaskFilter {
            tags: Some(vec!["backend".into()]),
            ..Default::default()
        };
        let results = db.list_tasks(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "t1");

        let filter2 = TaskFilter {
            tags: Some(vec!["frontend".into()]),
            ..Default::default()
        };
        let results2 = db.list_tasks(&filter2).unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].id, "t2");
    }

    #[test]
    fn get_next_returns_highest_priority_oldest_first() {
        let db = test_db();

        let mut t_low = Task::new("lo1", "Low priority");
        t_low.priority = Priority::Low;
        t_low.created_at = Utc::now() - chrono::Duration::hours(2);
        db.insert_task(&t_low).unwrap();

        let mut t_high = Task::new("hi1", "High priority");
        t_high.priority = Priority::High;
        t_high.created_at = Utc::now() - chrono::Duration::hours(1);
        db.insert_task(&t_high).unwrap();

        let next = db.get_next_task(None, None).unwrap().expect("should have a task");
        assert_eq!(next.id, "hi1", "high priority should come first");
    }

    #[test]
    fn get_next_returns_none_when_empty() {
        let db = test_db();
        let next = db.get_next_task(None, None).unwrap();
        assert!(next.is_none());
    }

    #[test]
    fn insert_and_get_project() {
        let db = test_db();
        let project = Project::new("proj1", "Test Project");

        db.insert_project(&project).unwrap();

        let fetched = db.get_project("proj1").unwrap().expect("project should exist");
        assert_eq!(fetched.id, "proj1");
        assert_eq!(fetched.name, "Test Project");
        assert!(fetched.updated_at.is_some());
    }

    #[test]
    fn get_project_by_name() {
        let db = test_db();
        let project = Project::new("p1", "My Project");
        db.insert_project(&project).unwrap();

        let fetched = db.get_project_by_name("My Project").unwrap().expect("should exist");
        assert_eq!(fetched.id, "p1");
    }

    #[test]
    fn delete_project_with_tasks_fails() {
        let db = test_db();

        let project = Project::new("p1", "Test");
        db.insert_project(&project).unwrap();

        let mut task = Task::new("t1", "Task");
        task.project_id = Some("p1".to_string());
        db.insert_task(&task).unwrap();

        let result = db.delete_project("p1");
        assert!(result.is_err());
    }

    #[test]
    fn project_stats() {
        let db = test_db();

        let project = Project::new("p1", "Test");
        db.insert_project(&project).unwrap();

        let mut t1 = Task::new("t1", "Task 1");
        t1.project_id = Some("p1".to_string());
        t1.status = Status::Pending;
        db.insert_task(&t1).unwrap();

        let mut t2 = Task::new("t2", "Task 2");
        t2.project_id = Some("p1".to_string());
        t2.status = Status::Done;
        t2.result = Some("done".into());
        t2.finished_at = Some(Utc::now());
        db.insert_task(&t2).unwrap();

        let stats = db.get_project_stats("p1").unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.done, 1);
    }

    #[test]
    fn sync_config_round_trip() {
        let db = test_db();
        assert!(db.get_sync_config("server_url").unwrap().is_none());
        db.set_sync_config("server_url", "http://localhost:3000").unwrap();
        assert_eq!(db.get_sync_config("server_url").unwrap(), Some("http://localhost:3000".to_string()));

        // Update
        db.set_sync_config("server_url", "http://new:3000").unwrap();
        assert_eq!(db.get_sync_config("server_url").unwrap(), Some("http://new:3000".to_string()));
    }

    #[test]
    fn sync_pending_detects_insert() {
        let db = test_db();
        let task = Task::new("abc1", "Sync test");
        db.insert_task(&task).unwrap();

        let (tasks, deleted, projects, deleted_projects) = db.get_sync_pending().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "abc1");
        assert!(deleted.is_empty());
        assert!(projects.is_empty());
        assert!(deleted_projects.is_empty());
    }

    #[test]
    fn sync_pending_resolves_latest_action() {
        let db = test_db();
        let mut task = Task::new("abc1", "First");
        db.insert_task(&task).unwrap();

        // Update — should still be 'upsert'
        task.title = "Second".to_string();
        db.update_task(&task).unwrap();

        let (tasks, _, _, _) = db.get_sync_pending().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Second");
    }
}
