# Todo CLI

A local-first CLI tool for Human-Agent task coordination. Both humans and AI agents use the same interface to queue, claim, execute, and report on tasks.

## Installation

### Quick Install (Linux/macOS)

```bash
curl -sSL https://raw.githubusercontent.com/JhihJian/SUMM-Todo/main/install.sh | bash
```

### Update (Linux/macOS)

```bash
curl -sSL https://raw.githubusercontent.com/JhihJian/SUMM-Todo/main/update.sh | bash
```

Or update to a specific version:

```bash
TODO_VERSION=v0.2.0 bash update.sh
```

### Download Binary

Download from [Releases](https://github.com/JhihJian/SUMM-Todo/releases) for your platform:

| Platform | Binary |
|----------|--------|
| Linux x64 | `todo-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `todo-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `todo-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `todo-aarch64-apple-darwin.tar.gz` |
| Windows x64 | `todo-x86_64-pc-windows-msvc.zip` |

Extract and place in your PATH:

```bash
# Linux/macOS
tar -xzf todo-*.tar.gz
sudo mv todo /usr/local/bin/

# Windows: extract zip and add to PATH
```

### cargo install

```bash
cargo install --git https://github.com/JhihJian/SUMM-Todo
```

### From Source

```bash
git clone https://github.com/JhihJian/SUMM-Todo
cd SUMM-Todo
cargo build --release
sudo cp target/release/todo /usr/local/bin/
```

## Quick Start

```bash
# Create a project
todo project add "my-app" -d "My Application"

# Create tasks (with or without project)
todo add "my-app: Implement JWT auth" -P high -t backend
todo add "my-app: Update README" -t docs
todo add "Standalone task"

# List tasks (TOON format by default - token-efficient for LLMs)
todo list

# Filter by project
todo list -p my-app

# Pretty output for humans
todo -p list

# Claim next task (auto-assigns to agent)
todo next

# Complete task
todo done <id> -m "Implemented JWT auth with RS256"
```

## Commands

| Command | Description |
|---------|-------------|
| `add` | Create a new task |
| `next` | Claim the next pending task |
| `start` | Start a specific task |
| `done` | Complete a task |
| `block` | Mark task as blocked |
| `resume` | Resume a blocked task |
| `cancel` | Cancel a task |
| `list` | List tasks with filters |
| `show` | Show task details |
| `log` | View execution log |
| `stats` | Show task statistics |
| `import` | Bulk import from JSON |
| `project` | Manage projects (add, edit, list, show, delete) |
| `sync` | Sync tasks with remote server |

## Multi-Device Sync

SUMM-Todo supports syncing tasks across multiple devices via a self-hosted sync server (`summ-sync`).

### Architecture

```
Device A                      summ-sync                   Device B
+----------+   HTTP/JSON   +-------------+   HTTP/JSON   +----------+
| todo CLI  | <-----------> | sync.db     | <-----------> | todo CLI  |
| todo.db   |               | (SQLite)    |               | todo.db   |
+----------+               +-------------+               +----------+
```

- **Conflict resolution**: Last Write Wins (LWW) via `updated_at` timestamp
- **Auth**: API key via `Authorization: Bearer <key>` header
- **Protocol**: REST over HTTP (use reverse proxy for TLS)

### Server Setup

**Build from source:**

```bash
cargo build --release -p summ-sync
```

**Run:**

```bash
# With CLI flags
./target/release/summ-sync --port 3000 --db ./sync.db --key my-secret-key

# Or with environment variables
SYNC_API_KEY=my-secret-key ./target/release/summ-sync --port 3000
```

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--port` | `SYNC_PORT` | `3000` | Listen port |
| `--db` | `SYNC_DB_PATH` | `./sync.db` | Database path |
| `--key` | `SYNC_API_KEY` | *(required)* | API key for authentication |

**Systemd service example** (`/etc/systemd/system/summ-sync.service`):

```ini
[Unit]
Description=SUMM-Todo Sync Server
After=network.target

[Service]
Type=simple
User=todo
Environment=SYNC_API_KEY=my-secret-key
ExecStart=/usr/local/bin/summ-sync --port 3000 --db /var/lib/summ-sync/sync.db
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

### Client Usage

```bash
# Initialize sync (first time on each device)
todo sync init --server http://your-server:3000 --key my-secret-key

# Full sync (pull then push)
todo sync

# One-directional sync
todo sync push
todo sync pull

# Check server status
todo sync status
```

**Notes:**
- Running `sync init` multiple times preserves the device ID (only updates server/key)
- Initial sync pulls all existing server data first, then pushes local data
- Change tracking is automatic via SQLite triggers (no code changes needed)

## Projects

Organize tasks into projects:

```bash
# Create a project (with optional path)
todo project add "web-app" -d "Web application" -p /path/to/web-app

# List all projects
todo project list

# Show project details with statistics
todo project show web-app

# Edit project (name, description, or path)
todo project edit web-app -n "my-web-app" -d "Updated description" -p /new/path

# Add task to project
todo add "web-app: Setup database"

# Filter tasks by project
todo list -p web-app

# Delete project (must have no tasks)
todo project delete web-app
```

### Project Fields

| Field | Flag | Description |
|-------|------|-------------|
| `name` | positional | Project name (required) |
| `description` | `-d` | Project description |
| `path` | `-p` | Project directory path |

## Task States

```
pending → in_progress → done
    ↓         ↓
cancelled  blocked
             ↓
          in_progress (resume)
```

**State transitions are strictly enforced.** Terminal states (done, cancelled) cannot be changed.

## Output Format

- **Default**: TOON (Token-Oriented Object Notation - optimized for LLMs, ~18-40% token savings)
- **With `--json`**: JSON format (for backwards compatibility)
- **With `-p`**: Human-readable format

```bash
todo list           # TOON output (default)
todo --json list    # JSON output
todo -p list        # Pretty output
```

### TOON vs JSON Comparison

**TOON** (default, token-efficient):
```
id: "019c"
title: Implement JWT auth
priority: high
tags[1]: backend
status: pending
```

**JSON** (`--json` flag):
```json
{
  "id": "019c",
  "title": "Implement JWT auth",
  "priority": "high",
  "tags": ["backend"],
  "status": "pending"
}
```

**Pretty** (`-p` flag):
```
○ ! 019c [Implement JWT auth] #backend
```

## Agent Integration

See [docs/agent-integration.md](docs/agent-integration.md) for integrating with AI agents.

## Documentation

- [v0.3.0 Release Notes](docs/v0.3.0-release.md) - Project support
- [v0.2.0 Release Notes](docs/v0.2.0-release.md) - TOON output format
- [v0.1.0 Release Notes](docs/v0.1.0-release.md) - Initial release features
- [Agent Integration Guide](docs/agent-integration.md) - For AI agents

## Development

```bash
cargo test --workspace              # Run all tests
cargo build --release -p todo -p summ-sync  # Build release binaries
cargo clippy --workspace -- -W clippy::all  # Lint
```

## License

MIT
