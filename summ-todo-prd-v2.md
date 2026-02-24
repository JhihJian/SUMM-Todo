# PRD v2：Todo as Human-Agent Coordination Protocol

> 版本：v2.0 · 作者：产品负责人视角的完全重写

---

## 零、为什么推翻重写

原版 PRD 有三个结构性问题：

1. **价值主张模糊**——同时讲了"零摩擦记录""知识积累""Agent 协作"三个故事，没有主次。MVP 不知道在验证哪个假设。
2. **核心差异点不可交付**——"越用越聪明"依赖大量历史数据和语义检索，这在产品冷启动时无法兑现，用户第一天就会失望。
3. **回避了关键工程决策**——存储方案、Agent 接入模型、任务状态机、并发控制全部缺失，无法指导开发。

本版重写的原则：**砍掉所有"听起来很好但第一天无法交付价值"的东西，聚焦一个可验证的楔子。**

---

## 一、我们到底在解决什么问题

不从第一性原理出发——从一个具体的、今天正在发生的痛点出发。

**痛点：人和 AI Agent 之间没有持久的任务状态。**

今天的开发者使用 Claude Code、Cursor 等 Agent 工具时，工作模式是这样的：

1. 在对话里告诉 Agent "帮我重构这个模块"
2. Agent 执行完，结果散落在对话历史、Git commit、终端输出里
3. 下一次会话，上下文归零——你要重新解释背景
4. 你脑子里有五件事想让 Agent 做，但没地方排队
5. 你不知道 Agent 上次到底做了什么、做到哪里、卡在哪里

**本质问题：Agent 有执行能力，但没有持久记忆和任务协议。人和 Agent 之间缺一个"共享工作台"。**

我们要做的就是这个共享工作台。

---

## 二、产品定义

**一句话：**
> 一个本地优先的 CLI 工具，作为人与 AI Agent 之间的任务协议层——在这里排队、领取、执行、汇报、回顾。

**它是什么：**
- 一个任务状态机，定义了任务从创建到完成的所有合法状态转换
- 一个 CLI 接口，人和 Agent 用同一套命令操作
- 一个本地数据库，持久化存储任务及其执行日志

**它不是什么：**
- 不是知识库（不做语义搜索、不做"越用越聪明"——那是 v2 的事）
- 不是 Agent 运行时（不调度 Agent、不管 Agent 怎么执行，只管任务状态）
- 不是项目管理工具（单人使用，无协作）

---

## 三、用户画像与核心场景

**唯一用户画像：** 每天使用 Code Agent 的个人开发者，在终端里生活。

### 场景 1：人排队，Agent 消费（主场景，MVP 必须做到丝滑）

```
# 早上，开发者快速把今天要做的事扔进去
$ todo add "把 user service 的认证逻辑换成 JWT" --tag=backend --pri=high
$ todo add "给 /api/health 加上 DB 连通性检查" --tag=backend
$ todo add "README 里的安装步骤过时了，更新一下" --tag=docs

# 开发者打开 Claude Code，给它一句话
> 从 todo 系统领取一个 backend 任务来做

# Claude Code 调用 CLI
$ todo next --tag=backend
→ 返回 JSON：任务 ID、标题、上下文
→ 任务状态自动变为 in_progress，记录执行者为 agent

# Agent 执行完毕
$ todo done 3a7f --result="JWT 认证已实现，替换了 session 方案" \
    --artifact="commit:a1b2c3d" \
    --log="遇到 refresh token 的并发问题，用 Redis 锁解决"

# 晚上，开发者回顾今天的成果
$ todo log --today --pretty
→ 人类可读的今日执行摘要
```

**这个场景要丝滑到什么程度：** Agent 从 `todo next` 到 `todo done` 的完整流程，零人工干预、零歧义、零副作用。

### 场景 2：Agent 执行中发现子任务

```
# Agent 在做"换成 JWT"的过程中，发现 token 过期处理也要改
$ todo add "实现 JWT refresh token 自动续期" \
    --tag=backend --pri=medium \
    --parent=3a7f \
    --creator=agent

# 这条任务进入队列，等待下次被 next 领取
# 开发者可以在 todo list 里看到这条新任务及其来源
```

### 场景 3：纯人工使用（无 Agent）

```
$ todo add "和 PM 确认 Q3 需求优先级" --tag=meeting
$ todo done 9c2e --result="确认了三个 P0 需求，见飞书文档"

# 即使没有 Agent，工具仍然是一个好用的个人任务记录器
# 价值：比脑子记靠谱，比 Notion 轻量，结果不会丢
```

---

## 四、数据模型

### 4.1 Task（唯一核心实体）

```
Task {
  // === 身份 ===
  id:           字符串    // 短哈希，如 "3a7f"，取 UUID 前 4-8 位，冲突时自动加长
  
  // === 意图（创建时写入，不可变）===
  title:        字符串    // 自然语言描述，必填
  creator:      枚举      // human | agent
  created_at:   时间戳
  
  // === 属性（可修改）===
  priority:     枚举      // high | medium | low，默认 medium
  tags:         字符串[]  // 自由标签
  parent_id:    字符串?   // 可选，父任务 ID（仅一层深度，不允许嵌套）
  due:          时间戳?   // 可选截止时间
  
  // === 状态机 ===
  status:       枚举      // pending → in_progress → blocked → done | cancelled
  assignee:     枚举?     // human | agent（领取时写入）
  blocked_reason: 字符串? // status=blocked 时必填
  
  // === 结果（完成时写入）===
  result:       字符串?   // 做了什么，一句话
  artifacts:    字符串[]  // 产出物引用：commit hash、文件路径、PR URL 等
  log:          字符串?   // 执行过程中的备注、踩坑记录
  started_at:   时间戳?   // 首次进入 in_progress 的时间
  finished_at:  时间戳?   // 进入 done/cancelled 的时间
}
```

### 4.2 设计决策与理由

**为什么没有 Goal 层级？**

原版要求每个 Task 必须关联 Goal。这带来两个问题：第一，增加了创建摩擦（用户想快速记一件事，却被迫先想"这属于哪个目标"）；第二，Goal 的管理本身成为负担。

替代方案：**用 tag 实现松散分组，用 parent_id 实现任务分解。** 如果用户需要目标概念，可以创建一条任务作为"目标任务"，其他任务通过 parent_id 挂在它下面。这样 Goal 只是 Task 的一种用法，不是独立实体，系统复杂度大幅降低。

如果验证后发现用户确实需要一个显式的目标层，v2 再加。不要在 MVP 里为一个未验证的需求增加核心模型复杂度。

**为什么 parent_id 只允许一层？**

多层嵌套带来的查询复杂度和 UI 呈现问题远大于收益。一层父子关系足以覆盖"大任务拆小任务"的场景。如果有人需要项目 → 模块 → 任务这种结构，他需要的是项目管理工具，不是我们。

**为什么 Task 是唯一实体？**

认知负担最低。用户只需要学一个概念。CLI 命令集最小。数据迁移最简单。"一个工具只做一件事"。

### 4.3 状态机（严格定义）

```
                    ┌──────────┐
                    │ cancelled│
                    └──────────┘
                         ↑
                         │ cancel（任何非终态均可取消）
                         │
┌─────────┐  next/start  ┌─────────────┐  done   ┌──────┐
│ pending │ ───────────→ │ in_progress │ ──────→ │ done │
└─────────┘              └─────────────┘         └──────┘
                           ↑         │
                    resume │         │ block
                           │         ↓
                         ┌───────────┐
                         │  blocked  │
                         └───────────┘
```

**状态转换规则（每一条都必须在代码中强制执行）：**

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

**幂等性规则：**
- `todo done <id>` 对已完成的任务：返回成功 + 当前状态，不报错
- `todo start <id>` 对已在 in_progress 的任务：返回成功 + 当前状态
- 对终态任务执行非法转换：返回错误码 `E_INVALID_TRANSITION`

---

## 五、CLI 接口完整定义

### 5.1 全局约定

- 所有命令的输出默认为 JSON（机器可读）
- 加 `--pretty` 或 `-p` 后输出人类可读的格式化文本
- 错误输出到 stderr，格式固定为 `{"error": "<code>", "message": "<描述>"}`
- 退出码：0 成功，1 用户输入错误，2 系统内部错误

### 5.2 命令清单

**创建**
```bash
todo add <title> [options]
  --pri, -r     high|medium|low    默认 medium
  --tag, -t     字符串（可多次）    
  --parent      任务 ID
  --due         日期（YYYY-MM-DD 或相对时间如 "3d"）
  --creator     human|agent        默认 human
  
# 返回：新任务的完整 JSON
```

**领取下一个任务**
```bash
todo next [options]
  --tag, -t     按标签过滤
  --pri, -r     按优先级过滤

# 行为：
#   1. 查询 status=pending 的任务，按 priority DESC → created_at ASC 排序
#   2. 取第一条，将其状态改为 in_progress
#   3. 写入 assignee=agent，started_at=now
#   4. 返回该任务的完整 JSON
#
# 幂等性说明：next 本质上是一个读取+写入的原子操作，
# 连续两次调用会返回不同的任务（第一个已变为 in_progress）。
# 如果没有可领取的任务：返回 {"error": "E_QUEUE_EMPTY", "message": "No pending tasks match filters"}
```

**手动开始某个任务**
```bash
todo start <id>
  --assignee    human|agent    默认 human

# 与 next 的区别：next 是"给我一个"，start 是"我要这个"
# 幂等：对已在 in_progress 的同一任务重复调用，返回成功 + 当前状态
```

**查看任务详情**
```bash
todo show <id>
# 返回：任务完整 JSON，包括所有字段
```

**完成任务**
```bash
todo done <id> [options]
  --result, -m  字符串    必填：一句话描述做了什么
  --artifact    字符串（可多次）  产出物链接
  --log         字符串    执行备注

# 将状态改为 done，写入 finished_at=now
```

**阻塞 / 恢复**
```bash
todo block <id> --reason <字符串>
todo resume <id>
```

**取消**
```bash
todo cancel <id> [--reason <字符串>]
```

**查询列表**
```bash
todo list [options]
  --status, -s   pending|in_progress|blocked|done|cancelled（可多次）
  --tag, -t      按标签过滤（可多次，AND 逻辑）
  --pri, -r      按优先级过滤
  --parent       查看某任务的子任务
  --creator      human|agent
  --since        时间过滤（如 "today", "7d", "2025-01-01"）
  --limit        返回数量，默认 20
  --sort         排序字段，默认 "priority,created_at"

# 返回：任务数组
```

**执行日志（回顾用）**
```bash
todo log [options]
  --today        等价于 --since=today --status=done
  --since        时间范围
  --tag          标签过滤
  --pretty       人类可读摘要格式

# 只返回 done 状态的任务，侧重展示 result 和 artifact 字段
# 这是给人用的回顾命令，--pretty 模式下按时间线排列
```

**批量导入（Agent 友好）**
```bash
todo import --json <file_or_stdin>

# 接受 JSON 数组，批量创建任务
# 用于 Agent 一次性拆解出多个子任务的场景
```

**数据统计**
```bash
todo stats [options]
  --since       时间范围
  --tag         标签过滤

# 返回：
# {
#   "total": 42,
#   "by_status": {"done": 30, "pending": 8, ...},
#   "by_creator": {"human": 25, "agent": 17},
#   "avg_duration_minutes": 47,
#   "by_tag": {"backend": 20, "docs": 5, ...}
# }
```

### 5.3 未进入 MVP 的命令

以下命令在 v2 中考虑：

- `todo context <id>`——基于语义相似度检索历史相关任务（需要向量存储）
- `todo goal add/list/show`——如果验证后确认需要显式目标层
- `todo sync`——多设备同步

---

## 六、存储方案

### 6.1 选型：本地 SQLite

**理由：**
- CLI 工具的用户期望是开箱即用，不需要启动服务
- SQLite 是单文件、零配置、嵌入式的，完美匹配
- 支持 SQL 查询，足以覆盖 MVP 所有过滤和排序需求
- Agent 和人在同一台机器上操作同一个文件，天然一致

**存储位置：** `~/.todo/todo.db`

**并发控制：** SQLite 的 WAL 模式支持一写多读。对于单用户单机场景足够。如果 Agent 和人同时写入，SQLite 的文件锁会串行化写操作，延迟在毫秒级，可接受。

### 6.2 Schema（与数据模型一一对应）

```sql
CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  creator       TEXT NOT NULL DEFAULT 'human' CHECK(creator IN ('human', 'agent')),
  created_at    TEXT NOT NULL DEFAULT (datetime('now')),
  
  priority      TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('high', 'medium', 'low')),
  tags          TEXT DEFAULT '[]',   -- JSON array
  parent_id     TEXT REFERENCES tasks(id),
  due           TEXT,
  
  status        TEXT NOT NULL DEFAULT 'pending' 
                CHECK(status IN ('pending', 'in_progress', 'blocked', 'done', 'cancelled')),
  assignee      TEXT CHECK(assignee IN ('human', 'agent')),
  blocked_reason TEXT,
  
  result        TEXT,
  artifacts     TEXT DEFAULT '[]',   -- JSON array
  log           TEXT,
  started_at    TEXT,
  finished_at   TEXT
);

CREATE INDEX idx_status ON tasks(status);
CREATE INDEX idx_tags ON tasks(tags);
CREATE INDEX idx_priority ON tasks(priority);
CREATE INDEX idx_parent ON tasks(parent_id);
CREATE INDEX idx_created ON tasks(created_at);
```

### 6.3 迁移策略

使用版本号管理 schema 变更。数据库文件内置 `PRAGMA user_version` 存储当前版本。每次启动时检查版本，自动执行增量迁移。

---

## 七、Agent 接入协议

这是产品最核心的差异化，必须定义清楚。

### 7.1 Agent 如何发现并使用此工具

**方式 A：直接 CLI 调用（MVP）**

Agent（如 Claude Code）在系统 PATH 中发现 `todo` 命令，通过 system prompt 或工具描述知道如何使用。

我们提供一段标准的工具描述文本，Agent 平台可以直接集成：

```
你可以使用 `todo` CLI 工具管理任务。所有命令输出 JSON。
核心流程：todo next → 执行任务 → todo done <id> --result="..."
完整命令：todo --help
```

**方式 B：MCP Server（v2）**

将 CLI 封装为 MCP（Model Context Protocol）server，让支持 MCP 的 Agent 平台自动发现和调用。

### 7.2 Agent 行为约定

| 场景 | Agent 应该做什么 |
|------|----------------|
| 领取任务 | `todo next --tag=...`，如果返回 E_QUEUE_EMPTY 则告知用户没有待办 |
| 执行中发现子任务 | `todo add "..." --parent=<当前任务id> --creator=agent` |
| 执行遇到阻塞 | `todo block <id> --reason="需要 API key"`，然后告知用户 |
| 执行完成 | `todo done <id> --result="..." --artifact="..."` |
| 不确定该做什么 | `todo list --status=pending --pretty`，让用户选择 |

### 7.3 安全边界

- Agent 可以创建任务（`--creator=agent`），但这些任务默认带有 `creator=agent` 标记，用户可以过滤审查
- Agent 不能删除任务（没有 delete 命令，只有 cancel）
- Agent 不能修改他人创建的任务的优先级或标签（v1 暂不限制，观察滥用情况后决定）
- 任务数量无上限，但 `todo stats` 可以帮助用户发现 Agent 是否在制造噪音

---

## 八、技术选型（MVP）

| 决策 | 选择 | 理由 |
|------|------|------|
| 语言 | Rust 或 Go | CLI 工具需要单二进制、快启动、跨平台。Rust 优选（性能 + 类型安全），Go 备选（开发速度）|
| 存储 | SQLite（via rusqlite / go-sqlite3）| 见第六节 |
| ID 生成 | UUID v7 取前 4 位 hex，冲突时自动扩展到 6/8 位 | 短 ID 便于手动输入 |
| 配置 | `~/.todo/config.toml` | 可配置默认标签、优先级、pretty 模式等 |
| 分发 | GitHub Release + Homebrew + cargo install | 覆盖主流安装方式 |

---

## 九、MVP 范围——精确到可交付物

### 必须做（发布拦截条件）

1. `todo add` / `todo start` / `todo next` / `todo done` / `todo block` / `todo resume` / `todo cancel`——完整状态机
2. `todo list` / `todo show`——带过滤的查询
3. `todo log --today --pretty`——人类可读的日回顾
4. `todo stats`——基础统计
5. JSON 默认输出 + `--pretty` 模式
6. SQLite 本地存储 + schema 迁移
7. 错误码体系（文档化）
8. `todo --help` 和 `todo <cmd> --help`——自文档化
9. Agent 集成描述文本（一段 Markdown，可粘贴到 system prompt）

### 明确不做（写下来防止 scope creep）

- 语义搜索 / 向量检索
- Goal 作为独立实体
- Web UI
- 多设备同步
- MCP Server
- 自动任务捕获（邮件、会议）
- 通知系统
- 撤销（undo）

---

## 十、成功指标——可度量的版本

**发布后 4 周内验证：**

| 指标 | 目标 | 度量方式 |
|------|------|---------|
| Agent 完整生命周期成功率 | > 95% | 统计 `next` 后在 30 分钟内到达 `done` 或 `blocked` 的比例（通过 started_at / finished_at 计算）|
| 任务完成时结果填写率 | > 80% | `done` 状态的任务中 `result` 字段非空的比例（CLI 已强制 result 必填，此指标验证用户是否在写有意义的内容而非敷衍）|
| Agent 创建任务的比例 | 观察值，不设目标 | `creator=agent` 的任务占比。用于判断 Agent 是否真的在自主发现子任务 |
| 日活跃命令调用次数 | > 10 次/天/用户 | 通过可选的匿名使用统计（opt-in）|
| `todo log --pretty` 使用频率 | > 3 次/周/用户 | 同上。验证"回顾"是否是真实需求 |

**核心假设与验证方式：**

| 假设 | 验证方式 | 判定标准 |
|------|---------|---------|
| Agent 可以通过 CLI 自主完成任务流转 | 在 Claude Code 中给 Agent 标准工具描述，让它处理 10 个任务 | 9/10 成功完成完整流程 |
| 用户愿意记录任务结果 | `result` 字段平均长度 | 平均 > 15 字（排除"done""ok"等敷衍内容）|
| `todo log` 对用户有回顾价值 | 用户访谈 + 使用频率 | 60% 的周活用户每周至少使用 3 次 |

---

## 十一、设计原则（修订版）

1. **Protocol first, UI second**——我们在设计一个人和 Agent 之间的协议，不是在做一个 App。所有设计决策先问"Agent 能不能正确调用"，再问"人用着舒不舒服"。
2. **输入宽松，输出严格**——接受自然语言输入、模糊时间、不完整参数。但输出必须是结构化 JSON，错误必须有编码。对人宽容，对机器精确。
3. **状态机是法律**——所有状态转换必须经过定义好的路径，没有后门。这是系统可靠性的基石，也是 Agent 可以信任系统的前提。
4. **本地优先，单文件部署**——一个二进制 + 一个 SQLite 文件。没有服务、没有配置、没有账号。`brew install todo && todo add "hello world"` 就能开始用。
5. **做少一点，但每一点都打磨到位**——宁可只有 10 个命令但每个都 100% 可靠，也不要 30 个命令但到处是边界情况。

---

## 十二、Roadmap 概览

| 阶段 | 聚焦 | 时间 |
|------|------|------|
| **v1.0 MVP** | CLI 完整状态机 + SQLite + Agent 集成描述 | 4 周 |
| **v1.1** | MCP Server 封装 + `todo context` 基于关键词的历史检索 | +2 周 |
| **v2.0** | 语义向量检索 + Goal 层级（如果数据验证需要）+ TUI 界面 | +6 周 |
| **v3.0** | 多设备同步 + Web UI + 可选的匿名使用统计 | 待定 |

---

## 十三、风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Agent 平台不开放 CLI 调用权限 | 中 | 高 | v1.1 紧跟 MCP 标准，降低接入门槛 |
| 用户觉得 CLI-only 太硬核 | 高 | 中 | v1 先服务开发者这个小众但高价值人群，v2 加 TUI，v3 加 Web UI |
| SQLite 在高频写入下的并发瓶颈 | 低 | 低 | 单用户场景写入量极小，WAL 模式足够 |
| Agent 创建大量垃圾任务 | 中 | 中 | `creator=agent` 标记 + `todo stats` 监控 + 未来可加 Agent 创建速率限制 |
| 竞品出现（Taskfile、Makefile 增强版等） | 中 | 中 | 我们的差异是"结果记录 + Agent 协议"，不是任务管理本身 |

---

## 附录 A：与原版 PRD 的核心差异对照

| 维度 | 原版 | 本版 | 为什么改 |
|------|------|------|---------|
| 核心价值 | "知识积累 + 越用越聪明" | "人-Agent 任务协议" | 知识积累需要冷启动数据，协议价值第一天就能兑现 |
| 数据模型 | Goal + Task，Goal 必填 | Task only，parent_id 可选 | 减少摩擦，降低模型复杂度 |
| 上下文检索 | MVP 包含关键词检索 | MVP 不包含，v1.1 加入 | MVP 聚焦状态机可靠性，检索是锦上添花 |
| 存储 | 未定义 | SQLite，路径 `~/.todo/todo.db` | CLI 工具必须明确存储方案 |
| 状态机 | 四个状态，无转换规则 | 五个状态 + 完整转换表 + 幂等性规则 | Agent 需要确定性，模糊的状态机是灾难 |
| 成功指标 | 模糊（"主动查看比例 > 40%"） | 具体度量方式 + 判定标准 | 不可度量的指标等于没有指标 |
| 技术选型 | 未提及 | Rust/Go + SQLite + 分发方案 | PRD 必须给出可行性约束 |
