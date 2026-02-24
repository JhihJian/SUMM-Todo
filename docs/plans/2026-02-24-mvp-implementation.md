# SUMM-Todo MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use summ:executing-plans to implement this plan task-by-task.

**Goal:** Build a complete CLI task coordination tool (`todo`) in Rust with SQLite storage, full state machine, and JSON/pretty output.

**Architecture:** Four-layer design: CLI (argh) -> Command Handlers -> Domain (Task model + state machine) -> Storage (rusqlite + SQLite). All state transitions enforced in the domain layer. Output defaults to JSON, `--pretty` for humans.

**Tech Stack:** Rust, argh, rusqlite (bundled), serde/serde_json, thiserror, chrono, uuid v7, dirs

**Reference docs:**
- PRD: `summ-todo-prd-v2.md`
- Technical Design: `docs/plans/2026-02-24-technical-design.md`

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`

**Step 1: Initialize Cargo project**

Run: `cargo init --name todo /data/dev/SUMM-Todo`
Expected: Creates Cargo.toml and src/main.rs

**Step 2: Set up Cargo.toml with dependencies**

```toml
[package]
name = "todo"
version = "0.1.0"
edition = "2021"

[dependencies]
argh = "0.1"
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v7"] }
dirs = "5.0"

[dev-dependencies]
tempfile = "3.0"
```

**Step 3: Create src/lib.rs with module declarations**

```rust
pub mod cli;
pub mod commands;
pub mod db;
pub mod error;
pub mod id;
pub mod output;
pub mod task;
pub mod time_parse;
```

**Step 4: Create stub modules**

Create empty files for each module listed in lib.rs, plus `src/commands/mod.rs`.

**Step 5: Create minimal main.rs**

```rust
fn main() {
    println!("todo: not yet implemented");
}
```

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Successful build (stub modules may need minimal content)

**Step 7: Commit**

```bash
git init
git add Cargo.toml src/
git commit -m "chore: scaffold Rust project with dependencies"
```

---

### Task 2: Error Types

**Files:**
- Create: `src/error.rs`
- Test: inline `#[cfg(test)]` module

**Step 1: Write tests for error codes**

```rust
// src/error.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_match_prd() {
        let err = TodoError::InvalidTransition {
            from: "pending".into(),
            to: "done".into(),
        };
        assert_eq!(err.code(), "E_INVALID_TRANSITION");

        assert_eq!(TodoError::ResultRequired.code(), "E_RESULT_REQUIRED");
        assert_eq!(TodoError::BlockedReasonRequired.code(), "E_BLOCKED_REASON_REQUIRED");
        assert_eq!(TodoError::TaskNotFound("x".into()).code(), "E_TASK_NOT_FOUND");
        assert_eq!(TodoError::QueueEmpty.code(), "E_QUEUE_EMPTY");
    }

    #[test]
    fn format_error_produces_valid_json() {
        let err = TodoError::QueueEmpty;
        let json = format_error(&err);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"], "E_QUEUE_EMPTY");
        assert!(parsed["message"].as_str().unwrap().contains("pending"));
    }

    #[test]
    fn exit_code_mapping() {
        assert_eq!(TodoError::QueueEmpty.exit_code(), 1);
        assert_eq!(TodoError::InvalidInput("x".into()).exit_code(), 1);
        assert_eq!(TodoError::Database(rusqlite::Error::QueryReturnedNoRows).exit_code(), 2);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib error`
Expected: FAIL - TodoError not defined

**Step 3: Implement error types**

```rust
// src/error.rs

use serde_json;

#[derive(Debug, thiserror::Error)]
pub enum TodoError {
    #[error("Invalid state transition: cannot go from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Result is required when completing a task")]
    ResultRequired,

    #[error("Blocked reason is required when blocking a task")]
    BlockedReasonRequired,

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("No pending tasks match filters")]
    QueueEmpty,

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl TodoError {
    pub fn code(&self) -> &'static str {
        match self {
            TodoError::InvalidTransition { .. } => "E_INVALID_TRANSITION",
            TodoError::ResultRequired => "E_RESULT_REQUIRED",
            TodoError::BlockedReasonRequired => "E_BLOCKED_REASON_REQUIRED",
            TodoError::TaskNotFound(_) => "E_TASK_NOT_FOUND",
            TodoError::QueueEmpty => "E_QUEUE_EMPTY",
            TodoError::Database(_) => "E_DATABASE",
            TodoError::InvalidInput(_) => "E_INVALID_INPUT",
            TodoError::ParseError(_) => "E_PARSE_ERROR",
            TodoError::Io(_) => "E_IO",
        }
    }

    /// PRD: 0 success, 1 user input error, 2 system internal error
    pub fn exit_code(&self) -> i32 {
        match self {
            TodoError::Database(_) | TodoError::Io(_) => 2,
            _ => 1,
        }
    }
}

pub fn format_error(err: &TodoError) -> String {
    serde_json::json!({
        "error": err.code(),
        "message": err.to_string()
    })
    .to_string()
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib error`
Expected: All 3 tests PASS

**Step 5: Commit**

```bash
git add src/error.rs
git commit -m "feat: add error types with PRD-defined error codes"
```

---

### Task 3: Task Model and State Machine

**Files:**
- Create: `src/task.rs`
- Test: inline `#[cfg(test)]` module

**Step 1: Write tests for task creation defaults**

```rust
// src/task.rs - tests module

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_has_correct_defaults() {
        let task = Task::new("test-id".into(), "Do something".into());
        assert_eq!(task.status, Status::Pending);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.creator, Creator::Human);
        assert!(task.tags.is_empty());
        assert!(task.assignee.is_none());
        assert!(task.result.is_none());
        assert!(task.started_at.is_none());
        assert!(task.finished_at.is_none());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib task`
Expected: FAIL - types not defined

**Step 3: Implement Task struct and enums**

```rust
// src/task.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::TodoError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pending => write!(f, "pending"),
            Status::InProgress => write!(f, "in_progress"),
            Status::Blocked => write!(f, "blocked"),
            Status::Done => write!(f, "done"),
            Status::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for Status {
    type Err = TodoError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Status::Pending),
            "in_progress" => Ok(Status::InProgress),
            "blocked" => Ok(Status::Blocked),
            "done" => Ok(Status::Done),
            "cancelled" => Ok(Status::Cancelled),
            _ => Err(TodoError::InvalidInput(format!("Unknown status: {}", s))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Creator {
    Human,
    Agent,
}

impl fmt::Display for Creator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Creator::Human => write!(f, "human"),
            Creator::Agent => write!(f, "agent"),
        }
    }
}

impl std::str::FromStr for Creator {
    type Err = TodoError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(Creator::Human),
            "agent" => Ok(Creator::Agent),
            _ => Err(TodoError::InvalidInput(format!("Unknown creator: {}", s))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = TodoError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "high" => Ok(Priority::High),
            "medium" => Ok(Priority::Medium),
            "low" => Ok(Priority::Low),
            _ => Err(TodoError::InvalidInput(format!("Unknown priority: {}", s))),
        }
    }
}

/// Context passed to state transitions
pub struct TransitionContext {
    pub assignee: Option<Creator>,
    pub result: Option<String>,
    pub artifacts: Option<Vec<String>>,
    pub log: Option<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub creator: Creator,
    pub created_at: DateTime<Utc>,

    pub priority: Priority,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub due: Option<DateTime<Utc>>,

    pub status: Status,
    pub assignee: Option<Creator>,
    pub blocked_reason: Option<String>,

    pub result: Option<String>,
    pub artifacts: Vec<String>,
    pub log: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(id: String, title: String) -> Self {
        Task {
            id,
            title,
            creator: Creator::Human,
            created_at: Utc::now(),
            priority: Priority::Medium,
            tags: Vec::new(),
            parent_id: None,
            due: None,
            status: Status::Pending,
            assignee: None,
            blocked_reason: None,
            result: None,
            artifacts: Vec::new(),
            log: None,
            started_at: None,
            finished_at: None,
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib task`
Expected: PASS

**Step 5: Write tests for state transitions**

Add to the tests module:

```rust
    #[test]
    fn pending_to_in_progress() {
        let mut task = Task::new("t1".into(), "test".into());
        let ctx = TransitionContext {
            assignee: Some(Creator::Agent),
            result: None, artifacts: None, log: None, blocked_reason: None,
        };
        task.transition(Status::InProgress, ctx).unwrap();
        assert_eq!(task.status, Status::InProgress);
        assert_eq!(task.assignee, Some(Creator::Agent));
        assert!(task.started_at.is_some());
    }

    #[test]
    fn in_progress_to_done_requires_result() {
        let mut task = Task::new("t1".into(), "test".into());
        // First move to in_progress
        let ctx = TransitionContext {
            assignee: Some(Creator::Human),
            result: None, artifacts: None, log: None, blocked_reason: None,
        };
        task.transition(Status::InProgress, ctx).unwrap();

        // Try done without result
        let ctx = TransitionContext {
            assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
        };
        let err = task.transition(Status::Done, ctx).unwrap_err();
        assert_eq!(err.code(), "E_RESULT_REQUIRED");
    }

    #[test]
    fn in_progress_to_done_with_result() {
        let mut task = Task::new("t1".into(), "test".into());
        let ctx = TransitionContext {
            assignee: Some(Creator::Human),
            result: None, artifacts: None, log: None, blocked_reason: None,
        };
        task.transition(Status::InProgress, ctx).unwrap();

        let ctx = TransitionContext {
            assignee: None,
            result: Some("Done it".into()),
            artifacts: Some(vec!["commit:abc".into()]),
            log: Some("went smoothly".into()),
            blocked_reason: None,
        };
        task.transition(Status::Done, ctx).unwrap();
        assert_eq!(task.status, Status::Done);
        assert_eq!(task.result.as_deref(), Some("Done it"));
        assert!(task.finished_at.is_some());
    }

    #[test]
    fn block_requires_reason() {
        let mut task = Task::new("t1".into(), "test".into());
        let ctx = TransitionContext {
            assignee: Some(Creator::Human),
            result: None, artifacts: None, log: None, blocked_reason: None,
        };
        task.transition(Status::InProgress, ctx).unwrap();

        let ctx = TransitionContext {
            assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
        };
        let err = task.transition(Status::Blocked, ctx).unwrap_err();
        assert_eq!(err.code(), "E_BLOCKED_REASON_REQUIRED");
    }

    #[test]
    fn block_and_resume() {
        let mut task = Task::new("t1".into(), "test".into());
        let ctx = TransitionContext {
            assignee: Some(Creator::Human),
            result: None, artifacts: None, log: None, blocked_reason: None,
        };
        task.transition(Status::InProgress, ctx).unwrap();

        let ctx = TransitionContext {
            assignee: None, result: None, artifacts: None, log: None,
            blocked_reason: Some("need API key".into()),
        };
        task.transition(Status::Blocked, ctx).unwrap();
        assert_eq!(task.status, Status::Blocked);
        assert_eq!(task.blocked_reason.as_deref(), Some("need API key"));

        let ctx = TransitionContext {
            assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
        };
        task.transition(Status::InProgress, ctx).unwrap();
        assert_eq!(task.status, Status::InProgress);
        assert!(task.blocked_reason.is_none());
    }

    #[test]
    fn cancel_from_any_non_terminal_state() {
        for initial_status in [Status::Pending, Status::InProgress, Status::Blocked] {
            let mut task = Task::new("t1".into(), "test".into());
            // Move to initial_status
            match initial_status {
                Status::InProgress => {
                    let ctx = TransitionContext {
                        assignee: Some(Creator::Human),
                        result: None, artifacts: None, log: None, blocked_reason: None,
                    };
                    task.transition(Status::InProgress, ctx).unwrap();
                }
                Status::Blocked => {
                    let ctx = TransitionContext {
                        assignee: Some(Creator::Human),
                        result: None, artifacts: None, log: None, blocked_reason: None,
                    };
                    task.transition(Status::InProgress, ctx).unwrap();
                    let ctx = TransitionContext {
                        assignee: None, result: None, artifacts: None, log: None,
                        blocked_reason: Some("reason".into()),
                    };
                    task.transition(Status::Blocked, ctx).unwrap();
                }
                _ => {}
            }
            let ctx = TransitionContext {
                assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
            };
            task.transition(Status::Cancelled, ctx).unwrap();
            assert_eq!(task.status, Status::Cancelled);
        }
    }

    #[test]
    fn terminal_states_reject_transitions() {
        for terminal in [Status::Done, Status::Cancelled] {
            let mut task = Task::new("t1".into(), "test".into());
            // Move to in_progress first
            let ctx = TransitionContext {
                assignee: Some(Creator::Human),
                result: None, artifacts: None, log: None, blocked_reason: None,
            };
            task.transition(Status::InProgress, ctx).unwrap();

            // Move to terminal
            match terminal {
                Status::Done => {
                    let ctx = TransitionContext {
                        assignee: None, result: Some("done".into()),
                        artifacts: None, log: None, blocked_reason: None,
                    };
                    task.transition(Status::Done, ctx).unwrap();
                }
                Status::Cancelled => {
                    let ctx = TransitionContext {
                        assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
                    };
                    task.transition(Status::Cancelled, ctx).unwrap();
                }
                _ => unreachable!(),
            }

            // Try invalid transition
            let ctx = TransitionContext {
                assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
            };
            let err = task.transition(Status::InProgress, ctx).unwrap_err();
            assert_eq!(err.code(), "E_INVALID_TRANSITION");
        }
    }

    #[test]
    fn idempotent_same_state() {
        let mut task = Task::new("t1".into(), "test".into());
        // pending -> pending is idempotent
        let ctx = TransitionContext {
            assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
        };
        task.transition(Status::Pending, ctx).unwrap();
        assert_eq!(task.status, Status::Pending);
    }

    #[test]
    fn illegal_pending_to_done() {
        let mut task = Task::new("t1".into(), "test".into());
        let ctx = TransitionContext {
            assignee: None, result: Some("skip".into()),
            artifacts: None, log: None, blocked_reason: None,
        };
        let err = task.transition(Status::Done, ctx).unwrap_err();
        assert_eq!(err.code(), "E_INVALID_TRANSITION");
    }
```

**Step 6: Run tests to verify they fail**

Run: `cargo test --lib task`
Expected: FAIL - `transition` method not defined

**Step 7: Implement state machine transition method**

Add to `impl Task`:

```rust
    pub fn transition(&mut self, target: Status, ctx: TransitionContext) -> Result<(), TodoError> {
        // Idempotent: already in target state
        if self.status == target {
            return Ok(());
        }

        match (&self.status, &target) {
            (Status::Pending, Status::InProgress) => {
                self.assignee = ctx.assignee;
                self.started_at = Some(Utc::now());
            }
            (Status::Pending, Status::Cancelled) => {
                self.finished_at = Some(Utc::now());
            }
            (Status::InProgress, Status::Done) => {
                let result = ctx.result.ok_or(TodoError::ResultRequired)?;
                self.result = Some(result);
                self.artifacts = ctx.artifacts.unwrap_or_default();
                self.log = ctx.log;
                self.finished_at = Some(Utc::now());
            }
            (Status::InProgress, Status::Blocked) => {
                let reason = ctx.blocked_reason.ok_or(TodoError::BlockedReasonRequired)?;
                self.blocked_reason = Some(reason);
            }
            (Status::InProgress, Status::Cancelled) => {
                self.finished_at = Some(Utc::now());
            }
            (Status::Blocked, Status::InProgress) => {
                self.blocked_reason = None;
            }
            (Status::Blocked, Status::Cancelled) => {
                self.blocked_reason = None;
                self.finished_at = Some(Utc::now());
            }
            _ => {
                return Err(TodoError::InvalidTransition {
                    from: self.status.to_string(),
                    to: target.to_string(),
                });
            }
        }

        self.status = target;
        Ok(())
    }
```

**Step 8: Run tests to verify they pass**

Run: `cargo test --lib task`
Expected: All tests PASS

**Step 9: Commit**

```bash
git add src/task.rs
git commit -m "feat: implement Task model with state machine and full transition rules"
```

---

### Task 4: Relative Time Parsing

**Files:**
- Create: `src/time_parse.rs`
- Test: inline `#[cfg(test)]` module

**Step 1: Write tests for time parsing**

```rust
// src/time_parse.rs

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn parse_absolute_date() {
        let dt = parse_due("2026-03-15").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn parse_relative_days() {
        let now = Utc::now();
        let dt = parse_due("3d").unwrap();
        let diff = dt.signed_duration_since(now);
        // Should be approximately 3 days (allow 1 second tolerance)
        assert!((diff.num_seconds() - 3 * 86400).abs() < 2);
    }

    #[test]
    fn parse_relative_weeks() {
        let now = Utc::now();
        let dt = parse_due("2w").unwrap();
        let diff = dt.signed_duration_since(now);
        assert!((diff.num_seconds() - 14 * 86400).abs() < 2);
    }

    #[test]
    fn parse_today() {
        let dt = parse_since("today").unwrap();
        let today = Utc::now().date_naive();
        assert_eq!(dt.date_naive(), today);
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!(parse_due("xyz").is_err());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib time_parse`
Expected: FAIL

**Step 3: Implement time parsing**

```rust
// src/time_parse.rs

use chrono::{DateTime, Duration, NaiveDate, Utc};
use crate::error::TodoError;

/// Parse due date: accepts "YYYY-MM-DD" or relative like "3d", "2w"
pub fn parse_due(input: &str) -> Result<DateTime<Utc>, TodoError> {
    // Try absolute date first
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(23, 59, 59).unwrap().and_utc());
    }

    // Try relative: Nd or Nw
    let input = input.trim();
    if let Some(num_str) = input.strip_suffix('d') {
        let n: i64 = num_str.parse().map_err(|_| TodoError::ParseError(format!("Invalid relative time: {}", input)))?;
        return Ok(Utc::now() + Duration::days(n));
    }
    if let Some(num_str) = input.strip_suffix('w') {
        let n: i64 = num_str.parse().map_err(|_| TodoError::ParseError(format!("Invalid relative time: {}", input)))?;
        return Ok(Utc::now() + Duration::weeks(n));
    }

    Err(TodoError::ParseError(format!("Cannot parse date: {}", input)))
}

/// Parse --since filter: accepts "today", "7d", "YYYY-MM-DD"
pub fn parse_since(input: &str) -> Result<DateTime<Utc>, TodoError> {
    match input {
        "today" => {
            let today = Utc::now().date_naive();
            Ok(today.and_hms_opt(0, 0, 0).unwrap().and_utc())
        }
        _ => {
            // Try relative days first
            if let Some(num_str) = input.strip_suffix('d') {
                let n: i64 = num_str.parse().map_err(|_| TodoError::ParseError(format!("Invalid relative time: {}", input)))?;
                return Ok(Utc::now() - Duration::days(n));
            }
            // Try absolute date
            if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
                return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
            }
            Err(TodoError::ParseError(format!("Cannot parse since: {}", input)))
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib time_parse`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/time_parse.rs
git commit -m "feat: add relative and absolute time parsing for due dates and filters"
```

---

### Task 5: ID Generation

**Files:**
- Create: `src/id.rs`
- Test: inline `#[cfg(test)]` module

**Step 1: Write tests**

```rust
// src/id.rs

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL);"
        ).unwrap();
        conn
    }

    #[test]
    fn generates_4_char_id_when_no_collision() {
        let conn = setup_db();
        let id = generate_id(&conn).unwrap();
        assert_eq!(id.len(), 4);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generates_longer_id_on_collision() {
        let conn = setup_db();
        let first_id = generate_id(&conn).unwrap();
        // Insert a row with the same 4-char prefix to force collision
        // This is hard to test deterministically, so we just verify
        // the generated ID is unique and valid hex
        conn.execute("INSERT INTO tasks (id, title) VALUES (?1, 'test')", [&first_id]).unwrap();
        let second_id = generate_id(&conn).unwrap();
        assert_ne!(first_id, second_id);
        assert!(second_id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib id`
Expected: FAIL

**Step 3: Implement ID generation**

```rust
// src/id.rs

use rusqlite::{Connection, params};
use crate::error::TodoError;

/// Generate a short unique ID from UUID v7.
/// Starts at 4 hex chars, expands to 6 or 8 on collision.
pub fn generate_id(conn: &Connection) -> Result<String, TodoError> {
    let uuid = uuid::Uuid::now_v7();
    let full_hex = uuid.simple().to_string();

    for len in [4, 6, 8] {
        let candidate = &full_hex[..len];
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
            params![candidate],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(candidate.to_string());
        }
    }

    // Fallback: full 32-char hex (virtually impossible)
    Ok(full_hex)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib id`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/id.rs
git commit -m "feat: add short ID generation with collision expansion"
```

---

### Task 6: Storage Layer - Schema and Migrations

**Files:**
- Create: `migrations/v1.sql`
- Create: `src/db.rs`
- Test: inline `#[cfg(test)]` module

**Step 1: Create migration SQL**

Create `migrations/v1.sql`:

```sql
CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  creator       TEXT NOT NULL DEFAULT 'human' CHECK(creator IN ('human', 'agent')),
  created_at    TEXT NOT NULL DEFAULT (datetime('now')),

  priority      TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('high', 'medium', 'low')),
  tags          TEXT DEFAULT '[]',
  parent_id     TEXT REFERENCES tasks(id),
  due           TEXT,

  status        TEXT NOT NULL DEFAULT 'pending'
                CHECK(status IN ('pending', 'in_progress', 'blocked', 'done', 'cancelled')),
  assignee      TEXT CHECK(assignee IS NULL OR assignee IN ('human', 'agent')),
  blocked_reason TEXT,

  result        TEXT,
  artifacts     TEXT DEFAULT '[]',
  log           TEXT,
  started_at    TEXT,
  finished_at   TEXT
);

CREATE INDEX idx_status ON tasks(status);
CREATE INDEX idx_priority ON tasks(priority);
CREATE INDEX idx_created ON tasks(created_at);
CREATE INDEX idx_parent ON tasks(parent_id);
```

**Step 2: Write tests for database initialization and migration**

```rust
// src/db.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_initializes_with_schema() {
        let db = Database::open_in_memory().unwrap();
        // Verify tasks table exists by inserting a minimal row
        db.conn.execute(
            "INSERT INTO tasks (id, title) VALUES ('test', 'hello')",
            [],
        ).unwrap();
        let count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM tasks", [], |row| row.get(0)
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn schema_version_is_set() {
        let db = Database::open_in_memory().unwrap();
        let version: i32 = db.conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        assert_eq!(version, 1);
    }
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --lib db`
Expected: FAIL

**Step 4: Implement Database struct with migration**

```rust
// src/db.rs

use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::error::TodoError;
use crate::task::{Task, Status, Creator, Priority};

pub struct Database {
    pub(crate) conn: Connection,
}

impl Database {
    /// Open database at the default path (~/.todo/todo.db)
    pub fn open() -> Result<Self, TodoError> {
        let db_path = Self::default_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        let db = Database { conn };
        db.init()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self, TodoError> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.init()?;
        Ok(db)
    }

    fn default_path() -> Result<PathBuf, TodoError> {
        let home = dirs::home_dir()
            .ok_or_else(|| TodoError::InvalidInput("Cannot find home directory".into()))?;
        Ok(home.join(".todo").join("todo.db"))
    }

    fn init(&self) -> Result<(), TodoError> {
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        self.run_migrations()?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), TodoError> {
        let version: i32 = self.conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version < 1 {
            self.conn.execute_batch(include_str!("../migrations/v1.sql"))?;
            self.conn.pragma_update(None, "user_version", 1)?;
        }

        Ok(())
    }
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --lib db`
Expected: All PASS

**Step 6: Commit**

```bash
git add migrations/v1.sql src/db.rs
git commit -m "feat: add SQLite database with schema migration"
```

---

### Task 7: Storage Layer - CRUD Operations

**Files:**
- Modify: `src/db.rs`
- Test: inline `#[cfg(test)]` module

**Step 1: Write tests for insert and get**

Add to `db::tests`:

```rust
    use crate::task::{Task, Creator, Priority, Status};

    fn make_task(id: &str, title: &str) -> Task {
        Task::new(id.into(), title.into())
    }

    #[test]
    fn insert_and_get_task() {
        let db = Database::open_in_memory().unwrap();
        let mut task = make_task("abcd", "Test task");
        task.tags = vec!["backend".into(), "auth".into()];
        task.priority = Priority::High;
        task.creator = Creator::Agent;

        db.insert_task(&task).unwrap();
        let loaded = db.get_task("abcd").unwrap().unwrap();

        assert_eq!(loaded.id, "abcd");
        assert_eq!(loaded.title, "Test task");
        assert_eq!(loaded.priority, Priority::High);
        assert_eq!(loaded.creator, Creator::Agent);
        assert_eq!(loaded.tags, vec!["backend", "auth"]);
        assert_eq!(loaded.status, Status::Pending);
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.get_task("nope").unwrap().is_none());
    }

    #[test]
    fn update_task() {
        let db = Database::open_in_memory().unwrap();
        let mut task = make_task("abcd", "Test");
        db.insert_task(&task).unwrap();

        task.status = Status::InProgress;
        task.assignee = Some(Creator::Agent);
        db.update_task(&task).unwrap();

        let loaded = db.get_task("abcd").unwrap().unwrap();
        assert_eq!(loaded.status, Status::InProgress);
        assert_eq!(loaded.assignee, Some(Creator::Agent));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib db`
Expected: FAIL - methods not defined

**Step 3: Implement insert, get, update**

Add to `impl Database`:

```rust
    pub fn insert_task(&self, task: &Task) -> Result<(), TodoError> {
        self.conn.execute(
            "INSERT INTO tasks (id, title, creator, created_at, priority, tags, parent_id, due, status, assignee, blocked_reason, result, artifacts, log, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                task.id,
                task.title,
                task.creator.to_string(),
                task.created_at.to_rfc3339(),
                task.priority.to_string(),
                serde_json::to_string(&task.tags).unwrap(),
                task.parent_id,
                task.due.map(|d| d.to_rfc3339()),
                task.status.to_string(),
                task.assignee.as_ref().map(|a| a.to_string()),
                task.blocked_reason,
                task.result,
                serde_json::to_string(&task.artifacts).unwrap(),
                task.log,
                task.started_at.map(|d| d.to_rfc3339()),
                task.finished_at.map(|d| d.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>, TodoError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, creator, created_at, priority, tags, parent_id, due, status, assignee, blocked_reason, result, artifacts, log, started_at, finished_at FROM tasks WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::row_to_task(row)?)),
            None => Ok(None),
        }
    }

    pub fn update_task(&self, task: &Task) -> Result<(), TodoError> {
        self.conn.execute(
            "UPDATE tasks SET title=?2, priority=?3, tags=?4, parent_id=?5, due=?6, status=?7, assignee=?8, blocked_reason=?9, result=?10, artifacts=?11, log=?12, started_at=?13, finished_at=?14 WHERE id=?1",
            params![
                task.id,
                task.title,
                task.priority.to_string(),
                serde_json::to_string(&task.tags).unwrap(),
                task.parent_id,
                task.due.map(|d| d.to_rfc3339()),
                task.status.to_string(),
                task.assignee.as_ref().map(|a| a.to_string()),
                task.blocked_reason,
                task.result,
                serde_json::to_string(&task.artifacts).unwrap(),
                task.log,
                task.started_at.map(|d| d.to_rfc3339()),
                task.finished_at.map(|d| d.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    fn row_to_task(row: &rusqlite::Row) -> Result<Task, TodoError> {
        let creator_str: String = row.get(2)?;
        let status_str: String = row.get(8)?;
        let assignee_str: Option<String> = row.get(9)?;
        let priority_str: String = row.get(4)?;
        let tags_json: String = row.get(5)?;
        let artifacts_json: String = row.get(12)?;
        let created_at_str: String = row.get(3)?;
        let due_str: Option<String> = row.get(7)?;
        let started_at_str: Option<String> = row.get(14)?;
        let finished_at_str: Option<String> = row.get(15)?;

        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            creator: creator_str.parse()?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| TodoError::ParseError(e.to_string()))?.with_timezone(&Utc),
            priority: priority_str.parse()?,
            tags: serde_json::from_str(&tags_json)
                .map_err(|e| TodoError::ParseError(e.to_string()))?,
            parent_id: row.get(6)?,
            due: due_str.map(|s| DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&Utc)))
                .transpose()
                .map_err(|e| TodoError::ParseError(e.to_string()))?,
            status: status_str.parse()?,
            assignee: assignee_str.map(|s| s.parse()).transpose()?,
            blocked_reason: row.get(10)?,
            result: row.get(11)?,
            artifacts: serde_json::from_str(&artifacts_json)
                .map_err(|e| TodoError::ParseError(e.to_string()))?,
            log: row.get(13)?,
            started_at: started_at_str.map(|s| DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&Utc)))
                .transpose()
                .map_err(|e| TodoError::ParseError(e.to_string()))?,
            finished_at: finished_at_str.map(|s| DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&Utc)))
                .transpose()
                .map_err(|e| TodoError::ParseError(e.to_string()))?,
        })
    }
```

Add `use chrono::{DateTime, Utc};` to the top of db.rs.

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib db`
Expected: All PASS

**Step 5: Write tests for list and get_next**

```rust
    #[test]
    fn list_filters_by_status() {
        let db = Database::open_in_memory().unwrap();
        db.insert_task(&make_task("a1", "Task A")).unwrap();
        let mut task_b = make_task("b2", "Task B");
        task_b.status = Status::Done;
        task_b.result = Some("done".into());
        db.insert_task(&task_b).unwrap();

        let filter = TaskFilter { status: Some(vec![Status::Pending]), ..Default::default() };
        let tasks = db.list_tasks(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "a1");
    }

    #[test]
    fn list_filters_by_tag() {
        let db = Database::open_in_memory().unwrap();
        let mut t1 = make_task("a1", "Task A");
        t1.tags = vec!["backend".into()];
        db.insert_task(&t1).unwrap();
        let mut t2 = make_task("b2", "Task B");
        t2.tags = vec!["frontend".into()];
        db.insert_task(&t2).unwrap();

        let filter = TaskFilter { tags: Some(vec!["backend".into()]), ..Default::default() };
        let tasks = db.list_tasks(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "a1");
    }

    #[test]
    fn get_next_returns_highest_priority_oldest_first() {
        let db = Database::open_in_memory().unwrap();
        let mut t1 = make_task("lo", "Low task");
        t1.priority = Priority::Low;
        let mut t2 = make_task("hi", "High task");
        t2.priority = Priority::High;

        db.insert_task(&t1).unwrap();
        db.insert_task(&t2).unwrap();

        let next = db.get_next_task(None, None).unwrap().unwrap();
        assert_eq!(next.id, "hi");
    }

    #[test]
    fn get_next_returns_none_when_empty() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.get_next_task(None, None).unwrap().is_none());
    }
```

**Step 6: Run tests to verify they fail**

Run: `cargo test --lib db`
Expected: FAIL - TaskFilter, list_tasks, get_next_task not defined

**Step 7: Implement TaskFilter, list_tasks, get_next_task**

Add to db.rs:

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
}
```

Add to `impl Database`:

```rust
    pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>, TodoError> {
        let mut sql = String::from(
            "SELECT id, title, creator, created_at, priority, tags, parent_id, due, status, assignee, blocked_reason, result, artifacts, log, started_at, finished_at FROM tasks WHERE 1=1"
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref statuses) = filter.status {
            let placeholders: Vec<String> = statuses.iter().enumerate().map(|(i, _)| {
                format!("?{}", param_idx + i)
            }).collect();
            sql.push_str(&format!(" AND status IN ({})", placeholders.join(",")));
            for s in statuses {
                param_values.push(Box::new(s.to_string()));
                param_idx += 1;
            }
        }

        if let Some(ref tags) = filter.tags {
            for tag in tags {
                sql.push_str(&format!(" AND tags LIKE ?{}", param_idx));
                param_values.push(Box::new(format!("%\"{}\"%" , tag)));
                param_idx += 1;
            }
        }

        if let Some(ref priority) = filter.priority {
            sql.push_str(&format!(" AND priority = ?{}", param_idx));
            param_values.push(Box::new(priority.to_string()));
            param_idx += 1;
        }

        if let Some(ref parent_id) = filter.parent_id {
            sql.push_str(&format!(" AND parent_id = ?{}", param_idx));
            param_values.push(Box::new(parent_id.clone()));
            param_idx += 1;
        }

        if let Some(ref creator) = filter.creator {
            sql.push_str(&format!(" AND creator = ?{}", param_idx));
            param_values.push(Box::new(creator.to_string()));
            param_idx += 1;
        }

        if let Some(ref since) = filter.since {
            sql.push_str(&format!(" AND created_at >= ?{}", param_idx));
            param_values.push(Box::new(since.to_rfc3339()));
            param_idx += 1;
        }

        // Default sort: priority DESC (high > medium > low), created_at ASC
        sql.push_str(" ORDER BY CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 WHEN 'low' THEN 2 END ASC, created_at ASC");

        let limit = filter.limit.unwrap_or(20);
        sql.push_str(&format!(" LIMIT ?{}", param_idx));
        param_values.push(Box::new(limit));

        let mut stmt = self.conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let mut rows = stmt.query(params_ref.as_slice())?;

        let mut tasks = Vec::new();
        while let Some(row) = rows.next()? {
            tasks.push(Self::row_to_task(row)?);
        }
        Ok(tasks)
    }

    pub fn get_next_task(&self, tag: Option<&str>, priority: Option<&str>) -> Result<Option<Task>, TodoError> {
        let mut filter = TaskFilter {
            status: Some(vec![Status::Pending]),
            limit: Some(1),
            ..Default::default()
        };
        if let Some(t) = tag {
            filter.tags = Some(vec![t.to_string()]);
        }
        if let Some(p) = priority {
            filter.priority = Some(p.parse()?);
        }
        let tasks = self.list_tasks(&filter)?;
        Ok(tasks.into_iter().next())
    }
```

**Step 8: Run tests to verify they pass**

Run: `cargo test --lib db`
Expected: All PASS

**Step 9: Commit**

```bash
git add src/db.rs
git commit -m "feat: add CRUD operations with filtering and priority-based next task"
```

---

### Task 8: Output Formatting

**Files:**
- Create: `src/output.rs`
- Test: inline `#[cfg(test)]` module

**Step 1: Write tests for JSON and pretty output**

```rust
// src/output.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Task, Status, Priority};

    #[test]
    fn json_output_is_valid_json() {
        let output = Output::new(false);
        let task = Task::new("abcd".into(), "Test task".into());
        let json_str = output.task(&task);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["id"], "abcd");
        assert_eq!(parsed["status"], "pending");
    }

    #[test]
    fn json_list_is_array() {
        let output = Output::new(false);
        let tasks = vec![
            Task::new("a".into(), "First".into()),
            Task::new("b".into(), "Second".into()),
        ];
        let json_str = output.task_list(&tasks);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn pretty_output_contains_status_icon() {
        let output = Output::new(true);
        let task = Task::new("abcd".into(), "Test task".into());
        let pretty = output.task(&task);
        assert!(pretty.contains("○")); // pending icon
        assert!(pretty.contains("abcd"));
        assert!(pretty.contains("Test task"));
    }

    #[test]
    fn error_output_is_json_to_stderr() {
        let err = TodoError::QueueEmpty;
        let json = output_error(&err);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"], "E_QUEUE_EMPTY");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib output`
Expected: FAIL

**Step 3: Implement output formatting**

```rust
// src/output.rs

use crate::error::TodoError;
use crate::task::{Task, Status, Priority};

pub struct Output {
    pretty: bool,
}

impl Output {
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    pub fn task(&self, task: &Task) -> String {
        if self.pretty {
            self.format_task_pretty(task)
        } else {
            serde_json::to_string_pretty(task).unwrap()
        }
    }

    pub fn task_list(&self, tasks: &[Task]) -> String {
        if self.pretty {
            tasks.iter().map(|t| self.format_task_pretty(t)).collect::<Vec<_>>().join("\n\n")
        } else {
            serde_json::to_string_pretty(tasks).unwrap()
        }
    }

    pub fn log(&self, tasks: &[Task]) -> String {
        if self.pretty {
            self.format_log_pretty(tasks)
        } else {
            serde_json::to_string_pretty(tasks).unwrap()
        }
    }

    pub fn stats(&self, stats: &serde_json::Value) -> String {
        if self.pretty {
            serde_json::to_string_pretty(stats).unwrap() // pretty JSON is fine for stats
        } else {
            serde_json::to_string_pretty(stats).unwrap()
        }
    }

    fn format_task_pretty(&self, task: &Task) -> String {
        let status_icon = match task.status {
            Status::Pending => "○",
            Status::InProgress => "◐",
            Status::Blocked => "⊘",
            Status::Done => "●",
            Status::Cancelled => "✕",
        };
        let pri = match task.priority {
            Priority::High => "!",
            Priority::Medium => "·",
            Priority::Low => "_",
        };
        let tags = if task.tags.is_empty() {
            String::new()
        } else {
            format!(" {}", task.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" "))
        };
        format!("{} {} {} [{}]{}", status_icon, pri, task.id, task.title, tags)
    }

    fn format_log_pretty(&self, tasks: &[Task]) -> String {
        if tasks.is_empty() {
            return "No completed tasks found.".to_string();
        }
        let mut lines = vec!["Completed tasks:".to_string(), "---".to_string()];
        for task in tasks {
            lines.push(format!(
                "● [{}] {}",
                task.id, task.title,
            ));
            if let Some(ref result) = task.result {
                lines.push(format!("  → {}", result));
            }
            for a in &task.artifacts {
                lines.push(format!("  📎 {}", a));
            }
        }
        lines.join("\n")
    }
}

pub fn output_error(err: &TodoError) -> String {
    serde_json::json!({
        "error": err.code(),
        "message": err.to_string()
    })
    .to_string()
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib output`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/output.rs
git commit -m "feat: add JSON and pretty output formatting"
```

---

### Task 9: CLI Argument Parsing

**Files:**
- Create: `src/cli.rs`

This task is definition-only (no complex logic to test). The integration tests in later tasks will cover CLI parsing.

**Step 1: Implement CLI argument definitions**

```rust
// src/cli.rs

use argh::FromArgs;

#[derive(FromArgs)]
/// Todo - Human-Agent Task Coordination Protocol
pub struct Args {
    #[argh(subcommand)]
    pub command: Command,

    /// output in human-readable format
    #[argh(switch, short = 'p')]
    pub pretty: bool,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Command {
    Add(AddArgs),
    Next(NextArgs),
    Start(StartArgs),
    Done(DoneArgs),
    Block(BlockArgs),
    Resume(ResumeArgs),
    Cancel(CancelArgs),
    List(ListArgs),
    Show(ShowArgs),
    Log(LogArgs),
    Stats(StatsArgs),
    Import(ImportArgs),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
/// Create a new task
pub struct AddArgs {
    /// task title
    #[argh(positional)]
    pub title: String,

    /// priority: high, medium, low
    #[argh(option, short = 'r')]
    pub pri: Option<String>,

    /// tag (can be repeated)
    #[argh(option, short = 't')]
    pub tag: Vec<String>,

    /// parent task id
    #[argh(option)]
    pub parent: Option<String>,

    /// due date (YYYY-MM-DD or relative like 3d)
    #[argh(option)]
    pub due: Option<String>,

    /// creator: human or agent
    #[argh(option)]
    pub creator: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "next")]
/// Claim the next pending task
pub struct NextArgs {
    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// filter by priority
    #[argh(option, short = 'r')]
    pub pri: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "start")]
/// Start a specific task
pub struct StartArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// assignee: human or agent
    #[argh(option)]
    pub assignee: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "done")]
/// Complete a task
pub struct DoneArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// what was done (required)
    #[argh(option, short = 'm')]
    pub result: String,

    /// artifact reference (can be repeated)
    #[argh(option)]
    pub artifact: Vec<String>,

    /// execution log/notes
    #[argh(option)]
    pub log: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "block")]
/// Block a task
pub struct BlockArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// reason for blocking (required)
    #[argh(option)]
    pub reason: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "resume")]
/// Resume a blocked task
pub struct ResumeArgs {
    /// task id
    #[argh(positional)]
    pub id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "cancel")]
/// Cancel a task
pub struct CancelArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// reason for cancellation
    #[argh(option)]
    pub reason: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List tasks with filters
pub struct ListArgs {
    /// filter by status (can be repeated)
    #[argh(option, short = 's')]
    pub status: Vec<String>,

    /// filter by tag (can be repeated)
    #[argh(option, short = 't')]
    pub tag: Vec<String>,

    /// filter by priority
    #[argh(option, short = 'r')]
    pub pri: Option<String>,

    /// filter by parent task id
    #[argh(option)]
    pub parent: Option<String>,

    /// filter by creator
    #[argh(option)]
    pub creator: Option<String>,

    /// time filter (today, 7d, YYYY-MM-DD)
    #[argh(option)]
    pub since: Option<String>,

    /// max results (default 20)
    #[argh(option)]
    pub limit: Option<i64>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "show")]
/// Show task details
pub struct ShowArgs {
    /// task id
    #[argh(positional)]
    pub id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "log")]
/// View execution log
pub struct LogArgs {
    /// show today's completed tasks
    #[argh(switch)]
    pub today: bool,

    /// time filter
    #[argh(option)]
    pub since: Option<String>,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "stats")]
/// Show task statistics
pub struct StatsArgs {
    /// time filter
    #[argh(option)]
    pub since: Option<String>,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "import")]
/// Bulk import tasks from JSON
pub struct ImportArgs {
    /// path to JSON file (or - for stdin)
    #[argh(option)]
    pub json: String,
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Successful build

**Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "feat: define CLI argument parsing with argh"
```

---

### Task 10: Command Handlers - State Machine Commands

**Files:**
- Create: `src/commands/mod.rs`
- Create: `src/commands/add.rs`
- Create: `src/commands/next.rs`
- Create: `src/commands/start.rs`
- Create: `src/commands/done.rs`
- Create: `src/commands/block.rs`
- Create: `src/commands/resume.rs`
- Create: `src/commands/cancel.rs`
- Test: `tests/commands_test.rs` (integration tests)

**Step 1: Write integration tests for add and next**

Create `tests/commands_test.rs`:

```rust
use todo::db::Database;
use todo::commands;
use todo::cli::*;
use todo::output::Output;
use todo::task::Status;

fn setup() -> (Database, Output) {
    (Database::open_in_memory().unwrap(), Output::new(false))
}

#[test]
fn add_creates_task_and_returns_json() {
    let (db, out) = setup();
    let args = AddArgs {
        title: "Test task".into(),
        pri: Some("high".into()),
        tag: vec!["backend".into()],
        parent: None,
        due: None,
        creator: None,
    };
    let result = commands::add::execute(&db, args, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["title"], "Test task");
    assert_eq!(parsed["priority"], "high");
    assert_eq!(parsed["status"], "pending");
}

#[test]
fn next_claims_highest_priority_task() {
    let (db, out) = setup();
    // Add two tasks
    commands::add::execute(&db, AddArgs {
        title: "Low".into(), pri: Some("low".into()),
        tag: vec![], parent: None, due: None, creator: None,
    }, &out).unwrap();
    commands::add::execute(&db, AddArgs {
        title: "High".into(), pri: Some("high".into()),
        tag: vec![], parent: None, due: None, creator: None,
    }, &out).unwrap();

    let result = commands::next::execute(&db, NextArgs { tag: None, pri: None }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["title"], "High");
    assert_eq!(parsed["status"], "in_progress");
}

#[test]
fn done_completes_task() {
    let (db, out) = setup();
    let add_result = commands::add::execute(&db, AddArgs {
        title: "Work".into(), pri: None, tag: vec![], parent: None, due: None, creator: None,
    }, &out).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&add_result).unwrap()["id"]
        .as_str().unwrap().into();

    commands::start::execute(&db, StartArgs { id: id.clone(), assignee: None }, &out).unwrap();
    let result = commands::done::execute(&db, DoneArgs {
        id: id.clone(),
        result: "Finished".into(),
        artifact: vec!["commit:abc".into()],
        log: None,
    }, &out).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "done");
    assert_eq!(parsed["result"], "Finished");
}

#[test]
fn block_and_resume() {
    let (db, out) = setup();
    let add_result = commands::add::execute(&db, AddArgs {
        title: "Work".into(), pri: None, tag: vec![], parent: None, due: None, creator: None,
    }, &out).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&add_result).unwrap()["id"]
        .as_str().unwrap().into();

    commands::start::execute(&db, StartArgs { id: id.clone(), assignee: None }, &out).unwrap();
    commands::block::execute(&db, BlockArgs { id: id.clone(), reason: "need key".into() }, &out).unwrap();

    let show = commands::show::execute(&db, ShowArgs { id: id.clone() }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(parsed["status"], "blocked");

    commands::resume::execute(&db, ResumeArgs { id: id.clone() }, &out).unwrap();
    let show = commands::show::execute(&db, ShowArgs { id: id.clone() }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(parsed["status"], "in_progress");
}

#[test]
fn cancel_from_pending() {
    let (db, out) = setup();
    let add_result = commands::add::execute(&db, AddArgs {
        title: "Work".into(), pri: None, tag: vec![], parent: None, due: None, creator: None,
    }, &out).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&add_result).unwrap()["id"]
        .as_str().unwrap().into();

    let result = commands::cancel::execute(&db, CancelArgs { id: id.clone(), reason: None }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "cancelled");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test commands_test`
Expected: FAIL - modules not found

**Step 3: Implement commands/mod.rs**

```rust
// src/commands/mod.rs

pub mod add;
pub mod next;
pub mod start;
pub mod done;
pub mod block;
pub mod resume;
pub mod cancel;
pub mod show;
pub mod list;
pub mod log;
pub mod stats;
pub mod import;
```

**Step 4: Implement add command**

```rust
// src/commands/add.rs

use crate::cli::AddArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::{Creator, Priority, Task};
use crate::time_parse::parse_due;

pub fn execute(db: &Database, args: AddArgs, output: &Output) -> Result<String, TodoError> {
    let priority: Priority = args.pri
        .map(|p| p.parse())
        .transpose()?
        .unwrap_or(Priority::Medium);

    let creator: Creator = args.creator
        .map(|c| c.parse())
        .transpose()?
        .unwrap_or(Creator::Human);

    let due = args.due
        .map(|d| parse_due(&d))
        .transpose()?;

    let id = generate_id(&db.conn)?;

    let mut task = Task::new(id, args.title);
    task.creator = creator;
    task.priority = priority;
    task.tags = args.tag;
    task.parent_id = args.parent;
    task.due = due;

    db.insert_task(&task)?;
    Ok(output.task(&task))
}
```

**Step 5: Implement next command**

```rust
// src/commands/next.rs

use crate::cli::NextArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Creator, Status, TransitionContext};

pub fn execute(db: &Database, args: NextArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db.get_next_task(args.tag.as_deref(), args.pri.as_deref())?
        .ok_or(TodoError::QueueEmpty)?;

    task.transition(Status::InProgress, TransitionContext {
        assignee: Some(Creator::Agent),
        result: None, artifacts: None, log: None, blocked_reason: None,
    })?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
```

**Step 6: Implement start command**

```rust
// src/commands/start.rs

use crate::cli::StartArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Creator, Status, TransitionContext};

pub fn execute(db: &Database, args: StartArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db.get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    let assignee: Creator = args.assignee
        .map(|a| a.parse())
        .transpose()?
        .unwrap_or(Creator::Human);

    task.transition(Status::InProgress, TransitionContext {
        assignee: Some(assignee),
        result: None, artifacts: None, log: None, blocked_reason: None,
    })?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
```

**Step 7: Implement done command**

```rust
// src/commands/done.rs

use crate::cli::DoneArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Status, TransitionContext};

pub fn execute(db: &Database, args: DoneArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db.get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    task.transition(Status::Done, TransitionContext {
        assignee: None,
        result: Some(args.result),
        artifacts: Some(args.artifact),
        log: args.log,
        blocked_reason: None,
    })?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
```

**Step 8: Implement block command**

```rust
// src/commands/block.rs

use crate::cli::BlockArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Status, TransitionContext};

pub fn execute(db: &Database, args: BlockArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db.get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    task.transition(Status::Blocked, TransitionContext {
        assignee: None, result: None, artifacts: None, log: None,
        blocked_reason: Some(args.reason),
    })?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
```

**Step 9: Implement resume command**

```rust
// src/commands/resume.rs

use crate::cli::ResumeArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Status, TransitionContext};

pub fn execute(db: &Database, args: ResumeArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db.get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    task.transition(Status::InProgress, TransitionContext {
        assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
    })?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
```

**Step 10: Implement cancel command**

```rust
// src/commands/cancel.rs

use crate::cli::CancelArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Status, TransitionContext};

pub fn execute(db: &Database, args: CancelArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db.get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    task.transition(Status::Cancelled, TransitionContext {
        assignee: None, result: None, artifacts: None, log: None, blocked_reason: None,
    })?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
```

**Step 11: Run tests to verify they pass**

Run: `cargo test --test commands_test`
Expected: All PASS

**Step 12: Commit**

```bash
git add src/commands/ tests/commands_test.rs
git commit -m "feat: implement state machine commands (add, next, start, done, block, resume, cancel)"
```

---

### Task 11: Command Handlers - Query Commands

**Files:**
- Create: `src/commands/show.rs`
- Create: `src/commands/list.rs`
- Create: `src/commands/log.rs`
- Create: `src/commands/stats.rs`
- Create: `src/commands/import.rs`
- Test: add to `tests/commands_test.rs`

**Step 1: Write tests for show, list, log, stats, import**

Add to `tests/commands_test.rs`:

```rust
#[test]
fn show_returns_full_task() {
    let (db, out) = setup();
    let add_result = commands::add::execute(&db, AddArgs {
        title: "Show me".into(), pri: None, tag: vec!["test".into()],
        parent: None, due: None, creator: None,
    }, &out).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&add_result).unwrap()["id"]
        .as_str().unwrap().into();

    let result = commands::show::execute(&db, ShowArgs { id }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["title"], "Show me");
    assert!(parsed["tags"].as_array().unwrap().contains(&serde_json::json!("test")));
}

#[test]
fn list_filters_by_status() {
    let (db, out) = setup();
    commands::add::execute(&db, AddArgs {
        title: "Pending".into(), pri: None, tag: vec![], parent: None, due: None, creator: None,
    }, &out).unwrap();

    let result = commands::list::execute(&db, ListArgs {
        status: vec!["pending".into()], tag: vec![], pri: None,
        parent: None, creator: None, since: None, limit: None,
    }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}

#[test]
fn log_returns_done_tasks() {
    let (db, out) = setup();
    let add_result = commands::add::execute(&db, AddArgs {
        title: "Log me".into(), pri: None, tag: vec![], parent: None, due: None, creator: None,
    }, &out).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&add_result).unwrap()["id"]
        .as_str().unwrap().into();

    commands::start::execute(&db, StartArgs { id: id.clone(), assignee: None }, &out).unwrap();
    commands::done::execute(&db, DoneArgs {
        id, result: "Did it".into(), artifact: vec![], log: None,
    }, &out).unwrap();

    let result = commands::log::execute(&db, LogArgs {
        today: true, since: None, tag: None,
    }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 1);
}

#[test]
fn stats_returns_counts() {
    let (db, out) = setup();
    commands::add::execute(&db, AddArgs {
        title: "A".into(), pri: None, tag: vec!["x".into()], parent: None, due: None, creator: None,
    }, &out).unwrap();
    commands::add::execute(&db, AddArgs {
        title: "B".into(), pri: None, tag: vec!["x".into()], parent: None, due: None, creator: Some("agent".into()),
    }, &out).unwrap();

    let result = commands::stats::execute(&db, StatsArgs { since: None, tag: None }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["total"], 2);
    assert!(parsed["by_status"]["pending"].as_i64().unwrap() >= 2);
}

#[test]
fn import_creates_tasks_from_json() {
    let (db, out) = setup();
    let json_input = r#"[{"title": "Import A", "priority": "high"}, {"title": "Import B"}]"#;
    let tmpfile = std::env::temp_dir().join("test_import.json");
    std::fs::write(&tmpfile, json_input).unwrap();

    let result = commands::import::execute(&db, ImportArgs {
        json: tmpfile.to_str().unwrap().into(),
    }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["imported"], 2);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test commands_test`
Expected: FAIL for new tests

**Step 3: Implement show command**

```rust
// src/commands/show.rs

use crate::cli::ShowArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;

pub fn execute(db: &Database, args: ShowArgs, output: &Output) -> Result<String, TodoError> {
    let task = db.get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;
    Ok(output.task(&task))
}
```

**Step 4: Implement list command**

```rust
// src/commands/list.rs

use crate::cli::ListArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::task::Status;
use crate::time_parse::parse_since;

pub fn execute(db: &Database, args: ListArgs, output: &Output) -> Result<String, TodoError> {
    let status = if args.status.is_empty() {
        None
    } else {
        Some(args.status.iter().map(|s| s.parse()).collect::<Result<Vec<Status>, _>>()?)
    };

    let tags = if args.tag.is_empty() { None } else { Some(args.tag) };

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
    };

    let tasks = db.list_tasks(&filter)?;
    Ok(output.task_list(&tasks))
}
```

**Step 5: Implement log command**

```rust
// src/commands/log.rs

use crate::cli::LogArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::task::Status;
use crate::time_parse::parse_since;

pub fn execute(db: &Database, args: LogArgs, output: &Output) -> Result<String, TodoError> {
    let since = if args.today {
        Some(parse_since("today")?)
    } else {
        args.since.map(|s| parse_since(&s)).transpose()?
    };

    let tags = args.tag.map(|t| vec![t]);

    let filter = TaskFilter {
        status: Some(vec![Status::Done]),
        tags,
        since,
        limit: Some(100),
        ..Default::default()
    };

    let tasks = db.list_tasks(&filter)?;
    Ok(output.log(&tasks))
}
```

**Step 6: Implement stats command**

```rust
// src/commands/stats.rs

use crate::cli::StatsArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::time_parse::parse_since;

pub fn execute(db: &Database, args: StatsArgs, output: &Output) -> Result<String, TodoError> {
    let since = args.since.map(|s| parse_since(&s)).transpose()?;
    let tags = args.tag.map(|t| vec![t]);

    let filter = TaskFilter {
        tags,
        since,
        limit: Some(10000),
        ..Default::default()
    };

    let tasks = db.list_tasks(&filter)?;

    let mut by_status = serde_json::Map::new();
    let mut by_creator = serde_json::Map::new();
    let mut by_tag: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut total_duration_secs: i64 = 0;
    let mut duration_count: i64 = 0;

    for task in &tasks {
        *by_status.entry(task.status.to_string())
            .or_insert(serde_json::Value::Number(0.into()))
            .as_i64().unwrap() += 1;
        // Rebuild the entry with the new count
        let status_key = task.status.to_string();
        let count = by_status.get(&status_key).and_then(|v| v.as_i64()).unwrap_or(0);
        by_status.insert(status_key, serde_json::json!(count));

        let creator_key = task.creator.to_string();
        let count = by_creator.get(&creator_key).and_then(|v| v.as_i64()).unwrap_or(0) + 1;
        by_creator.insert(creator_key, serde_json::json!(count));

        for tag in &task.tags {
            *by_tag.entry(tag.clone()).or_insert(0) += 1;
        }

        if let (Some(started), Some(finished)) = (task.started_at, task.finished_at) {
            let dur = finished.signed_duration_since(started).num_seconds();
            if dur > 0 {
                total_duration_secs += dur;
                duration_count += 1;
            }
        }
    }

    // Fix by_status counting (the above approach is messy, let's redo cleanly)
    let mut by_status = serde_json::Map::new();
    for task in &tasks {
        let key = task.status.to_string();
        let count = by_status.get(&key).and_then(|v| v.as_i64()).unwrap_or(0) + 1;
        by_status.insert(key, serde_json::json!(count));
    }

    let avg_minutes = if duration_count > 0 {
        total_duration_secs / duration_count / 60
    } else {
        0
    };

    let by_tag_json: serde_json::Map<String, serde_json::Value> = by_tag.into_iter()
        .map(|(k, v)| (k, serde_json::json!(v)))
        .collect();

    let stats = serde_json::json!({
        "total": tasks.len(),
        "by_status": by_status,
        "by_creator": by_creator,
        "avg_duration_minutes": avg_minutes,
        "by_tag": by_tag_json,
    });

    Ok(output.stats(&stats))
}
```

**Step 7: Implement import command**

```rust
// src/commands/import.rs

use crate::cli::ImportArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::{Priority, Task};
use std::io::Read;

pub fn execute(db: &Database, args: ImportArgs, output: &Output) -> Result<String, TodoError> {
    let json_str = if args.json == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)
            .map_err(|e| TodoError::Io(e))?;
        buf
    } else {
        std::fs::read_to_string(&args.json)
            .map_err(|e| TodoError::Io(e))?
    };

    let items: Vec<serde_json::Value> = serde_json::from_str(&json_str)
        .map_err(|e| TodoError::ParseError(e.to_string()))?;

    let mut count = 0;
    for item in &items {
        let title = item["title"].as_str()
            .ok_or_else(|| TodoError::InvalidInput("Each item must have a 'title' field".into()))?;

        let id = generate_id(&db.conn)?;
        let mut task = Task::new(id, title.to_string());

        if let Some(pri) = item["priority"].as_str() {
            task.priority = pri.parse()?;
        }
        if let Some(tags) = item["tags"].as_array() {
            task.tags = tags.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(creator) = item["creator"].as_str() {
            task.creator = creator.parse()?;
        }
        if let Some(parent) = item["parent_id"].as_str() {
            task.parent_id = Some(parent.to_string());
        }

        db.insert_task(&task)?;
        count += 1;
    }

    Ok(serde_json::json!({"imported": count}).to_string())
}
```

**Step 8: Run tests to verify they pass**

Run: `cargo test --test commands_test`
Expected: All PASS

**Step 9: Commit**

```bash
git add src/commands/ tests/commands_test.rs
git commit -m "feat: implement query commands (show, list, log, stats, import)"
```

---

### Task 12: Wire Up main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement main.rs with command dispatch and error handling**

```rust
// src/main.rs

use std::process;
use todo::cli::{Args, Command};
use todo::commands;
use todo::db::Database;
use todo::error::TodoError;
use todo::output::{Output, output_error};

fn main() {
    let args: Args = argh::from_env();
    let output = Output::new(args.pretty);

    let result = run(args.command, &output);

    match result {
        Ok(text) => {
            println!("{}", text);
            process::exit(0);
        }
        Err(err) => {
            eprintln!("{}", output_error(&err));
            process::exit(err.exit_code());
        }
    }
}

fn run(command: Command, output: &Output) -> Result<String, TodoError> {
    let db = Database::open()?;

    match command {
        Command::Add(args) => commands::add::execute(&db, args, output),
        Command::Next(args) => commands::next::execute(&db, args, output),
        Command::Start(args) => commands::start::execute(&db, args, output),
        Command::Done(args) => commands::done::execute(&db, args, output),
        Command::Block(args) => commands::block::execute(&db, args, output),
        Command::Resume(args) => commands::resume::execute(&db, args, output),
        Command::Cancel(args) => commands::cancel::execute(&db, args, output),
        Command::List(args) => commands::list::execute(&db, args, output),
        Command::Show(args) => commands::show::execute(&db, args, output),
        Command::Log(args) => commands::log::execute(&db, args, output),
        Command::Stats(args) => commands::stats::execute(&db, args, output),
        Command::Import(args) => commands::import::execute(&db, args, output),
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Successful build

**Step 3: Smoke test**

Run: `cargo run -- --help`
Expected: Prints help text with all subcommands listed

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up main.rs with command dispatch and error handling"
```

---

### Task 13: End-to-End CLI Integration Tests

**Files:**
- Create: `tests/cli_e2e_test.rs`

These tests exercise the actual binary to verify the full CLI → JSON output pipeline.

**Step 1: Write E2E tests**

```rust
// tests/cli_e2e_test.rs

use std::process::Command;

fn todo_cmd() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_todo"));
    // Use a temp DB to avoid polluting real data
    cmd.env("TODO_DB_PATH", "/tmp/todo_e2e_test.db");
    cmd
}

fn cleanup() {
    let _ = std::fs::remove_file("/tmp/todo_e2e_test.db");
}

#[test]
fn e2e_full_lifecycle() {
    cleanup();

    // Add a task
    let output = todo_cmd()
        .args(["add", "Test E2E task", "-r", "high", "-t", "test"])
        .output().unwrap();
    assert!(output.status.success(), "add failed: {}", String::from_utf8_lossy(&output.stderr));
    let add_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = add_json["id"].as_str().unwrap().to_string();

    // Start the task
    let output = todo_cmd()
        .args(["start", &id])
        .output().unwrap();
    assert!(output.status.success());

    // Complete the task
    let output = todo_cmd()
        .args(["done", &id, "-m", "All done"])
        .output().unwrap();
    assert!(output.status.success());
    let done_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(done_json["status"], "done");

    // Show the task
    let output = todo_cmd()
        .args(["show", &id])
        .output().unwrap();
    assert!(output.status.success());

    cleanup();
}

#[test]
fn e2e_error_on_invalid_transition() {
    cleanup();

    let output = todo_cmd()
        .args(["add", "Bad transition"])
        .output().unwrap();
    let add_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = add_json["id"].as_str().unwrap().to_string();

    // Try to complete a pending task (should fail)
    let output = todo_cmd()
        .args(["done", &id, "-m", "skip"])
        .output().unwrap();
    assert!(!output.status.success());
    let err: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(err["error"], "E_INVALID_TRANSITION");

    cleanup();
}

#[test]
fn e2e_pretty_output() {
    cleanup();

    let output = todo_cmd()
        .args(["add", "Pretty task", "-p"])
        .output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("○")); // pending icon
    assert!(stdout.contains("Pretty task"));

    cleanup();
}
```

**Step 2: Support TODO_DB_PATH environment variable**

Modify `Database::default_path()` in `src/db.rs` to check for env var:

```rust
    fn default_path() -> Result<PathBuf, TodoError> {
        if let Ok(path) = std::env::var("TODO_DB_PATH") {
            return Ok(PathBuf::from(path));
        }
        let home = dirs::home_dir()
            .ok_or_else(|| TodoError::InvalidInput("Cannot find home directory".into()))?;
        Ok(home.join(".todo").join("todo.db"))
    }
```

**Step 3: Run E2E tests**

Run: `cargo test --test cli_e2e_test`
Expected: All PASS

**Step 4: Commit**

```bash
git add tests/cli_e2e_test.rs src/db.rs
git commit -m "feat: add E2E CLI tests with TODO_DB_PATH env override"
```

---

### Task 14: Agent Integration Description

**Files:**
- Create: `docs/agent-integration.md`

**Step 1: Write the agent integration description**

This is the Markdown text that can be pasted into an agent's system prompt, as specified in the PRD.

```markdown
# Todo CLI - Agent Integration

You can use the `todo` CLI tool to manage tasks. All commands output JSON by default.

## Core Workflow

```bash
# Claim the next task
todo next --tag=<optional-filter>

# Execute the task...

# Report completion
todo done <id> --result="What you did" --artifact="commit:abc123"
```

## Commands

| Command | Purpose |
|---------|---------|
| `todo next [--tag=X]` | Claim next pending task (auto-assigns to agent) |
| `todo start <id>` | Start a specific task |
| `todo done <id> -m "result"` | Complete a task (result required) |
| `todo block <id> --reason="..."` | Mark task as blocked |
| `todo resume <id>` | Resume a blocked task |
| `todo cancel <id>` | Cancel a task |
| `todo add "title" [--creator=agent]` | Create a new task |
| `todo list [--status=pending]` | List tasks with filters |
| `todo show <id>` | View task details |

## Error Handling

Errors are JSON on stderr: `{"error": "E_QUEUE_EMPTY", "message": "..."}`

Common codes: `E_QUEUE_EMPTY`, `E_TASK_NOT_FOUND`, `E_INVALID_TRANSITION`

## Best Practices

1. Always check `todo next` result before working
2. Use `--creator=agent` when creating subtasks
3. Fill `--result` with meaningful description (not just "done")
4. Use `--artifact` to link commits, PRs, file paths
5. Use `todo block` when you need human input
```

**Step 2: Commit**

```bash
git add docs/agent-integration.md
git commit -m "docs: add agent integration description for system prompts"
```

---

### Task 15: Final Polish and Full Test Suite

**Files:**
- All test files

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy -- -W clippy::all`
Expected: No warnings (fix any that appear)

**Step 3: Build release binary**

Run: `cargo build --release`
Expected: Binary at `target/release/todo`

**Step 4: Manual smoke test**

```bash
export TODO_DB_PATH=/tmp/todo_smoke.db
./target/release/todo add "First task" -r high -t backend
./target/release/todo add "Second task" -t docs
./target/release/todo list -p
./target/release/todo next
./target/release/todo list -s in_progress -p
./target/release/todo stats
rm /tmp/todo_smoke.db
```

Expected: All commands produce correct output, JSON by default, pretty with `-p`.

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: polish and verify full test suite"
```

---

## Summary

| Task | Description | Key Files |
|------|-------------|-----------|
| 1 | Project scaffolding | Cargo.toml, src/main.rs, src/lib.rs |
| 2 | Error types | src/error.rs |
| 3 | Task model + state machine | src/task.rs |
| 4 | Time parsing | src/time_parse.rs |
| 5 | ID generation | src/id.rs |
| 6 | Storage - schema + migration | migrations/v1.sql, src/db.rs |
| 7 | Storage - CRUD operations | src/db.rs |
| 8 | Output formatting | src/output.rs |
| 9 | CLI argument parsing | src/cli.rs |
| 10 | State machine commands | src/commands/{add,next,start,done,block,resume,cancel}.rs |
| 11 | Query commands | src/commands/{show,list,log,stats,import}.rs |
| 12 | Wire up main.rs | src/main.rs |
| 13 | E2E integration tests | tests/cli_e2e_test.rs |
| 14 | Agent integration docs | docs/agent-integration.md |
| 15 | Final polish | All files |
