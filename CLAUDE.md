# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SUMM-Todo is a CLI tool for Human-Agent task coordination with multi-device sync support. The project is a Cargo workspace with three crates: `core` (shared types), `cli` (the todo binary), and `server` (summ-sync binary).

## Development Commands

```bash
cargo test --workspace                   # Run all tests
cargo test -p todo <test_name>           # Run specific CLI test
cargo test -p summ-sync <test_name>      # Run specific server test
cargo build --release -p todo -p summ-sync  # Build release binaries
cargo clippy --workspace -- -W clippy::all  # Lint with all warnings
cargo run -- <command>                   # Run CLI (e.g., cargo run -- list)
```

## Architecture

### Workspace Structure

```
crates/core/    - Shared types (Task, Project, TodoError, generate_id)
crates/cli/     - CLI tool (todo binary)
crates/server/  - Sync server (summ-sync binary)
```

### Core Modules (`crates/core/src/`)

- `task.rs` - Task/Project models with state machine transitions
- `error.rs` - Error types with codes (e.g., `E_TASK_NOT_FOUND`)
- `id.rs` - UUID v7-based 8-character ID generation with collision check

### CLI Modules (`crates/cli/src/`)

- `main.rs` - Entry point, dispatches commands
- `cli.rs` - CLI argument definitions using `argh` derive macros
- `db.rs` - SQLite database layer with `TaskFilter`, sync_config, sync_log queries
- `output.rs` - Three output modes: TOON (default), JSON, Pretty
- `commands/` - Command handlers (add, start, done, sync, etc.)
- `sync/` - Sync module (client.rs for HTTP, tracker.rs for change tracking)

### Server Modules (`crates/server/src/`)

- `main.rs` - Server entry point
- `config.rs` - CLI args & config
- `db.rs` - Server-side SQLite (JSON blob storage)
- `handlers.rs` - API handlers (push, pull, status)
- `router.rs` - Route definitions with auth middleware

### Command Pattern

Each command in `crates/cli/src/commands/` follows:

```rust
pub fn execute(db: &Database, args: XxxArgs, output: &Output) -> Result<String, TodoError>
```

### Task State Machine

```
pending → in_progress → done
    ↓         ↓
cancelled  blocked
             ↓
          in_progress (resume)
```

### Output Modes

| Mode | Flag | Use Case |
|------|------|----------|
| TOON | (default) | LLM consumption (18-40% token savings) |
| JSON | `--json` | Backwards compatibility |
| Pretty | `-p` | Human terminal viewing |

### Database

- Location: `~/.todo/todo.db` (or `TODO_DB_PATH` env var)
- Migrations: `migrations/v1.sql` through `v5.sql` - loaded via `include_str!`
- v5 adds: `updated_at`, `sync_config`, `sync_log` (change tracking triggers)
- Uses WAL mode for concurrent access
- Schema version tracked via `PRAGMA user_version`

### Sync Protocol

- `todo sync init --server <url> --key <key>` - Initialize
- `todo sync` - Full sync (pull then push)
- `todo sync push/pull` - One-directional
- Server: `summ-sync --port 3000 --key <key>`
- Auth: Bearer token
- Conflict resolution: Last Write Wins (LWW) via `updated_at`

## Release Process

Releases are automated via `.github/workflows/release.yml`:
- Triggered by git tags matching `v*`
- Builds for Linux (x64, ARM64), macOS (x64, ARM64), Windows (x64)

## Key Dependencies

- `argh` - CLI argument parsing (derive macros)
- `rusqlite` - SQLite with bundled feature
- `toon-format` - TOON encoding/decoding
- `chrono` / `chrono-english` - Date parsing and timestamps
- `uuid` (v7) - Time-sortable unique IDs
- `axum` + `tokio` - Server HTTP framework (server crate only)
- `reqwest` - HTTP client with rustls (CLI crate only)
