# Todo CLI - Agent Integration Guide

The `todo` CLI is designed for seamless human-agent task coordination. It outputs **TOON format** by default - a token-efficient format optimized for LLM consumption, achieving **18-40% token savings** compared to JSON.

## Output Formats

| Flag | Format | Use Case |
|------|--------|----------|
| *(default)* | TOON | LLM/agent consumption (token-efficient) |
| `--json` | JSON | Backwards compatibility with existing tooling |
| `--pretty` | Human-readable | Direct terminal inspection by humans |

### TOON Format

TOON (Token-Optimized Object Notation) is the default output format, specifically designed to reduce token usage when LLMs process task data.

**Key characteristics:**
- Compact key-value syntax: `key: value`
- Arrays use indexed notation: `tags[2]: backend,api`
- Empty fields are omitted (no visual noise)
- Strings are quoted only when necessary

**Example - Single Task:**
```toon
id: "019c92"
title: Implement JWT authentication
creator: agent
created_at: "2026-02-25T10:30:00Z"
priority: high
tags[2]: backend,security
status: in_progress
assignee: agent
started_at: "2026-02-25T11:00:00Z"
```

**Example - Task List:**
```toon
[2]:
  - id: "019c"
    title: Write API tests
    creator: human
    priority: medium
    tags[1]: testing
    status: pending
  - id: "019c92"
    title: Implement JWT authentication
    creator: agent
    priority: high
    tags[2]: backend,security
    status: in_progress
    assignee: agent
```

**Example - Statistics:**
```toon
total: 15
by_status:
  done: 8
  in_progress: 2
  pending: 5
by_creator:
  agent: 10
  human: 5
avg_duration_minutes: 45
by_tag:
  backend: 8
  frontend: 4
  testing: 3
```

### JSON Format (Legacy)

For backwards compatibility with existing tooling integrations, use the `--json` flag:

```bash
todo list --json
```

```json
[
  {
    "id": "019c92",
    "title": "Implement JWT authentication",
    "creator": "agent",
    "created_at": "2026-02-25T10:30:00Z",
    "priority": "high",
    "tags": ["backend", "security"],
    "status": "in_progress",
    "assignee": "agent",
    "started_at": "2026-02-25T11:00:00Z"
  }
]
```

## Core Workflow

The standard agent workflow follows this pattern:

```bash
# 1. Claim the next available task
todo next [--tag=<filter>] [--pri=<priority>]

# 2. Execute the task...

# 3. Report completion with results
todo done <id> -m "What you did" --artifact="commit:abc123"
```

## Commands Reference

| Command | Purpose |
|---------|---------|
| `todo next [-t TAG] [-r PRI]` | Claim next pending task (auto-assigns to agent) |
| `todo start <id> [--assignee NAME]` | Start a specific task |
| `todo done <id> -m "result"` | Complete a task (result required) |
| `todo block <id> --reason="..."` | Mark task as blocked |
| `todo resume <id>` | Resume a blocked task |
| `todo cancel <id> [--reason="..."]` | Cancel a task |
| `todo add "title" [options]` | Create a new task |
| `todo list [filters]` | List tasks with filters |
| `todo show <id>` | View task details |
| `todo log [--today]` | View completed task history |
| `todo stats [filters]` | View task statistics |
| `todo import -f FILE` | Bulk import tasks from JSON |

### Command Details

#### `todo add` - Create Task
```bash
todo add "Task title" \
  --pri high|medium|low \
  --tag backend \
  --tag api \
  --parent <parent_id> \
  --due "tomorrow" \
  --creator agent|human
```

#### `todo list` - List Tasks
```bash
todo list \
  --status pending \
  --status in_progress \
  --tag backend \
  --pri high \
  --parent <id> \
  --creator agent \
  --since "2026-01-01" \
  --limit 10
```

#### `todo done` - Complete Task
```bash
todo done <id> \
  -m "Implementation complete with tests" \
  --artifact "commit:abc123" \
  --artifact "pr:45" \
  --log "Additional execution notes"
```

## Error Handling

Errors are output in TOON format to stderr with a non-zero exit code.

### Error Output Format
```toon
error: E_TASK_NOT_FOUND
message: "Task not found: abc123"
```

### Exit Codes

| Code | Category | Meaning |
|------|----------|---------|
| 0 | Success | Command completed successfully |
| 1 | User Error | Invalid input, task not found, invalid transition, etc. |
| 2 | System Error | Database error, I/O error |

### Error Codes Reference

| Code | Description | Common Cause |
|------|-------------|--------------|
| `E_QUEUE_EMPTY` | No pending tasks available | `todo next` with empty queue |
| `E_TASK_NOT_FOUND` | Task ID does not exist | Invalid or deleted task ID |
| `E_INVALID_TRANSITION` | Invalid status change | Attempting done from pending |
| `E_RESULT_REQUIRED` | Missing result on completion | `todo done` without `-m` |
| `E_BLOCKED_REASON_REQUIRED` | Missing reason when blocking | `todo block` without `--reason` |
| `E_INVALID_INPUT` | Invalid parameter value | Bad priority, status, etc. |
| `E_PARSE_ERROR` | Failed to parse input | Invalid date format, etc. |
| `E_DATABASE` | Database operation failed | Corrupted DB, locked file |
| `E_IO` | Filesystem error | Permission denied, disk full |

### Handling Errors in Scripts

```bash
# Check exit code
if ! todo done abc123 -m "Done" 2>/dev/null; then
    echo "Task completion failed"
    # Handle error...
fi

# Capture error details
result=$(todo show nonexistent 2>&1)
if [ $? -ne 0 ]; then
    # Parse TOON error output
    echo "Error occurred: $result"
fi
```

## Best Practices

### 1. Always Check Task Results
Before working on a task, verify the claim was successful:

```bash
task=$(todo next --tag backend)
# Verify task was claimed before proceeding
```

### 2. Use Meaningful Result Messages
Provide descriptive completion messages, not just "done":

```bash
# Good
todo done abc123 -m "Implemented JWT auth with RS256 signing and token refresh"

# Bad
todo done abc123 -m "Done"
```

### 3. Link Artifacts for Traceability
Always link commits, PRs, and files:

```bash
todo done abc123 \
  -m "Feature complete" \
  --artifact "commit:abc123def456" \
  --artifact "pr:42" \
  --artifact "file:src/auth/jwt.rs"
```

### 4. Use `--creator agent` for Subtasks
When creating tasks discovered during work:

```bash
todo add "Handle token refresh edge case" \
  --parent abc123 \
  --creator agent \
  --tag backend
```

### 5. Block When Waiting on External Input
Use blocking when you need human decision or external dependency:

```bash
todo block abc123 --reason="Waiting for API credentials from ops team"
```

### 6. Use Tags for Filtering
Apply consistent tags for efficient filtering:

```bash
todo list --tag backend --status pending
todo next --tag urgent --pri high
```

### 7. Review Daily Progress
Use the log command to track completed work:

```bash
todo log --today
todo log --since "2026-02-01" --tag backend
```

### 8. Use Default TOON Output for Token Efficiency
TOON is the default output format for a reason - it saves 18-40% tokens compared to JSON. Only use `--json` when required for backwards compatibility with existing tooling.

```bash
# Good - uses default TOON format (token-efficient)
todo list --status pending

# Avoid unless needed for tooling integration
todo list --status pending --json
```

## Task States

```
                    +--------+
                    |pending |
                    +---+----+
                        |
            +-----------+-----------+
            |                       |
            v                       v
     +------+------+         +-------+------+
     | in_progress |         |  cancelled   |
     +------+------+         +--------------+
            |
    +-------+-------+-------+
    |               |       |
    v               v       v
+---+---+    +------+------+ |
| done  |    |  blocked  | |
+-------+    +-----------+ |
                 |        |
                 +--------+
```

**Valid Transitions:**
- `pending` -> `in_progress`, `cancelled`
- `in_progress` -> `done`, `blocked`, `cancelled`
- `blocked` -> `in_progress`, `cancelled`

**Terminal States:** `done`, `cancelled` (no further transitions allowed)

## Example Session

### Complete Agent Workflow

```bash
# Agent starts work session - claim next backend task
$ todo next --tag backend
id: "a1b2"
title: Implement JWT authentication
creator: human
priority: high
tags[1]: backend
status: in_progress
assignee: agent
started_at: "2026-02-25T10:00:00Z"

# Agent discovers subtask needed during implementation
$ todo add "Handle token refresh" \
    --parent a1b2 \
    --creator agent \
    --tag backend \
    --pri high
id: "c3d4"
title: Handle token refresh
creator: agent
priority: high
tags[1]: backend
status: pending
parent_id: "a1b2"

# Agent needs clarification - block the task
$ todo block a1b2 --reason="Need clarification on token expiry policy"
id: "a1b2"
title: Implement JWT authentication
status: blocked
blocked_reason: "Need clarification on token expiry policy"

# Human provides answer - resume work
$ todo resume a1b2
id: "a1b2"
title: Implement JWT authentication
status: in_progress
assignee: agent

# Agent completes the task with artifacts
$ todo done a1b2 \
    -m "JWT auth implemented with RS256 signing, 15min access tokens, 7day refresh tokens" \
    --artifact "commit:abc123" \
    --artifact "pr:45"
id: "a1b2"
title: Implement JWT authentication
status: done
result: JWT auth implemented with RS256 signing, 15min access tokens, 7day refresh tokens
artifacts[2]: "commit:abc123","pr:45"
finished_at: "2026-02-25T14:30:00Z"

# Agent reviews today's completed work
$ todo log --today
[1]:
  - id: "a1b2"
    title: Implement JWT authentication
    status: done
    result: JWT auth implemented with RS256 signing, 15min access tokens, 7day refresh tokens
    artifacts[2]: "commit:abc123","pr:45"
    finished_at: "2026-02-25T14:30:00Z"

# Agent checks remaining work
$ todo list --status pending --tag backend
[2]:
  - id: "c3d4"
    title: Handle token refresh
    priority: high
    tags[1]: backend
    status: pending
    parent_id: "a1b2"
```

### Error Handling Example

```bash
# Attempt to claim from empty queue
$ todo next --tag nonexistent
error: E_QUEUE_EMPTY
message: "Queue is empty"

# Invalid task ID
$ todo show xyz999
error: E_TASK_NOT_FOUND
message: "Task not found: xyz999"

# Try to complete task without result
$ todo done a1b2
error: E_RESULT_REQUIRED
message: "Result is required when completing a task"
```

## Token Efficiency Comparison

For a typical task, TOON format provides significant token savings:

| Format | Approximate Tokens | Savings vs JSON |
|--------|-------------------|-----------------|
| JSON | ~180 tokens | - |
| TOON | ~120 tokens | **33%** |

For task lists, savings increase to **35-40%** due to:
- No repeated braces and quotes
- Compact array notation
- Omitted empty/null fields

This efficiency is critical when:
- Processing large task lists
- Maintaining conversation context
- Working with token-limited models
