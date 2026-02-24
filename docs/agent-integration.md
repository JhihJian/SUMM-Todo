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

## Example Session

```bash
# Agent claims a task
$ todo next --tag=backend
{
  "id": "a1b2",
  "title": "Implement JWT authentication",
  "status": "in_progress",
  "assignee": "agent",
  ...
}

# Agent creates subtask discovered during work
$ todo add "Handle token refresh" --parent=a1b2 --creator=agent

# Agent completes the task
$ todo done a1b2 --result="JWT auth implemented with RS256" --artifact="commit:abc123"

# Agent reviews what's done today
$ todo log --today
```
