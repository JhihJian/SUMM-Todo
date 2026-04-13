# Multi-Device Sync Design

## Overview

Add multi-device synchronization to SUMM-Todo via a self-hosted Rust sync server. Both the CLI and server share a Cargo workspace with a common `core` crate for Task models and serialization.

## Context

- **Use case**: Personal multi-device sync (one user, multiple machines)
- **Network**: Stable network assumed
- **Transport**: Plain HTTP over trusted network or behind a reverse proxy handling TLS
- **Conflicts**: Rare (same task rarely edited concurrently on two devices)
- **Infrastructure**: Self-hosted single binary + SQLite

## Architecture

```
Device A                         Sync Server (Rust)                  Device B
+----------+    HTTP/JSON      +------------------+    HTTP/JSON    +----------+
| todo CLI  | <--------------> |  summ-sync       | <------------> | todo CLI  |
| todo.db   |    REST API      |  sync.db         |    REST API    | todo.db   |
+----------+                   +------------------+                 +----------+
                                (single binary + SQLite)
```

### Workspace Structure

```
SUMM-Todo/
+-- Cargo.toml                    # [workspace]
+-- crates/
    +-- core/                     # Shared library
    |   +-- Cargo.toml
    |   +-- src/
    |       +-- lib.rs
    |       +-- task.rs           # Task model (from src/task.rs)
    |       +-- error.rs          # Error types (from src/error.rs)
    |       +-- id.rs             # ID generation (from src/id.rs)
    +-- cli/                      # CLI tool
    |   +-- Cargo.toml
    |   +-- src/
    |       +-- main.rs
    |       +-- lib.rs
    |       +-- cli.rs
    |       +-- db.rs             # Local database layer
    |       +-- output.rs         # Output formatting (stays in CLI, depends on core types)
    |       +-- time_parse.rs
    |       +-- commands/
    |       |   +-- mod.rs
    |       |   +-- ... (existing commands)
    |       |   +-- sync.rs       # NEW: sync command
    |       +-- sync/             # NEW: sync module
    |           +-- mod.rs
    |           +-- client.rs     # HTTP client
    |           +-- tracker.rs    # Change tracking (manages sync_log, queries pending changes)
    +-- server/                   # Sync server
        +-- Cargo.toml
        +-- src/
            +-- main.rs           # Server entry point
            +-- config.rs         # Configuration
            +-- db.rs             # Server-side database
            +-- handlers.rs       # API handlers
            +-- router.rs         # Route definitions
```

Design decision: `output.rs` stays in `crates/cli/` because it imports `Database`, `ProjectStats`, and other CLI-specific types. Only `task.rs`, `error.rs`, and `id.rs` move to `core` — these have no CLI-specific dependencies.

### Crate Dependencies

```
core <- cli
core <- server
cli  (reqwest for HTTP client)
server (axum + tokio for HTTP server)
```

## Database Migration v5

This migration is the foundation for the sync feature. It must be applied before any sync code runs.

```sql
-- Add updated_at column to tasks table (backfill with created_at)
ALTER TABLE tasks ADD COLUMN updated_at TEXT;
UPDATE tasks SET updated_at = created_at WHERE updated_at IS NULL;

-- Add updated_at column to projects table (backfill with created_at)
ALTER TABLE projects ADD COLUMN updated_at TEXT;
UPDATE projects SET updated_at = created_at WHERE updated_at IS NULL;

-- Sync configuration
CREATE TABLE IF NOT EXISTS sync_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Change tracking log for push (generalized: supports tasks, projects, and future entities)
CREATE TABLE IF NOT EXISTS sync_log (
    entity_type TEXT NOT NULL,   -- 'task' | 'project'
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,        -- 'upsert' | 'delete'
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id, recorded_at)
);

-- Trigger: track INSERTs on tasks
CREATE TRIGGER IF NOT EXISTS sync_track_task_insert
AFTER INSERT ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', NEW.id, 'upsert', datetime('now'));
END;

-- Trigger: track UPDATEs on tasks
CREATE TRIGGER IF NOT EXISTS sync_track_task_update
AFTER UPDATE ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', NEW.id, 'upsert', datetime('now'));
END;

-- Trigger: track DELETEs on tasks
CREATE TRIGGER IF NOT EXISTS sync_track_task_delete
AFTER DELETE ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', OLD.id, 'delete', datetime('now'));
END;

-- Trigger: track INSERTs on projects
CREATE TRIGGER IF NOT EXISTS sync_track_project_insert
AFTER INSERT ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', NEW.id, 'upsert', datetime('now'));
END;

-- Trigger: track UPDATEs on projects
CREATE TRIGGER IF NOT EXISTS sync_track_project_update
AFTER UPDATE ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', NEW.id, 'upsert', datetime('now'));
END;

-- Trigger: track DELETEs on projects
CREATE TRIGGER IF NOT EXISTS sync_track_project_delete
AFTER DELETE ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', OLD.id, 'delete', datetime('now'));
END;
```

The `updated_at` column on both `tasks` and `projects` tables must be updated on every mutation. All existing command handlers that modify tasks or projects will set `updated_at = datetime('now')` as part of their UPDATE statements.

The `sync_log` uses a generalized schema with `entity_type` discriminator (`'task'` or `'project'`). This supports both entity types in a single table and is extensible for future entities without schema changes.

## Sync Protocol

### Authentication

API Key authentication:
- Server started with a configured API key (`--key` flag or `SYNC_API_KEY` env var)
- CLI stores `server_url` + `api_key` in local `sync_config` table
- Every request includes header: `Authorization: Bearer <api-key>`

### API Endpoints

```
POST   /api/v1/sync/push       Push local changes to server
POST   /api/v1/sync/pull       Pull server changes to local
GET    /api/v1/sync/status     Query sync status
```

### Error Response Format

All error responses use a consistent JSON structure:

```json
{
  "ok": false,
  "error": {
    "code": "AUTH_FAILED",
    "message": "Invalid API key"
  }
}
```

Error codes:
- `AUTH_FAILED` -- Invalid or missing API key
- `BAD_REQUEST` -- Malformed request body
- `CONFLICT` -- Sync conflicts detected (returned alongside conflict details)
- `INTERNAL` -- Server internal error

### POST /api/v1/sync/push

**Request:**

```json
{
  "device_id": "device-abc123",
  "tasks": [
    {
      "id": "abc12345",
      "title": "Complete report",
      "content": null,
      "project_id": null,
      "creator": "human",
      "created_at": "2024-03-01T08:00:00Z",
      "priority": "medium",
      "tags": [],
      "parent_id": null,
      "due": null,
      "status": "in_progress",
      "assignee": null,
      "blocked_reason": null,
      "result": null,
      "artifacts": [],
      "log": null,
      "started_at": "2024-03-01T10:00:00Z",
      "finished_at": null,
      "updated_at": "2024-03-01T10:30:00Z"
    }
  ],
  "deleted_ids": ["xyz789"],
  "projects": [
    {
      "id": "proj123",
      "name": "my-project",
      "description": "...",
      "path": "/path/to/project",
      "created_at": "2024-03-01T08:00:00Z",
      "updated_at": "2024-03-01T09:00:00Z"
    }
  ],
  "deleted_project_ids": ["old-proj-id"]
}
```

Each task in the `tasks` array contains the **complete Task JSON** (all fields). The server stores the entire blob without parsing individual fields.

**Response (success):**

```json
{
  "ok": true,
  "conflicts": []
}
```

**Response (conflicts detected):**

```json
{
  "ok": true,
  "conflicts": [
    {
      "id": "abc12345",
      "client_updated_at": "2024-03-01T09:00:00Z",
      "server_updated_at": "2024-03-01T10:00:00Z",
      "server_data": { }
    }
  ]
}
```

Server logic: For each task, compare `updated_at`. If client timestamp >= server timestamp, overwrite. Otherwise, server version is kept and the task is added to the conflicts list. Conflicts do not block the push — other tasks are still accepted.

### POST /api/v1/sync/pull

**Request:**

```json
{
  "device_id": "device-abc123",
  "since": "2024-03-01T08:00:00Z"
}
```

The client sends the `last_sync_at` timestamp from its `sync_config` table as the `since` value. After a successful pull, the client updates `last_sync_at` to the `server_time` value from the response.

**Response:**

```json
{
  "tasks": [
    {
      "id": "def67890",
      "title": "...",
      "...": "full task fields"
    }
  ],
  "deleted_ids": ["old-task-id"],
  "projects": [
    {
      "id": "proj123",
      "name": "my-project",
      "description": "...",
      "path": "/path/to/project",
      "created_at": "2024-03-01T08:00:00Z",
      "updated_at": "2024-03-01T09:00:00Z"
    }
  ],
  "deleted_project_ids": ["old-proj-id"],
  "server_time": "2024-03-01T10:35:00Z"
}
```

The `tasks` array contains complete Task JSON for all tasks updated since the `since` timestamp. The `deleted_ids` array contains IDs of tasks that were soft-deleted on the server since `since`. The `projects` array contains complete Project JSON for all projects updated since `since`. The `deleted_project_ids` array contains IDs of soft-deleted projects. The `server_time` is the server's current timestamp — the client stores this as its new `last_sync_at`.

### GET /api/v1/sync/status

**Response:**

```json
{
  "total_tasks": 42,
  "last_modified": "2024-03-01T10:30:00Z",
  "devices": [
    {"device_id": "device-abc123", "last_sync": "2024-03-01T10:30:00Z"}
  ]
}
```

## CLI Changes

### New Commands

```
todo sync init --server <url> --key <api-key>   Initialize sync config
todo sync                                        Full sync (pull then push)
todo sync push                                   Push only
todo sync pull                                   Pull only
todo sync status                                 Show sync status
```

### Change Tracking (Zero-Intrusion)

Uses SQLite triggers on the `tasks` and `projects` tables (defined in v5 migration) to record changes into `sync_log` automatically. No modifications to existing command code required — only the SQL statements they execute need to include `updated_at = datetime('now')` in UPDATE clauses.

The `tracker.rs` module is responsible for:
1. Querying `sync_log` for pending changes (filtered by `entity_type`)
2. Resolving the latest action per (entity_type, entity_id) pair (an entity may appear multiple times in sync_log)
3. Fetching full entity data for 'upsert' actions
4. Clearing `sync_log` entries after successful push

### sync init Flow

1. Check if `sync_config` already has a `device_id` — if yes, reuse it; if no, generate a new one (UUID v7)
2. Save `server_url` + `api_key` to `sync_config`
3. Create `sync_log` table and triggers (v5 migration, idempotent)
4. Validate connectivity via `GET /sync/status`
5. **Initial sync**: Pull first (to get any existing server data), then push all local tasks using LWW logic. This handles the case where the server already has tasks from another device.

### Device ID Management

Running `sync init` multiple times (e.g., to change the server URL) preserves the existing `device_id`. Only `server_url` and `api_key` are updated. To generate a new device identity, the user must explicitly clear sync config.

### Pull Import Logic

When importing pulled data into the local `todo.db`:
1. For each task, use `INSERT OR REPLACE` (upsert by ID)
2. Skip tasks where local `updated_at` > server `updated_at` (local version is newer)
3. For each `deleted_id`, delete the task from local `todo.db` if it exists and its local `updated_at` <= server's recorded deletion time
4. Same logic applies to projects: upsert by ID, skip if local is newer, delete if in `deleted_project_ids`

### Deleted Tasks Handling

The current CLI has no hard-delete command — tasks transition to `cancelled` status. The `deleted_ids` field in the sync protocol is reserved for a potential future `task delete` command. For now, the `sync_log` trigger on DELETE will only fire if such a command is added later. Cancelled tasks sync normally as status changes.

## Server Design

### Technology

| Component | Choice | Reason |
|-----------|--------|--------|
| HTTP framework | `axum` | Mainstream async Rust web framework |
| Database | `rusqlite` | Consistent with CLI |
| Serialization | `serde` + `serde_json` | Already in use |
| Async runtime | `tokio` | Required by axum |

### Server Schema

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    data TEXT NOT NULL,           -- Full Task JSON blob
    updated_at TEXT NOT NULL,     -- Extracted from data for comparison; client guarantees consistency
    deleted INTEGER DEFAULT 0,    -- Soft delete flag
    updated_by TEXT NOT NULL      -- device_id of last updater
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    data TEXT NOT NULL,           -- Full Project JSON blob
    updated_at TEXT NOT NULL,
    deleted INTEGER DEFAULT 0,
    updated_by TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    last_sync TEXT NOT NULL
);
```

The `updated_at` and `updated_by` columns are extracted from the push request (not parsed from the JSON blob). The server trusts the client to keep these consistent with the blob contents. On push, the server updates these columns from the explicit request fields alongside the `data` blob.

### Server CLI

```bash
summ-sync --port 3000 --db ./sync.db --key my-secret-api-key
```

Flags:
- `--port` (default: 3000) -- listen port
- `--db` (default: `./sync.db`) -- database path
- `--key` (required, or `SYNC_API_KEY` env var) -- API key for authentication

### Conflict Resolution

**Last Write Wins (LWW)** strategy:
1. Client push includes `updated_at` per task (from the task's `updated_at` field)
2. Server compares: client `updated_at` >= server `updated_at` -> overwrite
3. Client `updated_at` < server `updated_at` -> server version kept, task added to conflicts list
4. Server always wins conflicts by default. No force-overwrite mechanism is provided — the user resolves conflicts by editing the task on either device and syncing again.

### Error Handling

| Scenario | CLI Error Code | Behavior |
|----------|---------------|----------|
| Server unreachable | `E_SYNC_SERVER_UNREACHABLE` | Report error |
| Invalid API key | `E_SYNC_AUTH_FAILED` | Report error |
| Network timeout | retry with 5s timeout | Retry once after 1s delay, then report error. Applies to both push and pull. Retry is safe: pull is idempotent, push uses upsert semantics. |

## Migration Strategy

### Phase 1: Workspace Refactor

1. Create workspace root `Cargo.toml`
2. Move existing code to `crates/cli/`
3. Extract `crates/core/` (task.rs, error.rs, id.rs)
4. Keep `output.rs` in `crates/cli/`
5. Verify all existing tests pass

### Phase 2: Server Implementation

1. Create `crates/server/`
2. Implement server-side database layer
3. Implement API handlers and routing
4. Server tests

### Phase 3: CLI Sync Integration

1. Add `reqwest` dependency to CLI
2. Implement `sync/` module (client.rs, tracker.rs)
3. Add `sync` command to CLI
4. Add v5 database migration (updated_at, sync_config, sync_log, triggers)
5. Update existing command SQL to include `updated_at = datetime('now')` in UPDATE statements
6. Integration tests

## Dependencies to Add

**CLI (`crates/cli/Cargo.toml`):**
- `reqwest` (with `json` feature) -- HTTP client

**Server (`crates/server/Cargo.toml`):**
- `axum` -- HTTP framework
- `tokio` (full features) -- async runtime

**Core (`crates/core/Cargo.toml`):**
- Existing dependencies suffice (serde, chrono, uuid, etc.)
