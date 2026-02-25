# Task Content Field Design

> **For Claude:** REQUIRED SUB-SKILL: Use summ:executing-plans or summ:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add a `content` field to tasks for storing detailed descriptions/notes.

**Architecture:** Extend Task struct with optional `content` field, add `-d/--description` CLI flag for add/edit commands, display content only in `show` command output.

**Tech Stack:** Rust, SQLite, argh (CLI), toon-format (output)

---

## Data Model

### Task Struct Change

```rust
// src/task.rs
#[serde(skip_serializing_if = "Option::is_none")]
pub content: Option<String>,
```

- `Option<String>` allows empty content (not every task needs details)
- `skip_serializing_if` hides empty field in TOON/JSON output, saving tokens
- Plain text storage, no format parsing

### Database Migration

```sql
-- New column
ALTER TABLE tasks ADD COLUMN content TEXT;
```

- SQLite `TEXT` type can store multi-line content directly
- Migration should handle existing data (new column defaults to NULL)

---

## CLI Interface

### Add Command

```bash
# Short form
todo add "Title" -d "Detailed content"

# Long form
todo add "Title" --description "Detailed content"

# Multi-line content (shell-level support)
todo add "Title" -d "Line 1
Line 2"

todo add "Title" -d "$(cat notes.md)"
```

### Edit Command

```bash
# Modify content
todo edit <id> -d "New content"

# Clear content
todo edit <id> --clear-content
```

### CLI Argument Definition

```rust
// Add command
#[argh(option, short = 'd', long = "description")]
pub description: Option<String>,

// Edit command
#[argh(option, short = 'd', long = "description")]
pub description: Option<String>,

#[argh(switch, long = "clear-content")]
pub clear_content: bool,
```

---

## Output Display

### `list` Command - No Content

```
○ · abc12345 [Task Title] #tag
```

- Keep existing format, unchanged
- TOON/JSON output hides content field if empty

### `show` Command - With Content

**TOON format:**
```
id: "abc12345"
title: Task Title
content: |
  Line 1 of content
  Line 2 of content
  Line 3 of content
status: pending
```

**Pretty format:**
```
○ · abc12345 [Task Title] #tag

详细内容:
  Line 1 of content
  Line 2 of content
  Line 3 of content
```

- Pretty mode: if content exists, show it indented below title
- If content is empty, don't show "详细内容" section

---

## Implementation Files

| File | Changes |
|------|---------|
| `src/task.rs` | Add `content: Option<String>` field |
| `src/db.rs` | Migration for content column; CRUD support |
| `src/cli.rs` | Add `-d/--description` and `--clear-content` args |
| `src/commands/add.rs` | Handle description parameter |
| `src/commands/edit.rs` | Handle description/clear_content parameters |
| `src/commands/show.rs` | Pretty mode display content |
| `src/output.rs` | Pretty output handle content display |

---

## Test Cases

- Add task with content
- Add task without content
- Edit content
- Clear content
- Verify `show` displays content
- Verify `list` does not display content
- Verify TOON output hides empty content
