# Project Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use summ:executing-plans to implement this plan task-by-task.

**Goal:** Add project support to SUMM-Todo, allowing tasks to be organized under projects with full CRUD commands.

**Architecture:** Add `Project` struct and `projects` table, extend `Task` with `project_id`, implement `project` subcommand group and modify `add`/`list`/`edit` commands.

**Tech Stack:** Rust, rusqlite, argh CLI, chrono, uuid v7

---

## Task 1: Add Project Struct and Database Migration

**Files:**
- Modify: `src/task.rs` (add Project struct)
- Create: `migrations/v3.sql`
- Modify: `src/db.rs` (run v3 migration)

**Step 1: Add Project struct to task.rs**

Add after the `Task` struct definition (around line 160):

```rust
// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Project {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            created_at: Utc::now(),
        }
    }
}
```

**Step 2: Add project_id to Task struct**

Add to Task struct (after `content` field, line 134):

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
```

Update `Task::new()` to include:

```rust
            project_id: None,
```

**Step 3: Create migrations/v3.sql**

```sql
-- v3: Add projects table and project_id to tasks
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id);

CREATE INDEX idx_tasks_project_id ON tasks(project_id);
CREATE INDEX idx_projects_name ON projects(name);
```

**Step 4: Update db.rs to run v3 migration**

Add after the v2 migration check (line 95):

```rust
        if version < 3 {
            let sql = include_str!("../migrations/v3.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute_batch("PRAGMA user_version = 3;")?;
        }
```

**Step 5: Run tests**

```bash
cargo test
```

Expected: All tests pass

**Step 6: Commit**

```bash
git add src/task.rs migrations/v3.sql src/db.rs
git commit -m "feat(data): add Project struct and v3 migration"
```

---

## Task 2: Add Project CRUD Methods to Database

**Files:**
- Modify: `src/db.rs` (add project methods)

**Step 1: Add ProjectStats struct**

Add before `TaskFilter` (around line 11):

```rust
use crate::task::{Creator, Priority, Project, Status, Task};

// ---------------------------------------------------------------------------
// ProjectStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ProjectStats {
    pub total: i64,
    pub pending: i64,
    pub in_progress: i64,
    pub blocked: i64,
    pub done: i64,
    pub cancelled: i64,
}
```

**Step 2: Add project CRUD methods**

Add before `list_tasks` method (around line 193):

```rust
    // -----------------------------------------------------------------------
    // Project operations
    // -----------------------------------------------------------------------

    /// Insert a new project.
    pub fn insert_project(&self, project: &Project) -> Result<(), TodoError> {
        self.conn.execute(
            "INSERT INTO projects (id, name, description, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                project.id,
                project.name,
                project.description,
                project.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Fetch a project by ID.
    pub fn get_project(&self, id: &str) -> Result<Option<Project>, TodoError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, created_at FROM projects WHERE id = ?1",
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
            "SELECT id, name, description, created_at FROM projects WHERE name = ?1",
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
            "SELECT id, name, description, created_at FROM projects ORDER BY name ASC",
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
            "UPDATE projects SET name = ?2, description = ?3 WHERE id = ?1",
            params![project.id, project.name, project.description,],
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
```

**Step 3: Add project_id to TaskFilter**

Modify `TaskFilter` struct (line 14):

```rust
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
    pub project_id: Option<String>,  // NEW
}
```

**Step 4: Add project_id filter to list_tasks**

Add after the overdue filter (around line 258):

```rust
        // project_id
        if let Some(ref project_id) = filter.project_id {
            conditions.push(format!("project_id = ?{}", param_idx));
            param_idx += 1;
            param_values.push(Box::new(project_id.clone()));
        }
```

**Step 5: Add row_to_project helper**

Add after `row_to_task` function (around line 477):

```rust
fn row_to_project(row: &Row<'_>) -> Result<Project, TodoError> {
    let created_at_str: String = row.get(3)?;
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| TodoError::ParseError(e.to_string()))?
            .with_timezone(&Utc),
    })
}
```

**Step 6: Update insert_task and update_task to include project_id**

In `insert_task`, add `project_id` to the SQL and params:

```sql
INSERT INTO tasks (
    id, title, creator, created_at,
    priority, tags, parent_id, due,
    status, assignee, blocked_reason,
    result, artifacts, log,
    started_at, finished_at, content, project_id
) VALUES (
    ?1, ?2, ?3, ?4,
    ?5, ?6, ?7, ?8,
    ?9, ?10, ?11,
    ?12, ?13, ?14,
    ?15, ?16, ?17, ?18
)
```

Add to params:
```rust
task.project_id,
```

Similarly update `update_task` and the SELECT queries in `get_task`, `list_tasks`, `search_tasks`.

Also update `row_to_task` to read `project_id`:
```rust
project_id: row.get(17)?,  // Add at the end
```

**Step 7: Add ProjectHasTasks error**

Add to `src/error.rs`:

```rust
    #[error("Project has {0} tasks. Delete or move them first.")]
    ProjectHasTasks(i64),
```

Add error code:
```rust
            TodoError::ProjectHasTasks(_) => "E_PROJECT_HAS_TASKS",
```

**Step 8: Run tests**

```bash
cargo test
```

Expected: All tests pass

**Step 9: Commit**

```bash
git add src/db.rs src/error.rs
git commit -m "feat(db): add project CRUD methods"
```

---

## Task 3: Add Project CLI Commands

**Files:**
- Modify: `src/cli.rs` (add Project subcommands)
- Create: `src/commands/project.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Add Project subcommands to cli.rs**

Add to `Command` enum (after Search):

```rust
    Project(ProjectArgs),
```

Add the subcommand structs:

```rust
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "project")]
/// Manage projects
pub struct ProjectArgs {
    #[argh(subcommand)]
    pub command: ProjectCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum ProjectCommand {
    ProjectAdd(ProjectAddArgs),
    ProjectEdit(ProjectEditArgs),
    ProjectList(ProjectListArgs),
    ProjectShow(ProjectShowArgs),
    ProjectDelete(ProjectDeleteArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "add")]
/// Create a new project
pub struct ProjectAddArgs {
    /// project name
    #[argh(positional)]
    pub name: String,

    /// project description
    #[argh(option, short = 'd')]
    pub description: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "edit")]
/// Edit project properties
pub struct ProjectEditArgs {
    /// project name
    #[argh(positional)]
    pub name: String,

    /// new project name
    #[argh(option, short = 'n')]
    pub new_name: Option<String>,

    /// new description
    #[argh(option, short = 'd')]
    pub description: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
/// List all projects
pub struct ProjectListArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "show")]
/// Show project details
pub struct ProjectShowArgs {
    /// project name
    #[argh(positional)]
    pub name: String,

    /// number of recent tasks to show
    #[argh(option, short = 'n', default = "5")]
    pub limit: i64,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "delete")]
/// Delete a project
pub struct ProjectDeleteArgs {
    /// project name
    #[argh(positional)]
    pub name: String,
}
```

**Step 2: Create src/commands/project.rs**

```rust
use crate::cli::{ProjectAddArgs, ProjectCommand, ProjectDeleteArgs, ProjectEditArgs, ProjectListArgs, ProjectShowArgs};
use crate::db::{Database, ProjectStats};
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::Project;

pub fn execute(db: &Database, command: ProjectCommand, output: &Output) -> Result<String, TodoError> {
    match command {
        ProjectCommand::ProjectAdd(args) => add(db, args, output),
        ProjectCommand::ProjectEdit(args) => edit(db, args, output),
        ProjectCommand::ProjectList(args) => list(db, args, output),
        ProjectCommand::ProjectShow(args) => show(db, args, output),
        ProjectCommand::ProjectDelete(args) => delete(db, args, output),
    }
}

fn add(db: &Database, args: ProjectAddArgs, output: &Output) -> Result<String, TodoError> {
    // Check if project already exists
    if db.get_project_by_name(&args.name)?.is_some() {
        return Err(TodoError::ProjectExists(args.name));
    }

    let id = generate_id(&db.conn)?;
    let mut project = Project::new(id, args.name);
    project.description = args.description;

    db.insert_project(&project)?;
    Ok(output.project(&project))
}

fn edit(db: &Database, args: ProjectEditArgs, output: &Output) -> Result<String, TodoError> {
    let project = db
        .get_project_by_name(&args.name)?
        .ok_or_else(|| TodoError::ProjectNotFound(args.name.clone()))?;

    let mut updated = project.clone();

    if let Some(new_name) = args.new_name {
        // Check if new name already exists
        if new_name != project.name {
            if db.get_project_by_name(&new_name)?.is_some() {
                return Err(TodoError::ProjectExists(new_name));
            }
        }
        updated.name = new_name;
    }

    if let Some(description) = args.description {
        updated.description = Some(description);
    }

    db.update_project(&updated)?;
    Ok(output.project(&updated))
}

fn list(db: &Database, _args: ProjectListArgs, output: &Output) -> Result<String, TodoError> {
    let projects = db.list_projects()?;
    let mut result = String::new();

    for project in projects {
        let stats = db.get_project_stats(&project.id)?;
        result.push_str(&output.project_list_item(&project, &stats));
        result.push_str("\n\n");
    }

    if result.is_empty() {
        result = "No projects found.\n".to_string();
    }

    Ok(result.trim_end().to_string())
}

fn show(db: &Database, args: ProjectShowArgs, output: &Output) -> Result<String, TodoError> {
    let project = db
        .get_project_by_name(&args.name)?
        .ok_or_else(|| TodoError::ProjectNotFound(args.name.clone()))?;

    let stats = db.get_project_stats(&project.id)?;
    let recent_tasks = db.get_project_recent_tasks(&project.id, args.limit)?;

    Ok(output.project_detail(&project, &stats, &recent_tasks))
}

fn delete(db: &Database, args: ProjectDeleteArgs, output: &Output) -> Result<String, TodoError> {
    let project = db
        .get_project_by_name(&args.name)?
        .ok_or_else(|| TodoError::ProjectNotFound(args.name.clone()))?;

    db.delete_project(&project.id)?;

    Ok(output.project_deleted(&project))
}
```

**Step 3: Update src/commands/mod.rs**

Add:
```rust
pub mod project;
```

**Step 4: Add new errors to error.rs**

```rust
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Project already exists: {0}")]
    ProjectExists(String),
```

Add error codes:
```rust
            TodoError::ProjectNotFound(_) => "E_PROJECT_NOT_FOUND",
            TodoError::ProjectExists(_) => "E_PROJECT_EXISTS",
```

**Step 5: Update main.rs**

Add to Command match:
```rust
        Command::Project(args) => commands::project::execute(&db, args.command, output),
```

**Step 6: Run tests**

```bash
cargo test
```

Expected: Compilation succeeds, tests pass

**Step 7: Commit**

```bash
git add src/cli.rs src/commands/project.rs src/commands/mod.rs src/main.rs src/error.rs
git commit -m "feat(cli): add project subcommands"
```

---

## Task 4: Add Project Output Formatting

**Files:**
- Modify: `src/output.rs`

**Step 1: Add project formatting methods**

Add to `Output` impl:

```rust
    pub fn project(&self, project: &Project) -> String {
        match self.mode {
            OutputMode::Toon => toon_format::encode_default(project).unwrap_or_else(|e| {
                format!("error: failed to encode project to TOON: {}", e)
            }),
            OutputMode::Json => {
                serde_json::to_string_pretty(project).expect("project serialization should not fail")
            }
            OutputMode::Pretty => self.pretty_project(project),
        }
    }

    pub fn project_list_item(&self, project: &Project, stats: &ProjectStats) -> String {
        match self.mode {
            OutputMode::Toon | OutputMode::Json => {
                let obj = serde_json::json!({
                    "id": project.id,
                    "name": project.name,
                    "tasks": stats.total,
                    "task_breakdown": {
                        "pending": stats.pending,
                        "in_progress": stats.in_progress,
                        "done": stats.done,
                    }
                });
                toon_format::encode_default(&obj).unwrap_or_else(|e| {
                    format!("error: failed to encode project to TOON: {}", e)
                })
            }
            OutputMode::Pretty => {
                format!(
                    "id: \"{}\"\nname: {}\ntasks: {} ({} pending, {} in_progress, {} done)",
                    project.id,
                    project.name,
                    stats.total,
                    stats.pending,
                    stats.in_progress,
                    stats.done
                )
            }
        }
    }

    pub fn project_detail(&self, project: &Project, stats: &ProjectStats, recent_tasks: &[Task]) -> String {
        match self.mode {
            OutputMode::Toon | OutputMode::Json => {
                let tasks_json: Vec<_> = recent_tasks.iter().map(|t| serde_json::json!({
                    "id": t.id,
                    "title": t.title,
                    "status": t.status.to_string(),
                })).collect();

                let obj = serde_json::json!({
                    "name": project.name,
                    "description": project.description,
                    "created": project.created_at.format("%Y-%m-%d").to_string(),
                    "statistics": {
                        "total": stats.total,
                        "pending": stats.pending,
                        "in_progress": stats.in_progress,
                        "blocked": stats.blocked,
                        "done": stats.done,
                    },
                    "recent_tasks": tasks_json,
                });
                toon_format::encode_default(&obj).unwrap_or_else(|e| {
                    format!("error: failed to encode project to TOON: {}", e)
                })
            }
            OutputMode::Pretty => {
                let mut out = format!(
                    "name: {}\ndescription: {}\ncreated: {}\n\nstatistics:\n  total: {}\n  pending: {}\n  in_progress: {}\n  blocked: {}\n  done: {}\n\nrecent tasks:\n",
                    project.name,
                    project.description.as_deref().unwrap_or("N/A"),
                    project.created_at.format("%Y-%m-%d"),
                    stats.total,
                    stats.pending,
                    stats.in_progress,
                    stats.blocked,
                    stats.done,
                );

                for task in recent_tasks {
                    out.push_str(&format!("  {}\n", self.pretty_task(task)));
                }

                if recent_tasks.is_empty() {
                    out.push_str("  (no tasks)\n");
                }

                out.trim_end().to_string()
            }
        }
    }

    pub fn project_deleted(&self, project: &Project) -> String {
        match self.mode {
            OutputMode::Toon | OutputMode::Json => {
                let obj = serde_json::json!({
                    "deleted": true,
                    "name": project.name,
                });
                toon_format::encode_default(&obj).unwrap_or_else(|e| {
                    format!("error: failed to encode to TOON: {}", e)
                })
            }
            OutputMode::Pretty => {
                format!("Project '{}' deleted.", project.name)
            }
        }
    }

    fn pretty_project(&self, project: &Project) -> String {
        let mut out = format!("id: \"{}\"\nname: {}", project.id, project.name);
        if let Some(ref desc) = project.description {
            out.push_str(&format!("\ndescription: {}", desc));
        }
        out
    }
```

**Step 2: Add import for Project and ProjectStats**

```rust
use crate::db::ProjectStats;
use crate::task::{Priority, Project, Status, Task};
```

**Step 3: Run tests**

```bash
cargo test
```

Expected: All tests pass

**Step 4: Commit**

```bash
git add src/output.rs
git commit -m "feat(output): add project formatting methods"
```

---

## Task 5: Modify Add Command for Project Syntax

**Files:**
- Modify: `src/commands/add.rs`

**Step 1: Parse `<project>:` prefix**

Modify `execute` function:

```rust
use crate::cli::AddArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::{Creator, Priority, Task};
use crate::time_parse::parse_due;

pub fn execute(db: &Database, args: AddArgs, output: &Output) -> Result<String, TodoError> {
    // Parse project prefix: "project_name: task title"
    let (project_name, title) = parse_project_prefix(&args.title);

    let project = project_name
        .map(|name| {
            db.get_project_by_name(name)
                .and_then(|p| p.ok_or_else(|| TodoError::ProjectNotFound(name.to_string())))
        })
        .transpose()?;

    let priority: Priority = args
        .pri
        .map(|p| p.parse())
        .transpose()?
        .unwrap_or(Priority::Medium);

    let creator: Creator = args
        .creator
        .map(|c| c.parse())
        .transpose()?
        .unwrap_or(Creator::Human);

    let due = args.due.map(|d| parse_due(&d)).transpose()?;

    let id = generate_id(&db.conn)?;

    let mut task = Task::new(id, title);
    task.creator = creator;
    task.priority = priority;
    task.tags = args.tag;
    task.parent_id = args.parent;
    task.due = due;
    task.content = args.description;
    task.project_id = project.map(|p| p.id);

    db.insert_task(&task)?;
    Ok(output.task(&task))
}

/// Parse "project: title" format, returns (project_name, title).
/// Returns (None, original_title) if no project prefix.
fn parse_project_prefix(input: &str) -> (Option<&str>, String) {
    // Find the first colon that's followed by a space or is at the end
    if let Some(pos) = input.find(':') {
        let project = input[..pos].trim();
        let title = input[pos + 1..].trim();

        // Only treat as project prefix if project name is not empty
        if !project.is_empty() && !title.is_empty() {
            return (Some(project), title.to_string());
        }
    }
    (None, input.to_string())
}
```

**Step 2: Run tests**

```bash
cargo test
```

Expected: All tests pass

**Step 3: Commit**

```bash
git add src/commands/add.rs
git commit -m "feat(add): parse project prefix syntax"
```

---

## Task 6: Modify List Command for Project Grouping

**Files:**
- Modify: `src/cli.rs` (add -p option)
- Modify: `src/commands/list.rs`
- Modify: `src/output.rs`

**Step 1: Add -p option to ListArgs in cli.rs**

Add to `ListArgs`:
```rust
    /// filter by project name
    #[argh(option, short = 'p')]
    pub project: Option<String>,
```

**Step 2: Modify list.rs to support project grouping**

```rust
use crate::cli::ListArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::task::Status;
use crate::time_parse::parse_since;

pub fn execute(db: &Database, args: ListArgs, output: &Output) -> Result<String, TodoError> {
    // Resolve project_id if project filter specified
    let project_id = if let Some(ref project_name) = args.project {
        let project = db
            .get_project_by_name(project_name)?
            .ok_or_else(|| TodoError::ProjectNotFound(project_name.clone()))?;
        Some(project.id)
    } else {
        None
    };

    let status = if args.status.is_empty() {
        if args.all {
            None
        } else {
            Some(vec![Status::Pending, Status::InProgress, Status::Blocked])
        }
    } else {
        Some(
            args.status
                .iter()
                .map(|s| s.parse())
                .collect::<Result<Vec<Status>, _>>()?,
        )
    };

    let tags = if args.tag.is_empty() {
        None
    } else {
        Some(args.tag)
    };

    let priority = args.pri.map(|p| p.parse()).transpose()?;
    let creator = args.creator.map(|c| c.parse()).transpose()?;
    let since = args.since.map(|s| parse_since(&s)).transpose()?;

    let filter = TaskFilter {
        status,
        tags,
        priority,
        parent_id: args.parent,
        creator,
        since,
        limit: args.limit,
        sort: None,
        overdue: args.overdue,
        project_id,
    };

    let tasks = db.list_tasks(&filter)?;

    // Group by project if no specific project filter
    if project_id.is_none() {
        Ok(output.task_list_grouped(&tasks, db)?)
    } else {
        Ok(output.task_list(&tasks))
    }
}
```

**Step 3: Add task_list_grouped to output.rs**

```rust
    pub fn task_list_grouped(&self, tasks: &[Task], db: &Database) -> Result<String, TodoError> {
        // Group tasks by project_id
        let mut groups: std::collections::HashMap<Option<String>, Vec<&Task>> =
            std::collections::HashMap::new();

        for task in tasks {
            groups.entry(task.project_id.clone()).or_default().push(task);
        }

        let mut result = String::new();

        // Sort groups: projects with names first, then no-project tasks
        let mut project_ids: Vec<_> = groups.keys().collect();
        project_ids.sort_by(|a, b| {
            match (a, b) {
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(a_id), Some(b_id)) => a_id.cmp(b_id),
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        for project_id in project_ids {
            let group_tasks = groups.get(&project_id).unwrap();

            let project_name = if let Some(ref pid) = project_id {
                db.get_project(pid)?
                    .map(|p| p.name)
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "(no project)".to_string()
            };

            result.push_str(&format!("=== {} ({}) ===\n", project_name, group_tasks.len()));

            for task in group_tasks {
                result.push_str(&self.pretty_task(task));
                result.push('\n');
            }
            result.push('\n');
        }

        Ok(result.trim_end().to_string())
    }
```

**Step 4: Run tests**

```bash
cargo test
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/cli.rs src/commands/list.rs src/output.rs
git commit -m "feat(list): add project grouping and -p filter"
```

---

## Task 7: Add Tests for Project Feature

**Files:**
- Modify: `src/db.rs` (add tests)
- Create: `tests/project_test.rs`

**Step 1: Add db.rs tests**

Add to tests module in `db.rs`:

```rust
    #[test]
    fn insert_and_get_project() {
        let db = test_db();
        let project = Project::new("proj1", "Test Project");

        db.insert_project(&project).unwrap();

        let fetched = db.get_project("proj1").unwrap().expect("project should exist");
        assert_eq!(fetched.id, "proj1");
        assert_eq!(fetched.name, "Test Project");
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
```

**Step 2: Create tests/project_test.rs**

```rust
use assert_cmd::Command;
use tempfile::tempdir;

fn todo_cmd() -> Command {
    Command::cargo_bin("todo").unwrap()
}

#[test]
fn project_add_list_show_delete() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Add project
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "add", "MyProject", "-d", "Test description"])
        .assert()
        .success();

    // List projects
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "list"])
        .assert()
        .stdout(predicates::str::contains("MyProject"))
        .success();

    // Show project
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "show", "MyProject"])
        .assert()
        .stdout(predicates::str::contains("MyProject"))
        .stdout(predicates::str::contains("Test description"))
        .success();

    // Delete project
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "delete", "MyProject"])
        .assert()
        .success();
}

#[test]
fn add_task_with_project() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Create project first
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "add", "Work"])
        .assert()
        .success();

    // Add task with project prefix
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["add", "Work: Do something"])
        .assert()
        .success();

    // List should show task under project
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["list"])
        .assert()
        .stdout(predicates::str::contains("Work"))
        .stdout(predicates::str::contains("Do something"))
        .success();
}

#[test]
fn add_task_nonexistent_project_fails() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["add", "Nonexistent: Task"])
        .assert()
        .stderr(predicates::str::contains("E_PROJECT_NOT_FOUND"))
        .failure();
}

#[test]
fn delete_project_with_tasks_fails() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Create project and task
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "add", "Work"])
        .assert()
        .success();

    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["add", "Work: Task"])
        .assert()
        .success();

    // Delete should fail
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "delete", "Work"])
        .assert()
        .stderr(predicates::str::contains("E_PROJECT_HAS_TASKS"))
        .failure();
}
```

**Step 3: Add test dependencies to Cargo.toml**

```toml
[dev-dependencies]
tempfile = "3.0"
assert_cmd = "2.0"
predicates = "3.0"
```

**Step 4: Run tests**

```bash
cargo test
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/db.rs tests/project_test.rs Cargo.toml
git commit -m "test: add project feature tests"
```

---

## Task 8: Final Verification and Merge

**Step 1: Run full test suite**

```bash
cargo test --all
cargo clippy -- -W clippy::all
```

Expected: No errors

**Step 2: Manual testing**

```bash
# Build release
cargo build --release

# Test workflow
./target/release/todo project add SUMM-Todo -d "CLI tool"
./target/release/todo project add my-website -d "Personal site"

./target/release/todo project list

./target/release/todo add "SUMM-Todo: Implement project feature"
./target/release/todo add "SUMM-Todo: Write documentation"
./target/release/todo add "my-website: Update homepage"

./target/release/todo list
./target/release/todo list -p SUMM-Todo

./target/release/todo project show SUMM-Todo
```

**Step 3: Commit final state**

```bash
git add -A
git commit -m "feat: complete project feature implementation"
```

---

## Summary

| Task | Description |
|------|-------------|
| 1 | Add Project struct and v3 migration |
| 2 | Add project CRUD methods to Database |
| 3 | Add Project CLI commands |
| 4 | Add project output formatting |
| 5 | Modify add command for project syntax |
| 6 | Modify list command for project grouping |
| 7 | Add tests for project feature |
| 8 | Final verification |
