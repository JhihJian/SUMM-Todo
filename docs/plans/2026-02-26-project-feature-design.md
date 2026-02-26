# Project Feature Design

## Overview

Add project support to SUMM-Todo, allowing tasks to be organized under projects. Each task must belong to exactly one project.

## Requirements Summary

- Tasks and projects: one-to-one relationship
- Project attributes: name + description
- Tasks must have a project (no default/inbox)
- Project management: manual with full commands
- Delete project: must clear tasks first
- Add task syntax: `todo add <project_name>: "task"`
- List command: group by project by default
- Project show: name + description + statistics + recent tasks

## Data Model

### Project Structure

```rust
pub struct Project {
    pub id: String,           // UUID v7
    pub name: String,         // Project name, unique
    pub description: Option<String>,  // Project description
    pub created_at: DateTime<Utc>,
}
```

### Task Changes

```rust
pub struct Task {
    // ... existing fields ...
    pub project_id: String,   // NEW: Required project reference
}
```

### Database Schema

**New table: projects**
```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL
);
```

**tasks table changes**
```sql
ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id);
CREATE INDEX idx_tasks_project_id ON tasks(project_id);
```

## Commands

### Project Management

```
todo project add <name> [-d <description>]   # Create project
todo project edit <name> [-n <new_name>] [-d <description>]  # Edit project
todo project list                            # List all projects
todo project show <name>                     # Show project details + stats + recent tasks
todo project delete <name>                   # Delete project (must clear tasks first)
```

### Task Commands Changes

```
todo add <project_name>: <title> [options]   # Add task, must specify project
todo list                                    # Show tasks grouped by project
todo list -p <project_name>                  # Filter by specific project
```

### Add Command Parsing

- Detect `:` separator in title
- Part before `:` is project name, after is task title
- Error if project doesn't exist
- Error if no project specified

## Output Formats

### `todo project list`

```
id: "01abc"
name: SUMM-Todo
tasks: 12 (3 pending, 2 in_progress, 7 done)

id: "01def"
name: my-website
tasks: 5 (1 pending, 4 done)
```

### `todo project show <name>`

```
name: SUMM-Todo
description: CLI tool for Human-Agent task coordination
created: 2025-01-15

statistics:
  total: 12
  pending: 3
  in_progress: 2
  blocked: 0
  done: 7

recent tasks:
  ○ ! 019c [Implement auth] #backend
  ● 01ab [Fix login bug] #urgent
  ✓ 01ef [Update docs]
```

### `todo list` (grouped by project)

```
=== SUMM-Todo (3) ===
○ ! 019c [Implement auth] #backend
○ 01de [Add tests] #testing
● 01ab [Fix bug]

=== my-website (2) ===
○ 01gh [Update homepage]
✓ 01ij [Deploy to production]
```

## Database Migration (v3.sql)

```sql
-- 1. Create projects table
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL
);

-- 2. Add project_id to tasks (nullable initially for backward compatibility)
ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id);

-- 3. Create index for faster queries
CREATE INDEX idx_tasks_project_id ON tasks(project_id);
```

### Migration Strategy

- Existing tasks need manual project assignment via `todo edit <id> -p <project>`
- Or use migration command: `todo migrate --assign-projects` for interactive assignment
- Future version can make `project_id` NOT NULL

## Implementation Plan

### Phase 1: Data Layer

1. Add `Project` struct to `task.rs`
2. Add `project_id` field to `Task` struct
3. Create `migrations/v3.sql`
4. Add project CRUD methods to `db.rs`:
   - `insert_project(&self, project: &Project)`
   - `get_project(&self, id: &str) -> Option<Project>`
   - `get_project_by_name(&self, name: &str) -> Option<Project>`
   - `list_projects(&self) -> Vec<Project>`
   - `update_project(&self, project: &Project)`
   - `delete_project(&self, id: &str)`
   - `count_project_tasks(&self, project_id: &str) -> ProjectStats`

### Phase 2: Command Layer

1. Create `src/commands/project.rs` for project subcommands
2. Modify `add.rs`: parse `<project>:` prefix, validate project exists
3. Modify `list.rs`: support project grouping and `-p` filter
4. Modify `edit.rs`: support changing task's project (`-p` option)
5. Add `project_id` to `TaskFilter`

### Phase 3: CLI Layer

1. Modify `cli.rs`: add `Project` subcommand enum
2. Add `-p/--project` filter argument to list command
3. Update help text and examples

### Phase 4: Output

1. Update `output.rs` for project-related formatting
2. Add project grouping to list output
3. Add project statistics formatting

### Phase 5: Testing

1. Unit tests: Project CRUD, Task-Project association
2. Integration tests: Complete workflow
3. Test migration path for existing data

## Open Questions

None - all requirements have been clarified.
