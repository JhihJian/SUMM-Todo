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
# Create tasks
todo add "Implement JWT auth" -r high -t backend
todo add "Update README" -t docs

# List tasks (TOON format by default - token-efficient for LLMs)
todo list

# JSON output (for backwards compatibility)
todo --json list

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

- [v0.2.0 Release Notes](docs/v0.2.0-release.md) - TOON output format
- [v0.1.0 Release Notes](docs/v0.1.0-release.md) - Initial release features
- [Agent Integration Guide](docs/agent-integration.md) - For AI agents

## Development

```bash
cargo test              # Run tests
cargo build --release   # Build release binary
cargo clippy -- -W clippy::all  # Lint
```

## License

MIT
