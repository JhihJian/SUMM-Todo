# Todo CLI

A local-first CLI tool for Human-Agent task coordination. Both humans and AI agents use the same interface to queue, claim, execute, and report on tasks.

## Installation

### Quick Install (Linux/macOS)

```bash
curl -sSL https://raw.githubusercontent.com/JhihJian/SUMM-Todo/main/install.sh | bash
```

### Homebrew (macOS)

```bash
brew tap YOUR_USERNAME/tap
brew install todo
```

### cargo install

```bash
cargo install --git https://github.com/JhihJian/SUMM-Todo
```

### Download Binary

Download from [Releases](https://github.com/JhihJian/SUMM-Todo/releases) for your platform:

| Platform | Binary |
|----------|--------|
| Linux x64 | `todo-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `todo-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x64 | `todo-x86_64-apple-darwin.tar.gz` |
| macOS ARM64 | `todo-aarch64-apple-darwin.tar.gz` |
| Windows x64 | `todo-x86_64-pc-windows-msvc.zip` |

### From Source

```bash
git clone https://github.com/JhihJian/SUMM-Todo
cd todo
cargo build --release
sudo cp target/release/todo /usr/local/bin/
```

## Usage

### Core Workflow

```bash
# Create tasks
todo add "Implement JWT auth" -r high -t backend
todo add "Update README" -t docs

# List tasks (JSON by default, -p for pretty)
todo list -p

# Claim next task (auto-assigns to agent)
todo next --tag=backend

# Complete task
todo done <id> -m "JWT auth implemented" --artifact="commit:abc123"
```

### Commands

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

### Output Format

- Default: JSON (for Agent consumption)
- With `-p` or `--pretty`: Human-readable format

```bash
# JSON output
todo list

# Pretty output
todo -p list
```

### Task States

```
pending → in_progress → done
    ↓         ↓
cancelled  blocked
             ↓
          in_progress (resume)
```

## Agent Integration

See [docs/agent-integration.md](docs/agent-integration.md) for integrating with AI agents like Claude Code.

## Development

```bash
# Run tests
cargo test

# Build release
cargo build --release

# Run clippy
cargo clippy -- -W clippy::all
```

## License

MIT
