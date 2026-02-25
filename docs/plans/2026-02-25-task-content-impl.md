# Task Content Field Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use summ:executing-plans or summ:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add a `content` field to tasks for storing detailed descriptions/notes.

**Architecture:** Extend Task struct with optional `content` field, add `-d/--description` CLI flag for add/edit commands, display content only in `show` command pretty output.

**Tech Stack:** Rust, SQLite, argh (CLI), toon-format (output), serde (serialization)

---

## Task 1: Add content field to Task struct

**Files:**
- Modify: `src/task.rs:129-157`
- Test: `src/task.rs` (inline tests)

**Step 1: Add content field to Task struct**

```rust
// In src/task.rs, add after the title field (around line 132):
#[serde(skip_serializing_if = "Option::is_none")]
pub content: Option<String>,
```

**Step 2: Update Task::new() to initialize content**

```rust
// In Task::new() method, add after title initialization:
content: None,
```

**Step 3: Update new_task_has_correct_defaults test**

```rust
// In tests::new_task_has_correct_defaults, add:
assert!(task.content.is_none());
```

**Step 4: Run tests to verify**

Run: `cargo test new_task_has_correct_defaults`
Expected: PASS

**Step 5: Commit**

```bash
git add src/task.rs
git commit -m "feat(task): add optional content field"
```

---

## Task 2: Add database migration for content column

**Files:**
- Create: `migrations/v2.sql`
- Modify: `src/db.rs:79-92`

**Step 1: Create migration file**

```sql
-- migrations/v2.sql
ALTER TABLE tasks ADD COLUMN content TEXT;
```

**Step 2: Update run_migrations to include v2**

```rust
// In src/db.rs, update run_migrations:
fn run_migrations(&mut self) -> Result<(), TodoError> {
    let version: i32 =
        self.conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))?;

    if version < 1 {
        let sql = include_str!("../migrations/v1.sql");
        self.conn.execute_batch(sql)?;
        self.conn.execute_batch("PRAGMA user_version = 1;")?;
    }

    if version < 2 {
        let sql = include_str!("../migrations/v2.sql");
        self.conn.execute_batch(sql)?;
        self.conn.execute_batch("PRAGMA user_version = 2;")?;
    }

    Ok(())
}
```

**Step 3: Update insert_task to include content**

```rust
// In src/db.rs insert_task, update the INSERT statement:
// Add content to column list and params
// The params array should include task.content at the end

// Updated INSERT:
"INSERT INTO tasks (
    id, title, creator, created_at,
    priority, tags, parent_id, due,
    status, assignee, blocked_reason,
    result, artifacts, log,
    started_at, finished_at, content
) VALUES (
    ?1, ?2, ?3, ?4,
    ?5, ?6, ?7, ?8,
    ?9, ?10, ?11,
    ?12, ?13, ?14,
    ?15, ?16, ?17
)"

// Add to params:
task.content,
```

**Step 4: Update update_task to include content**

```rust
// In src/db.rs update_task, update the UPDATE statement:
// Add content = ?17 to SET clause
// Add task.content to params

// Updated UPDATE:
"UPDATE tasks SET
    title = ?2, creator = ?3, created_at = ?4,
    priority = ?5, tags = ?6, parent_id = ?7, due = ?8,
    status = ?9, assignee = ?10, blocked_reason = ?11,
    result = ?12, artifacts = ?13, log = ?14,
    started_at = ?15, finished_at = ?16, content = ?17
 WHERE id = ?1"
```

**Step 5: Update SELECT queries to include content**

```rust
// In get_task, list_tasks, search_tasks - add content to SELECT:
"SELECT id, title, creator, created_at,
        priority, tags, parent_id, due,
        status, assignee, blocked_reason,
        result, artifacts, log,
        started_at, finished_at, content
 FROM tasks WHERE id = ?1"
```

**Step 6: Update row_to_task to parse content**

```rust
// In row_to_task function, add content field:
// content is at index 16 (after finished_at)
content: row.get(16)?,
```

**Step 7: Update schema_version_is_set test**

```rust
// Update expected version to 2:
assert_eq!(version, 2);
```

**Step 8: Run tests to verify**

Run: `cargo test`
Expected: All tests pass

**Step 9: Commit**

```bash
git add migrations/v2.sql src/db.rs
git commit -m "feat(db): add content column migration"
```

---

## Task 3: Add CLI arguments for description

**Files:**
- Modify: `src/cli.rs:44-71` (AddArgs)
- Modify: `src/cli.rs:257-280` (EditArgs)

**Step 1: Add description argument to AddArgs**

```rust
// In src/cli.rs AddArgs struct, add after due field:
/// detailed description (supports multi-line)
#[argh(option, short = 'd', long = "description")]
pub description: Option<String>,
```

**Step 2: Add description and clear_content arguments to EditArgs**

```rust
// In src/cli.rs EditArgs struct, add after due field:
/// new description
#[argh(option, short = 'd', long = "description")]
pub description: Option<String>,

/// clear the description
#[argh(switch, long = "clear-content")]
pub clear_content: bool,
```

**Step 3: Run build to verify**

Run: `cargo build`
Expected: No errors

**Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add -d/--description and --clear-content args"
```

---

## Task 4: Update add command to handle description

**Files:**
- Modify: `src/commands/add.rs`

**Step 1: Update add execute to handle description**

```rust
// In src/commands/add.rs, add after due assignment:
task.content = args.description;
```

**Step 2: Run tests to verify**

Run: `cargo test`
Expected: All tests pass

**Step 3: Manual test**

```bash
cargo run -- add "Test task" -d "This is a detailed description"
cargo run -- show <id>
```

Expected: Task shows with content field

**Step 4: Commit**

```bash
git add src/commands/add.rs
git commit -m "feat(add): support -d/--description for task content"
```

---

## Task 5: Update edit command to handle description/clear_content

**Files:**
- Modify: `src/commands/edit.rs`

**Step 1: Update edit execute to handle description**

```rust
// In src/commands/edit.rs, add after due handling:
if args.clear_content {
    task.content = None;
} else if let Some(content) = args.description {
    task.content = Some(content);
}
```

**Step 2: Run tests to verify**

Run: `cargo test`
Expected: All tests pass

**Step 3: Manual test**

```bash
# Set content
cargo run -- edit <id> -d "Updated description"

# Clear content
cargo run -- edit <id> --clear-content
```

**Step 4: Commit**

```bash
git add src/commands/edit.rs
git commit -m "feat(edit): support -d/--description and --clear-content"
```

---

## Task 6: Update output to show content in show command (pretty mode)

**Files:**
- Modify: `src/output.rs:109-158`

**Step 1: Update pretty_task to show content**

```rust
// In src/output.rs pretty_task function, add at the end (before returning line):
// Show content if present (only in show context, not list)
if let Some(ref content) = task.content {
    if !content.is_empty() {
        line.push_str(&format!("\n\n详细内容:\n"));
        for content_line in content.lines() {
            line.push_str(&format!("  {}\n", content_line));
        }
        // Remove trailing newline
        line.pop();
    }
}
```

**Step 2: Run tests to verify**

Run: `cargo test`
Expected: All tests pass

**Step 3: Manual test**

```bash
# Create task with content
cargo run -- add "Task with content" -d "Line 1
Line 2
Line 3"

# Show in pretty mode
cargo run -- -p show <id>
```

Expected:
```
○ · <id> [Task with content]

详细内容:
  Line 1
  Line 2
  Line 3
```

**Step 4: Commit**

```bash
git add src/output.rs
git commit -m "feat(output): show content in pretty mode"
```

---

## Task 7: Add integration tests for content field

**Files:**
- Modify: `tests/commands_test.rs`

**Step 1: Add test for add with description**

```rust
#[test]
fn add_with_description() {
    let db = Database::open_in_memory().unwrap();
    let output = Output::json();

    let args = AddArgs {
        title: "Task with content".into(),
        pri: None,
        tag: vec![],
        parent: None,
        due: None,
        creator: None,
        description: Some("Detailed description here".into()),
    };

    let result = add::execute(&db, args, &output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["content"], "Detailed description here");
}
```

**Step 2: Add test for edit description**

```rust
#[test]
fn edit_description() {
    let db = Database::open_in_memory().unwrap();
    let output = Output::json();

    // Create task
    let add_args = AddArgs {
        title: "Test".into(),
        pri: None,
        tag: vec![],
        parent: None,
        due: None,
        creator: None,
        description: None,
    };
    let result = add::execute(&db, add_args, &output).unwrap();
    let task: serde_json::Value = serde_json::from_str(&result).unwrap();
    let id = task["id"].as_str().unwrap().to_string();

    // Edit with description
    let edit_args = EditArgs {
        id: id.clone(),
        title: None,
        priority: None,
        tag: vec![],
        due: None,
        description: Some("New description".into()),
        clear_content: false,
    };
    edit::execute(&db, edit_args, &output).unwrap();

    // Verify
    let task = db.get_task(&id).unwrap().unwrap();
    assert_eq!(task.content, Some("New description".into()));
}
```

**Step 3: Add test for clear content**

```rust
#[test]
fn clear_content() {
    let db = Database::open_in_memory().unwrap();
    let output = Output::json();

    // Create task with content
    let add_args = AddArgs {
        title: "Test".into(),
        pri: None,
        tag: vec![],
        parent: None,
        due: None,
        creator: None,
        description: Some("Initial content".into()),
    };
    let result = add::execute(&db, add_args, &output).unwrap();
    let task: serde_json::Value = serde_json::from_str(&result).unwrap();
    let id = task["id"].as_str().unwrap().to_string();

    // Clear content
    let edit_args = EditArgs {
        id: id.clone(),
        title: None,
        priority: None,
        tag: vec![],
        due: None,
        description: None,
        clear_content: true,
    };
    edit::execute(&db, edit_args, &output).unwrap();

    // Verify
    let task = db.get_task(&id).unwrap().unwrap();
    assert!(task.content.is_none());
}
```

**Step 4: Run tests to verify**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add tests/commands_test.rs
git commit -m "test: add integration tests for content field"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add content field to Task struct | `src/task.rs` |
| 2 | Add database migration | `migrations/v2.sql`, `src/db.rs` |
| 3 | Add CLI arguments | `src/cli.rs` |
| 4 | Update add command | `src/commands/add.rs` |
| 5 | Update edit command | `src/commands/edit.rs` |
| 6 | Update pretty output | `src/output.rs` |
| 7 | Add integration tests | `tests/commands_test.rs` |
