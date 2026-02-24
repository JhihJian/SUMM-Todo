# SUMM-Todo 技术设计方案

> 日期：2026-02-24
> 基于：summ-todo-prd-v2.md

---

## 一、技术选型

| 决策 | 选择 | 理由 |
|------|------|------|
| 语言 | Rust | 类型安全、性能好、适合 CLI 工具 |
| CLI 框架 | argh | 轻量、编译快、代码简洁 |
| SQLite 库 | rusqlite | 成熟稳定、同步模型适合 CLI |
| 错误处理 | thiserror | 支持自定义错误码，匹配 PRD 需求 |
| 日期时间 | chrono | 生态成熟、文档丰富 |
| ID 生成 | uuid v7 短截 | 时序有序、短 ID 便于手动输入 |

---

## 二、整体架构

```
┌─────────────────────────────────────────────────────────┐
│                      CLI Layer                          │
│  argh 解析 → 命令分发 → 结果格式化（JSON/Pretty）          │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│                   Command Handlers                      │
│  add / next / start / done / block / resume / cancel    │
│  list / show / log / stats / import                     │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│                    Domain Layer                         │
│  Task 模型 + 状态机（运行时校验）+ 业务规则                │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│                   Storage Layer                         │
│  rusqlite + Schema 迁移 + 查询构建                       │
│  文件位置: ~/.todo/todo.db                              │
└─────────────────────────────────────────────────────────┘
```

**核心设计原则：**
- CLI 层只负责解析和输出，不包含业务逻辑
- 命令处理器协调领域层和存储层
- 状态转换逻辑集中在 Task 模型中
- 存储层对上层隐藏 SQL 细节

---

## 三、核心数据模型

```rust
// src/task.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status {
    Pending,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Creator {
    Human,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    // === 身份 ===
    pub id: String,                    // 短哈希 "3a7f"

    // === 意图（创建时写入，不可变）===
    pub title: String,
    pub creator: Creator,
    pub created_at: DateTime<Utc>,

    // === 属性（可修改）===
    pub priority: Priority,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub due: Option<DateTime<Utc>>,

    // === 状态机 ===
    pub status: Status,
    pub assignee: Option<Creator>,
    pub blocked_reason: Option<String>,

    // === 结果（完成时写入）===
    pub result: Option<String>,
    pub artifacts: Vec<String>,
    pub log: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}
```

**设计要点：**
- 枚举类型保证字段值合法（Priority 只能是 High/Medium/Low）
- `Option<T>` 明确标记可选字段
- `DateTime<Utc>` 统一使用 UTC 时间，输出时再本地化

---

## 四、状态机实现

```rust
// src/task.rs (续)

impl Task {
    /// 状态转换：返回 Ok(()) 表示成功，Err(TodoError) 表示非法转换
    pub fn transition(&mut self, target: Status, context: TransitionContext) -> Result<(), TodoError> {
        match (&self.status, &target) {
            // pending → in_progress
            (Status::Pending, Status::InProgress) => {
                self.assignee = Some(context.assignee);
                self.started_at = Some(Utc::now());
            }

            // pending → cancelled
            (Status::Pending, Status::Cancelled) => {}

            // in_progress → done
            (Status::InProgress, Status::Done) => {
                let result = context.result.ok_or(TodoError::ResultRequired)?;
                self.result = Some(result);
                self.artifacts = context.artifacts.unwrap_or_default();
                self.log = context.log;
                self.finished_at = Some(Utc::now());
            }

            // in_progress → blocked
            (Status::InProgress, Status::Blocked) => {
                let reason = context.blocked_reason.ok_or(TodoError::BlockedReasonRequired)?;
                self.blocked_reason = Some(reason);
            }

            // blocked → in_progress (resume)
            (Status::Blocked, Status::InProgress) => {
                self.blocked_reason = None;
            }

            // in_progress/blocked → cancelled
            (Status::InProgress | Status::Blocked, Status::Cancelled) => {}

            // 幂等性：已在目标状态
            _ if self.status == target => return Ok(()),

            // 非法转换
            _ => return Err(TodoError::InvalidTransition {
                from: self.status.clone(),
                to: target,
            }),
        }

        self.status = target;
        Ok(())
    }
}
```

**状态转换规则（与 PRD 一致）：**

| 当前状态 | 允许的转换 | 触发命令 | 附加约束 |
|---------|-----------|---------|---------|
| pending | → in_progress | `next` 或 `start <id>` | 写入 assignee 和 started_at |
| pending | → cancelled | `cancel <id>` | — |
| in_progress | → done | `done <id>` | result 必填 |
| in_progress | → blocked | `block <id>` | blocked_reason 必填 |
| in_progress | → cancelled | `cancel <id>` | — |
| blocked | → in_progress | `resume <id>` | 清除 blocked_reason |
| blocked | → cancelled | `cancel <id>` | — |
| done | （终态） | — | 不可逆 |
| cancelled | （终态） | — | 不可逆 |

---

## 五、错误处理

```rust
// src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum TodoError {
    // 状态转换错误
    #[error("Invalid state transition: cannot go from {from:?} to {to:?}")]
    InvalidTransition { from: Status, to: Status },

    #[error("Result is required when completing a task")]
    ResultRequired,

    #[error("Blocked reason is required when blocking a task")]
    BlockedReasonRequired,

    // 任务操作错误
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("No pending tasks match filters")]
    QueueEmpty,

    // 数据库错误
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    // 输入错误
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl TodoError {
    /// 返回 PRD 定义的错误码
    pub fn code(&self) -> &'static str {
        match self {
            TodoError::InvalidTransition { .. } => "E_INVALID_TRANSITION",
            TodoError::ResultRequired => "E_RESULT_REQUIRED",
            TodoError::BlockedReasonRequired => "E_BLOCKED_REASON_REQUIRED",
            TodoError::TaskNotFound(_) => "E_TASK_NOT_FOUND",
            TodoError::QueueEmpty => "E_QUEUE_EMPTY",
            TodoError::Database(_) => "E_DATABASE",
            TodoError::InvalidInput(_) => "E_INVALID_INPUT",
            TodoError::ParseError(_) => "E_PARSE_ERROR",
        }
    }
}

/// 输出到 stderr 的 JSON 格式
pub fn format_error(err: &TodoError) -> String {
    serde_json::json!({
        "error": err.code(),
        "message": err.to_string()
    }).to_string()
}
```

---

## 六、存储层设计

```rust
// src/db.rs

use rusqlite::{Connection, params};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    const SCHEMA_VERSION: i32 = 1;

    pub fn new() -> Result<Self, TodoError> {
        let db_path = Self::db_path()?;
        std::fs::create_dir_all(db_path.parent().unwrap())?;

        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "journal_mode", &"WAL")?;

        let db = Database { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn db_path() -> Result<PathBuf, TodoError> {
        let home = dirs::home_dir().ok_or(TodoError::InvalidInput("Cannot find home directory".into()))?;
        Ok(home.join(".todo").join("todo.db"))
    }

    fn run_migrations(&self) -> Result<(), TodoError> {
        let version: i32 = self.conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version < 1 {
            self.conn.execute_batch(include_str!("../migrations/v1.sql"))?;
            self.conn.pragma_update(None, "user_version", 1)?;
        }

        Ok(())
    }

    pub fn insert_task(&self, task: &Task) -> Result<(), TodoError> { /* ... */ }
    pub fn get_task(&self, id: &str) -> Result<Option<Task>, TodoError> { /* ... */ }
    pub fn update_task(&self, task: &Task) -> Result<(), TodoError> { /* ... */ }
    pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>, TodoError> { /* ... */ }
    pub fn get_next_task(&self, tag: Option<&str>, priority: Option<&Priority>) -> Result<Option<Task>, TodoError> { /* ... */ }
}
```

**Schema（migrations/v1.sql）：**

```sql
CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  creator       TEXT NOT NULL DEFAULT 'human' CHECK(creator IN ('human', 'agent')),
  created_at    TEXT NOT NULL DEFAULT (datetime('now')),

  priority      TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('high', 'medium', 'low')),
  tags          TEXT DEFAULT '[]',
  parent_id     TEXT REFERENCES tasks(id),
  due           TEXT,

  status        TEXT NOT NULL DEFAULT 'pending'
                CHECK(status IN ('pending', 'in_progress', 'blocked', 'done', 'cancelled')),
  assignee      TEXT CHECK(assignee IN ('human', 'agent')),
  blocked_reason TEXT,

  result        TEXT,
  artifacts     TEXT DEFAULT '[]',
  log           TEXT,
  started_at    TEXT,
  finished_at   TEXT
);

CREATE INDEX idx_status ON tasks(status);
CREATE INDEX idx_priority ON tasks(priority);
CREATE INDEX idx_created ON tasks(created_at);
CREATE INDEX idx_parent ON tasks(parent_id);
```

---

## 七、ID 生成策略

PRD 要求：UUID v7 取前 4 位 hex，冲突时自动扩展到 6/8 位。

```rust
// src/id.rs

use rusqlite::Connection;

pub struct IdGenerator<'a> {
    conn: &'a Connection,
}

impl<'a> IdGenerator<'a> {
    /// 生成唯一 ID：从 4 位开始，冲突时扩展
    pub fn generate(&self) -> Result<String, TodoError> {
        let uuid = uuid::Uuid::now_v7();
        let full_hex = uuid.simple().to_string();  // 32 位 hex

        for len in [4, 6, 8] {
            let candidate = full_hex[..len].to_string();
            if !self.id_exists(&candidate)? {
                return Ok(candidate);
            }
        }

        // 极端情况：8 位也冲突，使用完整 UUID
        Ok(full_hex)
    }

    fn id_exists(&self, id: &str) -> Result<bool, TodoError> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)",
            params![id],
            |row| row.get(0)
        )?;
        Ok(exists)
    }
}
```

**冲突概率分析：**
- 4 位 hex = 65536 种可能，任务数 < 1000 时冲突概率 < 1.5%
- 6 位 hex = 16M 种可能，几乎不会冲突
- 8 位是安全兜底

---

## 八、CLI 命令层

```rust
// src/cli.rs

use argh::FromArgs;

#[derive(FromArgs)]
/// Todo - Human-Agent Task Coordination Protocol
pub struct Args {
    #[argh(subcommand)]
    pub command: Command,

    /// output in human-readable format
    #[argh(switch, short = 'p')]
    pub pretty: bool,
}

#[derive(FromArgs, SubCommand)]
pub enum Command {
    Add(AddArgs),
    Next(NextArgs),
    Start(StartArgs),
    Done(DoneArgs),
    Block(BlockArgs),
    Resume(ResumeArgs),
    Cancel(CancelArgs),
    List(ListArgs),
    Show(ShowArgs),
    Log(LogArgs),
    Stats(StatsArgs),
    Import(ImportArgs),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
/// Create a new task
pub struct AddArgs {
    #[argh(positional)]
    pub title: String,

    /// priority: high, medium, low
    #[argh(option, short = 'r')]
    pub pri: Option<String>,

    /// tag (can be used multiple times)
    #[argh(option, short = 't')]
    pub tag: Vec<String>,

    /// parent task id
    #[argh(option)]
    pub parent: Option<String>,

    /// due date (YYYY-MM-DD or relative like 3d)
    #[argh(option)]
    pub due: Option<String>,

    /// creator: human or agent
    #[argh(option)]
    pub creator: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "next")]
/// Claim the next pending task
pub struct NextArgs {
    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// filter by priority
    #[argh(option, short = 'r')]
    pub pri: Option<String>,
}

// ... 其他命令类似
```

---

## 九、输出格式化

```rust
// src/output.rs

pub struct Output {
    pretty: bool,
}

impl Output {
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    /// 输出单个任务
    pub fn task(&self, task: &Task) -> String {
        if self.pretty {
            self.format_task_pretty(task)
        } else {
            serde_json::to_string_pretty(task).unwrap()
        }
    }

    /// 输出任务列表
    pub fn task_list(&self, tasks: &[Task]) -> String {
        if self.pretty {
            self.format_list_pretty(tasks)
        } else {
            serde_json::to_string_pretty(tasks).unwrap()
        }
    }

    /// 输出执行日志（todo log --pretty）
    pub fn log(&self, tasks: &[Task]) -> String {
        if self.pretty {
            self.format_log_pretty(tasks)
        } else {
            serde_json::to_string_pretty(tasks).unwrap()
        }
    }

    fn format_task_pretty(&self, task: &Task) -> String {
        let status_icon = match task.status {
            Status::Pending => "○",
            Status::InProgress => "◐",
            Status::Blocked => "⊘",
            Status::Done => "●",
            Status::Cancelled => "✕",
        };

        let pri_indicator = match task.priority {
            Priority::High => "!",
            Priority::Medium => "·",
            Priority::Low => "_",
        };

        format!(
            "{} {} {} [{}]\n  {}",
            status_icon, pri_indicator, task.id, task.title,
            task.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ")
        )
    }

    fn format_log_pretty(&self, tasks: &[Task]) -> String {
        let mut lines = vec!["Today's completed tasks:".to_string(), "---".to_string()];

        for task in tasks {
            lines.push(format!(
                "✓ [{}] {}\n  → {}",
                task.id, task.title,
                task.result.as_deref().unwrap_or("(no result)")
            ));

            if let Some(ref artifacts) = task.artifacts {
                for a in artifacts {
                    lines.push(format!("    📎 {}", a));
                }
            }
        }

        lines.join("\n")
    }
}
```

**Pretty 模式输出示例：**
```
◐ ! 3a7f [Replace auth with JWT] #backend #security
  Started: 2h ago | Assignee: agent

Today's completed tasks:
---
✓ [b2c4] Add health check endpoint
  → Added DB connectivity check to /api/health
    📎 commit:a1b2c3d
```

---

## 十、命令处理器示例

```rust
// src/commands/add.rs

pub fn execute(db: &Database, args: AddArgs, output: &Output) -> Result<String, TodoError> {
    // 1. 解析参数
    let priority = args.pri
        .map(|p| p.parse())
        .transpose()?
        .unwrap_or(Priority::Medium);

    let creator = args.creator
        .map(|c| c.parse())
        .transpose()?
        .unwrap_or(Creator::Human);

    let due = args.due
        .map(|d| parse_relative_time(&d))
        .transpose()?;

    // 2. 生成 ID
    let id = IdGenerator::new(db).generate()?;

    // 3. 创建任务
    let task = Task {
        id,
        title: args.title,
        creator,
        priority,
        tags: args.tag,
        parent_id: args.parent,
        due,
        ..Default::default()
    };

    // 4. 持久化
    db.insert_task(&task)?;

    // 5. 格式化输出
    Ok(output.task(&task))
}

// src/commands/next.rs

pub fn execute(db: &Database, args: NextArgs, output: &Output) -> Result<String, TodoError> {
    // 1. 查询下一个待办任务
    let mut task = db.get_next_task(args.tag.as_deref(), args.pri.as_deref())?
        .ok_or(TodoError::QueueEmpty)?;

    // 2. 状态转换：pending → in_progress
    let context = TransitionContext {
        assignee: Creator::Agent,
        ..Default::default()
    };
    task.transition(Status::InProgress, context)?;

    // 3. 更新数据库
    db.update_task(&task)?;

    // 4. 格式化输出
    Ok(output.task(&task))
}
```

---

## 十一、项目依赖

**Cargo.toml：**

```toml
[package]
name = "todo"
version = "0.1.0"
edition = "2021"

[dependencies]
argh = "0.1"
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v7"] }
dirs = "5.0"
```

---

## 十二、文件结构

```
todo/
├── Cargo.toml
├── src/
│   ├── main.rs           # 入口
│   ├── cli.rs            # argh 命令定义
│   ├── error.rs          # TodoError
│   ├── task.rs           # Task 模型 + 状态机
│   ├── id.rs             # ID 生成器
│   ├── db.rs             # SQLite 操作
│   ├── output.rs         # JSON/Pretty 输出
│   ├── time.rs           # 相对时间解析
│   └── commands/
│       ├── mod.rs
│       ├── add.rs
│       ├── next.rs
│       ├── start.rs
│       ├── done.rs
│       ├── block.rs
│       ├── resume.rs
│       ├── cancel.rs
│       ├── list.rs
│       ├── show.rs
│       ├── log.rs
│       ├── stats.rs
│       └── import.rs
└── migrations/
    └── v1.sql            # Schema 初始化
```

---

## 十三、MVP 实现清单

根据 PRD 要求，MVP 必须包含：

- [ ] `todo add` / `todo start` / `todo next` / `todo done` / `todo block` / `todo resume` / `todo cancel`
- [ ] `todo list` / `todo show`
- [ ] `todo log --today --pretty`
- [ ] `todo stats`
- [ ] JSON 默认输出 + `--pretty` 模式
- [ ] SQLite 本地存储 + schema 迁移
- [ ] 错误码体系
- [ ] `todo --help` 和 `todo <cmd> --help`
- [ ] Agent 集成描述文本
