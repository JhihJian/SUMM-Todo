# Multi-Device Sync — Implementation Plan

**Spec:** `docs/plans/2026-04-10-sync-design.md`
**Date:** 2026-04-11

## Scope

3 sequential phases:
1. **Workspace Refactor** — Convert single crate → Cargo workspace, extract `core` crate
2. **Server** — `summ-sync` binary (axum + SQLite)
3. **CLI Sync** — `todo sync` command + change tracking via triggers

**Note:** Phases 2 and 3 are independent subsystems but share the `core` crate and need integration testing together, so they're in one plan.

## File Structure

### Created
| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Workspace root (replaces single-crate `Cargo.toml`) |
| `crates/core/Cargo.toml` | Shared library deps |
| `crates/core/src/lib.rs` | Re-exports |
| `crates/core/src/task.rs` | Task, Project, Status, Creator, Priority, TransitionContext |
| `crates/core/src/error.rs` | TodoError |
| `crates/core/src/id.rs` | generate_id() |
| `crates/server/Cargo.toml` | Server deps (axum, tokio, rusqlite, serde) |
| `crates/server/src/main.rs` | Server entry point |
| `crates/server/src/config.rs` | CLI args & config |
| `crates/server/src/db.rs` | Server-side SQLite layer |
| `crates/server/src/handlers.rs` | API handlers (push, pull, status) |
| `crates/server/src/router.rs` | Route definitions |
| `crates/cli/src/sync/mod.rs` | Sync module |
| `crates/cli/src/sync/client.rs` | HTTP client |
| `crates/cli/src/sync/tracker.rs` | Change tracking |
| `crates/cli/src/commands/sync.rs` | Sync command handler |
| `migrations/v5.sql` | updated_at, sync_config, sync_log, triggers |

### Modified (from existing)
| File | Change |
|------|--------|
| `crates/cli/Cargo.toml` | Add `reqwest`, `todo-core` dependency |
| `crates/cli/src/db.rs` | Add updated_at to SQL, add sync_config queries, add v5 migration |
| `crates/cli/src/cli.rs` | Add Sync subcommand |
| `crates/cli/src/main.rs` | Add Sync dispatch |
| `crates/cli/src/lib.rs` | Add sync module |
| `crates/cli/src/commands/mod.rs` | Add sync module |
| `crates/core/src/task.rs` | Add `updated_at` field to Task and Project |

### Moved (from `src/` → `crates/cli/src/`)
All existing CLI source files with updated import paths (`crate::` → `todo_core::` for core types).

---

## Phase 1: Workspace Refactor

### Task 1: Create workspace Cargo.toml files

Replace root `Cargo.toml` with workspace definition. Create `crates/core/Cargo.toml` and `crates/cli/Cargo.toml`.

**Files:**
- `Cargo.toml` — rewrite as `[workspace]` with `members = ["crates/*"]`
- `crates/core/Cargo.toml` — new, deps: `serde`, `chrono`, `uuid`, `rusqlite`, `thiserror`
- `crates/cli/Cargo.toml` — new, copy existing deps + add `todo-core` path dep

**`Cargo.toml` (workspace root):**
```toml
[workspace]
members = ["crates/*"]
resolver = "2"
```

**`crates/core/Cargo.toml`:**
```toml
[package]
name = "todo-core"
version = "0.3.1"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v7"] }
rusqlite = { version = "0.31", features = ["bundled"] }
```

**`crates/cli/Cargo.toml`:**
```toml
[package]
name = "todo"
version = "0.3.1"
edition = "2021"

[dependencies]
todo-core = { path = "../core" }
argh = "0.1"
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
chrono-english = "0.1"
uuid = { version = "1.0", features = ["v7"] }
dirs = "5.0"
toon-format = "0.4"

[dev-dependencies]
tempfile = "3.0"
assert_cmd = "2.0"
predicates = "3.0"
```

**Verification:** `cargo check --workspace` should fail at this point (files not yet moved). That's expected.

---

### Task 2: Create `crates/core/src/` with extracted types

Copy `src/task.rs`, `src/error.rs`, `src/id.rs` to `crates/core/src/`. Update internal `crate::` references within core.

**Files:**
- `crates/core/src/lib.rs` — new
- `crates/core/src/task.rs` — copy from `src/task.rs`
- `crates/core/src/error.rs` — copy from `src/error.rs`
- `crates/core/src/id.rs` — copy from `src/id.rs`

**`crates/core/src/lib.rs`:**
```rust
pub mod error;
pub mod id;
pub mod task;

pub use error::TodoError;
pub use task::{Creator, Priority, Project, Status, Task, TransitionContext};
```

**Changes in copied files:**
- `task.rs`: `use crate::error::TodoError;` → `use crate::error::TodoError;` (no change, same crate)
- `error.rs`: no internal crate refs, no change
- `id.rs`: `use crate::error::TodoError;` → `use crate::error::TodoError;` (no change)

All three files already use `crate::` internally which maps correctly within the `core` crate.

**Verification:** `cargo check -p todo-core` should pass.

---

### Task 3: Move CLI code to `crates/cli/src/` and fix imports

Copy all files from `src/` to `crates/cli/src/`. Update imports to use `todo_core::` for types from core.

**Files to move to `crates/cli/src/`:**
- `main.rs`
- `lib.rs`
- `cli.rs`
- `db.rs`
- `output.rs`
- `time_parse.rs`
- `commands/*.rs` (all command files)

**Import changes in CLI files:**

In every CLI file that imports from core types, replace:
- `use crate::task::{...}` → `use todo_core::task::{...}` (or use re-exports)
- `use crate::error::TodoError` → `use todo_core::TodoError` (or `use todo_core::error::TodoError`)
- `use crate::id::generate_id` → `use todo_core::id::generate_id`

**Files that need import changes:**
| File | Changes |
|------|---------|
| `main.rs` | `crate::cli` stays, `crate::commands` stays, `crate::db` stays, `crate::error` → `todo_core::TodoError`, `crate::output` stays |
| `lib.rs` | Remove `pub mod error;`, `pub mod id;`, `pub mod task;` (moved to core). Keep `pub mod cli;`, `pub mod commands;`, `pub mod db;`, `pub mod output;`, `pub mod time_parse;` |
| `db.rs` | `use crate::error::TodoError` → `use todo_core::TodoError`, `use crate::task::{...}` → `use todo_core::task::{...}` |
| `output.rs` | `use crate::error::TodoError` → `use todo_core::TodoError`, `use crate::task::{...}` → `use todo_core::task::{...}`, `use crate::db::ProjectStats` stays |
| `time_parse.rs` | `use crate::error::TodoError` → `use todo_core::TodoError` |
| `commands/add.rs` | `use crate::task::Creator` → `use todo_core::task::Creator` |
| `commands/done.rs` | `use crate::task::{Status, TransitionContext}` → `use todo_core::task::{Status, TransitionContext}` |
| `commands/start.rs` | Similar core type imports |
| `commands/block.rs` | Similar |
| `commands/resume.rs` | Similar |
| `commands/cancel.rs` | Similar |
| `commands/undo.rs` | Similar |
| `commands/abandon.rs` | Similar |
| `commands/edit.rs` | Similar |
| `commands/list.rs` | Similar |
| `commands/show.rs` | Similar |
| `commands/log.rs` | Similar |
| `commands/stats.rs` | Similar |
| `commands/search.rs` | Similar |
| `commands/next.rs` | Similar |
| `commands/import.rs` | Similar |
| `commands/export.rs` | Similar |
| `commands/project.rs` | `use crate::id::generate_id` → `use todo_core::id::generate_id`, task imports |
| `db.rs` (tests) | `use crate::task::...` → `use todo_core::task::...` |
| `output.rs` (tests) | `use crate::task::Task` → `use todo_core::task::Task` |

**Also update `migrations/` path:**
- `db.rs` uses `include_str!("../migrations/v1.sql")` — path changes to `include_str!("../../../migrations/v1.sql")` since file is now at `crates/cli/src/db.rs`

**Verification:** `cargo build --workspace` should pass. `cargo test --workspace` should pass.

---

### Task 4: Remove old `src/` directory

Delete the original `src/` directory. Update `.github/workflows/release.yml` if it references paths.

**Commands:**
```bash
rm -rf src/
cargo test --workspace
cargo clippy --workspace -- -W clippy::all
```

**Verification:** `cargo test --workspace` passes all existing tests.

---

### Task 5: Verify CI still works

Check that the release workflow, cross-compilation, and all existing tooling still work.

**Files to check:**
- `.github/workflows/release.yml` — may need path updates for new crate structure
- `Cross.toml` — may need updates

**Verification:**
```bash
cargo test --workspace
cargo build --release -p todo
cargo clippy --workspace -- -W clippy::all
```

**Commit:** `refactor: convert to Cargo workspace with core and CLI crates`

---

## Phase 2: Add `updated_at` Field and v5 Migration

This phase prepares the database for sync by adding the `updated_at` column and change tracking infrastructure.

### Task 6: Add `updated_at` field to Task and Project models

Add `updated_at: DateTime<Utc>` to both structs in `crates/core/src/task.rs`.

**File:** `crates/core/src/task.rs`

**Task struct — add field:**
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub updated_at: Option<DateTime<Utc>>,  // Option for backward compat during migration
```

Place after `finished_at`. Use `Option<DateTime<Utc>>` so existing deserialized tasks (without `updated_at`) don't break. The field becomes `Some(...)` after migration runs.

**Task::new() — add:**
```rust
updated_at: None,  // Set by DB layer on insert
```

**Project struct — add field:**
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub updated_at: Option<DateTime<Utc>>,
```

**Project::new() — add:**
```rust
updated_at: None,
```

**Verification:** `cargo test -p todo-core` passes.

---

### Task 7: Create v5 migration

Create `migrations/v5.sql` with the SQL from the spec.

**File:** `migrations/v5.sql`

```sql
-- v5: Add updated_at columns, sync_config, sync_log, triggers

-- Add updated_at column to tasks table
ALTER TABLE tasks ADD COLUMN updated_at TEXT;
UPDATE tasks SET updated_at = created_at WHERE updated_at IS NULL;

-- Add updated_at column to projects table
ALTER TABLE projects ADD COLUMN updated_at TEXT;
UPDATE projects SET updated_at = created_at WHERE updated_at IS NULL;

-- Sync configuration
CREATE TABLE IF NOT EXISTS sync_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Change tracking log
CREATE TABLE IF NOT EXISTS sync_log (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id, recorded_at)
);

-- Triggers for tasks
CREATE TRIGGER IF NOT EXISTS sync_track_task_insert
AFTER INSERT ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', NEW.id, 'upsert', datetime('now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_task_update
AFTER UPDATE ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', NEW.id, 'upsert', datetime('now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_task_delete
AFTER DELETE ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', OLD.id, 'delete', datetime('now'));
END;

-- Triggers for projects
CREATE TRIGGER IF NOT EXISTS sync_track_project_insert
AFTER INSERT ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', NEW.id, 'upsert', datetime('now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_project_update
AFTER UPDATE ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', NEW.id, 'upsert', datetime('now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_project_delete
AFTER DELETE ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', OLD.id, 'delete', datetime('now'));
END;
```

**File:** `crates/cli/src/db.rs` — add to `run_migrations()`:
```rust
if version < 5 {
    let sql = include_str!("../../../migrations/v5.sql");
    self.conn.execute_batch(sql)?;
    self.conn.execute_batch("PRAGMA user_version = 5;")?;
}
```

**Verification:** `cargo test -p todo` — test `schema_version_is_set` now expects version 5.

---

### Task 8: Update `db.rs` to read/write `updated_at`

Update SQL statements in `db.rs` to include `updated_at` column.

**File:** `crates/cli/src/db.rs`

**`insert_task` — add `updated_at` to INSERT:**
```sql
INSERT INTO tasks (
    id, title, creator, created_at,
    priority, tags, parent_id, due,
    status, assignee, blocked_reason,
    result, artifacts, log,
    started_at, finished_at, content, project_id, updated_at
) VALUES (
    ?1, ?2, ?3, ?4,
    ?5, ?6, ?7, ?8,
    ?9, ?10, ?11,
    ?12, ?13, ?14,
    ?15, ?16, ?17, ?18, ?19
)
```
Add `?19` param: `task.updated_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339())`

**`update_task` — add `updated_at` to SET:**
```sql
UPDATE tasks SET
    title = ?2, creator = ?3, created_at = ?4,
    priority = ?5, tags = ?6, parent_id = ?7, due = ?8,
    status = ?9, assignee = ?10, blocked_reason = ?11,
    result = ?12, artifacts = ?13, log = ?14,
    started_at = ?15, finished_at = ?16, content = ?17, project_id = ?18,
    updated_at = datetime('now')
WHERE id = ?1
```
Note: `update_task` always sets `updated_at = datetime('now')` (server override happens via `insert_task` with explicit value).

**`row_to_task` — read `updated_at`:**
Add to the SELECT columns and parsing. Update column index mapping.

**`insert_project` — add `updated_at`:**
```sql
INSERT INTO projects (id, name, description, path, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6)
```

**`update_project` — add `updated_at`:**
```sql
UPDATE projects SET name = ?2, description = ?3, path = ?4, updated_at = datetime('now')
WHERE id = ?1
```

**`row_to_project` — read `updated_at`:**
Add to SELECT and parsing.

**Verification:** `cargo test -p todo` — all existing tests should still pass (updated_at is now populated).

---

### Task 9: Add `sync_config` query methods to `db.rs`

Add helper methods for reading/writing sync configuration.

**File:** `crates/cli/src/db.rs`

```rust
// --- Sync config operations ---

pub fn get_sync_config(&self, key: &str) -> Result<Option<String>, TodoError> {
    self.conn.query_row(
        "SELECT value FROM sync_config WHERE key = ?1",
        params![key],
        |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRow => TodoError::Database(rusqlite::Error::QueryReturnedNoRow),
        e => TodoError::Database(e),
    }).ok()
}
```

Wait, `query_row` returns `Err(QueryReturnedNoRow)` when no row. Let me handle that:

```rust
pub fn get_sync_config(&self, key: &str) -> Result<Option<String>, TodoError> {
    let mut stmt = self.conn.prepare("SELECT value FROM sync_config WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn set_sync_config(&self, key: &str, value: &str) -> Result<(), TodoError> {
    self.conn.execute(
        "INSERT OR REPLACE INTO sync_config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn clear_sync_log(&self) -> Result<(), TodoError> {
    self.conn.execute("DELETE FROM sync_log", [])?;
    Ok(())
}
```

**Verification:** Write test:
```rust
#[test]
fn sync_config_round_trip() {
    let db = test_db();
    assert!(db.get_sync_config("server_url").unwrap().is_none());
    db.set_sync_config("server_url", "http://localhost:3000").unwrap();
    assert_eq!(db.get_sync_config("server_url").unwrap(), Some("http://localhost:3000".to_string()));
}
```

**Commit:** `feat(db): add updated_at, sync_config, sync_log (v5 migration)`

---

## Phase 3: Server Implementation

### Task 10: Create `crates/server/` skeleton

Create the server crate with `main.rs`, `config.rs`, and `Cargo.toml`.

**File:** `crates/server/Cargo.toml`
```toml
[package]
name = "summ-sync"
version = "0.1.0"
edition = "2021"

[dependencies]
todo-core = { path = "../core" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
```

**File:** `crates/server/src/main.rs`
```rust
mod config;
mod db;
mod handlers;
mod router;

use config::Config;

fn main() {
    let config = Config::from_args();
    // TODO: wire up server
}
```

**File:** `crates/server/src/config.rs`
```rust
pub struct Config {
    pub port: u16,
    pub db_path: String,
    pub api_key: String,
}

impl Config {
    pub fn from_args() -> Self {
        let port: u16 = std::env::args()
            .position(|a| a == "--port")
            .and_then(|i| std::env::args().nth(i + 1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);

        let db_path = std::env::args()
            .position(|a| a == "--db")
            .and_then(|i| std::env::args().nth(i + 1))
            .unwrap_or_else(|| "./sync.db".to_string());

        let api_key = std::env::var("SYNC_API_KEY")
            .ok()
            .or_else(|| {
                std::env::args()
                    .position(|a| a == "--key")
                    .and_then(|i| std::env::args().nth(i + 1))
            })
            .expect("API key required: set SYNC_API_KEY or pass --key");

        Self { port, db_path, api_key }
    }
}
```

**Verification:** `cargo build -p summ-sync` compiles.

---

### Task 11: Implement server database layer

Server-side SQLite with schema for tasks (as JSON blobs), projects, and devices.

**File:** `crates/server/src/db.rs`

```rust
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub struct SyncDb {
    pub conn: Mutex<Connection>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncTask {
    pub id: String,
    pub data: serde_json::Value,  // Full Task JSON blob
    pub updated_at: String,
    pub deleted: bool,
    pub updated_by: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncProject {
    pub id: String,
    pub data: serde_json::Value,
    pub updated_at: String,
    pub deleted: bool,
    pub updated_by: String,
}

impl SyncDb {
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        let db = Self { conn: Mutex::new(conn) };
        db.init()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn: Mutex::new(conn) };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
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
            );"
        )?;
        Ok(())
    }

    // Methods: upsert_task, upsert_project, get_changes_since, record_device_sync, get_status
    // ... (detailed implementations per spec)
}
```

**Key methods to implement:**
1. `upsert_task(id, data_json, updated_at, device_id)` — INSERT OR REPLACE, returns old updated_at if conflict
2. `soft_delete_task(id, updated_at, device_id)` — Set deleted=1
3. `upsert_project(id, data_json, updated_at, device_id)` — Same pattern
4. `soft_delete_project(id, updated_at, device_id)` — Set deleted=1
5. `get_changes_since(since_timestamp)` — Returns tasks/projects updated since, plus deleted IDs
6. `record_device_sync(device_id, timestamp)` — INSERT OR REPLACE into devices
7. `get_status()` — Returns total_tasks, last_modified, devices list

**Verification:** Write tests:
```rust
#[test]
fn upsert_and_retrieve_task() { ... }
#[test]
fn conflict_detection() { ... }  // older timestamp rejected
#[test]
fn get_changes_since() { ... }
```

Run: `cargo test -p summ-sync`

---

### Task 12: Implement API handlers

HTTP handlers for `/api/v1/sync/push`, `/pull`, `/status`.

**File:** `crates/server/src/handlers.rs`

Request/response types (from spec):
```rust
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PushRequest {
    pub device_id: String,
    pub tasks: Vec<serde_json::Value>,     // Full Task JSON blobs
    pub deleted_ids: Vec<String>,
    pub projects: Vec<serde_json::Value>,
    pub deleted_project_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct PushResponse {
    pub ok: bool,
    pub conflicts: Vec<Conflict>,
}

#[derive(Serialize)]
pub struct Conflict {
    pub id: String,
    pub client_updated_at: String,
    pub server_updated_at: String,
    pub server_data: serde_json::Value,
}

#[derive(Deserialize)]
pub struct PullRequest {
    pub device_id: String,
    pub since: String,
}

#[derive(Serialize)]
pub struct PullResponse {
    pub tasks: Vec<serde_json::Value>,
    pub deleted_ids: Vec<String>,
    pub projects: Vec<serde_json::Value>,
    pub deleted_project_ids: Vec<String>,
    pub server_time: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub total_tasks: i64,
    pub last_modified: Option<String>,
    pub devices: Vec<DeviceInfo>,
}

#[derive(Serialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_sync: String,
}
```

Handler functions:
```rust
pub async fn push(State(state): State<AppState>, Json(req): Json<PushRequest>) -> Json<PushResponse> { ... }
pub async fn pull(State(state): State<AppState>, Json(req): Json<PullRequest>) -> Json<PullResponse> { ... }
pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> { ... }
```

**Push logic (per task):**
1. Extract `id` and `updated_at` from the JSON blob
2. Compare `updated_at` with server value
3. If client >= server: overwrite (upsert)
4. If client < server: add to conflicts list, keep server version
5. Same for projects

**Pull logic:**
1. Query all tasks/projects with `updated_at > since` AND `deleted = 0`
2. Query all tasks/projects with `deleted = 1` AND `updated_at > since` (for deleted_ids)
3. Record device sync timestamp
4. Return results with `server_time = now()`

**Verification:** `cargo test -p summ-sync`

---

### Task 13: Implement router, auth middleware, and wire up server

**File:** `crates/server/src/router.rs`
```rust
use axum::{routing::{get, post}, Router, middleware, extract::State};
use crate::handlers;

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/sync/push", post(handlers::push))
        .route("/sync/pull", post(handlers::pull))
        .route("/sync/status", get(handlers::status))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    Router::new().nest("/api/v1", api).with_state(state)
}

async fn auth_middleware(
    State(config): State<Config>,
    req: axum::extract::Request,
    next: middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let auth = req.headers().get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth {
        Some(key) if key == config.api_key => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

**File:** `crates/server/src/main.rs` — complete:
```rust
#[tokio::main]
async fn main() {
    let config = Config::from_args();
    let db = SyncDb::open(&config.db_path).expect("failed to open database");
    let state = AppState { db, config: config.clone() };
    let app = router::build_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    println!("summ-sync listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

**Verification:** `cargo build -p summ-sync`. Start server manually and curl the endpoints.

**Commit:** `feat(server): add summ-sync server with push/pull/status API`

---

## Phase 4: CLI Sync Integration

### Task 14: Add sync module skeleton to CLI

Create the `sync/` module directory structure in CLI.

**Files:**
- `crates/cli/src/sync/mod.rs` — new
- `crates/cli/src/sync/client.rs` — new (empty for now)
- `crates/cli/src/sync/tracker.rs` — new (empty for now)

**`crates/cli/src/sync/mod.rs`:**
```rust
pub mod client;
pub mod tracker;
```

**`crates/cli/src/lib.rs` — add:**
```rust
pub mod sync;
```

**`crates/cli/Cargo.toml` — add:**
```toml
reqwest = { version = "0.12", features = ["json"] }
```

**Verification:** `cargo build -p todo` compiles.

---

### Task 15: Implement `tracker.rs` — change tracking

Queries `sync_log` for pending changes and resolves latest action per entity.

**File:** `crates/cli/src/sync/tracker.rs`

```rust
use crate::db::Database;
use crate::error::TodoError;
use todo_core::task::{Task, Project};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingChange {
    pub entity_type: String,  // "task" or "project"
    pub entity_id: String,
    pub action: String,       // "upsert" or "delete"
}

pub struct SyncTracker<'a> {
    db: &'a Database,
}

impl<'a> SyncTracker<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get pending changes from sync_log, resolving to latest action per entity.
    /// Returns (upserted_tasks, deleted_task_ids, upserted_projects, deleted_project_ids)
    pub fn get_pending_changes(&self) -> Result<(Vec<Task>, Vec<String>, Vec<Project>, Vec<String>), TodoError> {
        // 1. Query sync_log for all entries, resolve latest action per (entity_type, entity_id)
        // 2. For 'upsert' actions, fetch full entity data
        // 3. For 'delete' actions, just collect the ID
        // 4. Return structured results
    }

    /// Clear sync_log after successful push.
    pub fn clear_log(&self) -> Result<(), TodoError> {
        self.db.clear_sync_log()
    }
}
```

**Implementation of `get_pending_changes`:**
1. Query: `SELECT entity_type, entity_id, action FROM sync_log ORDER BY recorded_at ASC`
2. Build HashMap<(entity_type, entity_id), action> — last entry wins (latest action)
3. For each "upsert" task: fetch full task from DB via `db.get_task(id)`
4. For each "delete" task: collect id into Vec
5. Same for projects

**Verification:** Write test using in-memory DB with v5 migration:
```rust
#[test]
fn tracker_detects_insert() {
    let db = Database::open_in_memory().unwrap();
    // Insert a task
    let task = Task::new("abc1", "Test");
    db.insert_task(&task).unwrap();
    // Tracker should detect it
    let tracker = SyncTracker::new(&db);
    let (tasks, deleted, _, _) = tracker.get_pending_changes().unwrap();
    assert_eq!(tasks.len(), 1);
    assert!(deleted.is_empty());
}
```

Run: `cargo test -p todo tracker`

---

### Task 16: Implement `client.rs` — HTTP sync client

HTTP client that communicates with the sync server.

**File:** `crates/cli/src/sync/client.rs`

```rust
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use crate::error::TodoError;
use todo_core::task::{Task, Project};

#[derive(Debug, Serialize)]
pub struct PushPayload {
    pub device_id: String,
    pub tasks: Vec<serde_json::Value>,
    pub deleted_ids: Vec<String>,
    pub projects: Vec<serde_json::Value>,
    pub deleted_project_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PushResponse {
    pub ok: bool,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub client_updated_at: String,
    pub server_updated_at: String,
    pub server_data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PullPayload {
    pub device_id: String,
    pub since: String,
}

#[derive(Debug, Deserialize)]
pub struct PullResponse {
    pub tasks: Vec<serde_json::Value>,
    pub deleted_ids: Vec<String>,
    pub projects: Vec<serde_json::Value>,
    pub deleted_project_ids: Vec<String>,
    pub server_time: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub total_tasks: i64,
    pub last_modified: Option<String>,
    pub devices: Vec<DeviceInfo>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_sync: String,
}

pub struct SyncClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl SyncClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    pub fn push(&self, payload: &PushPayload) -> Result<PushResponse, TodoError> {
        let resp = self.http
            .post(self.url("/sync/push"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(payload)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .map_err(|e| TodoError::SyncError(e.to_string()))?;
        // Handle status, parse response
    }

    pub fn pull(&self, payload: &PullPayload) -> Result<PullResponse, TodoError> {
        // Same pattern as push
    }

    pub fn status(&self) -> Result<StatusResponse, TodoError> {
        // GET /api/v1/sync/status
    }
}
```

**Retry logic:** Both push and pull retry once after 1s delay on timeout/connection error.

**Verification:** Unit test with mock server (or integration test with real server).

---

### Task 17: Add sync error variants to `TodoError`

**File:** `crates/core/src/error.rs`

```rust
#[error("Sync error: {0}")]
SyncError(String),

#[error("Sync server unreachable")]
SyncServerUnreachable,

#[error("Sync authentication failed")]
SyncAuthFailed,
```

Update `code()` and `exit_code()` methods accordingly:
```rust
TodoError::SyncError(_) => "E_SYNC_ERROR",
TodoError::SyncServerUnreachable => "E_SYNC_SERVER_UNREACHABLE",
TodoError::SyncAuthFailed => "E_SYNC_AUTH_FAILED",
```

**Verification:** `cargo test -p todo-core`

---

### Task 18: Add sync CLI args and command dispatch

**File:** `crates/cli/src/cli.rs`

Add `Sync` variant to `Command` enum:
```rust
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Command {
    // ... existing commands
    Sync(SyncArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "sync")]
/// Sync tasks with remote server
pub struct SyncArgs {
    #[argh(subcommand)]
    pub command: Option<SyncCommand>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum SyncCommand {
    SyncInit(SyncInitArgs),
    SyncPush(SyncPushArgs),
    SyncPull(SyncPullArgs),
    SyncStatus(SyncStatusArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "init")]
/// Initialize sync configuration
pub struct SyncInitArgs {
    /// server URL
    #[argh(option)]
    pub server: String,

    /// API key
    #[argh(option)]
    pub key: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "push")]
/// Push local changes to server
pub struct SyncPushArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "pull")]
/// Pull changes from server
pub struct SyncPullArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "status")]
/// Show sync status
pub struct SyncStatusArgs {}
```

**File:** `crates/cli/src/main.rs` — add dispatch:
```rust
Command::Sync(args) => commands::sync::execute(&db, args, output),
```

**File:** `crates/cli/src/commands/mod.rs` — add:
```rust
pub mod sync;
```

**Verification:** `cargo build -p todo`

---

### Task 19: Implement `commands/sync.rs` — sync command handler

**File:** `crates/cli/src/commands/sync.rs`

```rust
use crate::cli::{SyncArgs, SyncCommand};
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::sync::client::SyncClient;
use crate::sync::tracker::SyncTracker;
use todo_core::id;

pub fn execute(db: &Database, args: SyncArgs, output: &Output) -> Result<String, TodoError> {
    match args.command {
        None => full_sync(db, output),                    // todo sync
        Some(SyncCommand::SyncInit(a)) => sync_init(db, &a, output),
        Some(SyncCommand::SyncPush(_)) => sync_push(db, output),
        Some(SyncCommand::SyncPull(_)) => sync_pull(db, output),
        Some(SyncCommand::SyncStatus(_)) => sync_status(db, output),
    }
}

fn sync_init(db: &Database, args: &SyncInitArgs, output: &Output) -> Result<String, TodoError> {
    // 1. Generate or reuse device_id
    let device_id = match db.get_sync_config("device_id")? {
        Some(id) => id,
        None => {
            let uuid = uuid::Uuid::now_v7();
            let id = uuid.simple().to_string()[24..].to_string();
            db.set_sync_config("device_id", &id)?;
            id
        }
    };

    // 2. Save server_url + api_key
    db.set_sync_config("server_url", &args.server)?;
    db.set_sync_config("api_key", &args.key)?;

    // 3. Validate connectivity
    let client = SyncClient::new(&args.server, &args.key);
    let status = client.status()?;

    // 4. Initial sync: pull first, then push all
    sync_pull(db, output)?;
    sync_push(db, output)?;

    Ok(format!("Sync initialized. Device ID: {}. Server has {} tasks.", device_id, status.total_tasks))
}

fn full_sync(db: &Database, output: &Output) -> Result<String, TodoError> {
    sync_pull(db, output)?;
    sync_push(db, output)?;
    Ok("Sync complete.".to_string())
}

fn sync_push(db: &Database, output: &Output) -> Result<String, TodoError> {
    let server_url = db.get_sync_config("server_url")?
        .ok_or(TodoError::SyncError("Not initialized. Run: todo sync init".into()))?;
    let api_key = db.get_sync_config("api_key")?
        .ok_or(TodoError::SyncError("Not initialized. Run: todo sync init".into()))?;
    let device_id = db.get_sync_config("device_id")?
        .ok_or(TodoError::SyncError("Not initialized. Run: todo sync init".into()))?;

    let tracker = SyncTracker::new(db);
    let (tasks, deleted_ids, projects, deleted_project_ids) = tracker.get_pending_changes()?;

    let client = SyncClient::new(&server_url, &api_key);
    let payload = PushPayload {
        device_id,
        tasks: tasks.iter().map(|t| serde_json::to_value(t).unwrap()).collect(),
        deleted_ids,
        projects: projects.iter().map(|p| serde_json::to_value(p).unwrap()).collect(),
        deleted_project_ids,
    };

    let response = client.push(&payload)?;
    tracker.clear_log()?;

    // Update last_sync_at
    db.set_sync_config("last_sync_at", &chrono::Utc::now().to_rfc3339())?;

    if response.conflicts.is_empty() {
        Ok(format!("Pushed {} tasks, {} projects.", payload.tasks.len(), payload.projects.len()))
    } else {
        Ok(format!("Pushed with {} conflicts.", response.conflicts.len()))
    }
}

fn sync_pull(db: &Database, output: &Output) -> Result<String, TodoError> {
    // Similar: get config, create client, pull, upsert/delete locally
}

fn sync_status(db: &Database, output: &Output) -> Result<String, TodoError> {
    // GET /api/v1/sync/status and display
}
```

**Pull import logic:**
1. For each task in response: `INSERT OR REPLACE` (upsert by ID), skip if local `updated_at > server updated_at`
2. For each `deleted_id`: delete locally if `updated_at <= deletion time`
3. Same for projects
4. Store `server_time` as `last_sync_at`

**Verification:** `cargo build -p todo`

---

### Task 20: Integration test — full sync round trip

**File:** `tests/sync_test.rs` (or `crates/cli/tests/sync_test.rs`)

Test the full flow:
1. Create two in-memory CLI databases (simulating two devices)
2. Start the sync server on a random port
3. Device A: `sync init`, add a task, `sync push`
4. Device B: `sync init`, `sync pull` — verify task appears
5. Device B: modify task, `sync push`
6. Device A: `sync pull` — verify modification

**Verification:** `cargo test --workspace`

---

### Task 21: Update existing mutation commands to set `updated_at`

Ensure all commands that modify tasks/projects set `updated_at = Some(Utc::now())` before calling DB methods.

**Files to check:**
| Command file | Mutation | Action |
|-------------|----------|--------|
| `commands/add.rs` | `db.insert_task()` | Set `task.updated_at = Some(Utc::now())` before insert |
| `commands/start.rs` | `db.update_task()` | Set `task.updated_at = Some(Utc::now())` before update |
| `commands/done.rs` | `db.update_task()` | Set `task.updated_at = Some(Utc::now())` before update |
| `commands/block.rs` | `db.update_task()` | Same |
| `commands/resume.rs` | `db.update_task()` | Same |
| `commands/cancel.rs` | `db.update_task()` | Same |
| `commands/undo.rs` | `db.update_task()` | Same |
| `commands/abandon.rs` | `db.update_task()` | Same |
| `commands/edit.rs` | `db.update_task()` | Same |
| `commands/import.rs` | `db.insert_task()` | Set for each imported task |
| `commands/project.rs` (add) | `db.insert_project()` | Set `project.updated_at` |
| `commands/project.rs` (edit) | `db.update_project()` | Set `project.updated_at` |

**Note:** The `update_task` SQL already sets `updated_at = datetime('now')` in the SET clause (from Task 8). The Rust model field should match. Since triggers also fire on UPDATE, the `sync_log` gets populated automatically.

**Verification:** `cargo test --workspace`

**Commit:** `feat(sync): add todo sync command with push/pull/init/status`

---

### Task 22: Final verification and cleanup

1. Run full test suite: `cargo test --workspace`
2. Run clippy: `cargo clippy --workspace -- -W clippy::all`
3. Manual smoke test:
   ```bash
   # Build
   cargo build --release -p todo -p summ-sync

   # Start server
   SYNC_API_KEY=test-key ./target/release/summ-sync --port 3000 --db /tmp/sync.db &

   # CLI usage
   ./target/release/todo sync init --server http://localhost:3000 --key test-key
   ./target/release/todo add "Test sync task"
   ./target/release/todo sync push
   ./target/release/todo sync pull
   ./target/release/todo sync status
   ```

**Final commit:** `feat: add multi-device sync (workspace refactor + server + CLI integration)`
