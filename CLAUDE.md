# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SUMM-Todo is a CLI tool for Human-Agent task coordination. Both humans and AI agents use the same interface to queue, claim, execute, and report on tasks. The default output format is TOON (Token-Optimized Object Notation) for LLM efficiency.

## Development Commands

```bash
cargo test                        # Run all tests
cargo test <test_name>            # Run specific test
cargo build --release             # Build release binary
cargo clippy -- -W clippy::all    # Lint with all warnings
cargo run -- <command>            # Run with arguments (e.g., cargo run -- list)
```

## Architecture

### Core Modules

- `src/main.rs` - Entry point, dispatches commands to handlers
- `src/cli.rs` - CLI argument definitions using `argh` derive macros
- `src/task.rs` - Task model with strict state machine transitions
- `src/db.rs` - SQLite database layer with `TaskFilter` for queries
- `src/output.rs` - Three output modes: TOON (default), JSON, Pretty
- `src/id.rs` - UUID v7-based 8-character ID generation with collision check
- `src/error.rs` - Error types with codes (e.g., `E_TASK_NOT_FOUND`)

### Command Pattern

Each command in `src/commands/` follows the same signature:

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

Valid transitions are enforced in `Task::transition()` in `task.rs`. Terminal states (`done`, `cancelled`) cannot be changed except via explicit `undo` (done→in_progress) or `abandon` (in_progress→pending).

### Output Modes

| Mode | Flag | Use Case |
|------|------|----------|
| TOON | (default) | LLM consumption (18-40% token savings) |
| JSON | `--json` | Backwards compatibility |
| Pretty | `-p` | Human terminal viewing |

### Database

- Location: `~/.todo/todo.db` (or `TODO_DB_PATH` env var)
- Migrations: `migrations/v1.sql`, `migrations/v2.sql` - loaded via `include_str!`
- Uses WAL mode for concurrent access
- Schema version tracked via `PRAGMA user_version`

## Release Process

Releases are automated via `.github/workflows/release.yml`:
- Triggered by git tags matching `v*`
- Builds for Linux (x64, ARM64), macOS (x64, ARM64), Windows (x64)
- Artifacts uploaded to GitHub Releases

## Key Dependencies

- `argh` - CLI argument parsing (derive macros)
- `rusqlite` - SQLite with bundled feature
- `toon-format` - TOON encoding/decoding
- `chrono` / `chrono-english` - Date parsing and timestamps
- `uuid` (v7) - Time-sortable unique IDs
