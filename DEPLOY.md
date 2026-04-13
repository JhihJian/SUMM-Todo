# DEPLOY.md — SUMM-Todo Sync 部署手册

## 概述

SUMM-Todo 支持通过中央 `summ-sync` 服务器在多设备间同步任务数据。服务端使用 axum + SQLite，客户端通过 HTTP Bearer token 认证。

## 架构

```
设备 A (todo CLI) ──┐
                    ├──► summ-sync (port 3911) ── SQLite (sync.db)
设备 B (todo CLI) ──┘
```

## 服务端部署

### 构建

```bash
cargo build --release -p summ-sync
# 二进制位于 target/release/summ-sync
```

### 安装

```bash
sudo cp target/release/summ-sync /usr/local/bin/
```

### 配置

| 参数 | CLI Flag | 环境变量 | 默认值 |
|------|----------|----------|--------|
| 端口 | `--port` | `SYNC_PORT` | `3000` |
| 数据库路径 | `--db` | `SYNC_DB_PATH` | `./sync.db` |
| API Key | `--key` | `SYNC_API_KEY` | (必填) |

### 生成 API Key

```bash
openssl rand -hex 16
```

### Systemd User Service (推荐)

1. 创建配置文件 `~/.config/summ-sync.env`：

```
SYNC_API_KEY=<your-generated-key>
```

2. 创建 service 文件 `~/.config/systemd/user/summ-sync.service`：

```ini
[Unit]
Description=SUMM Sync Server
After=network.target

[Service]
Type=simple
EnvironmentFile=%h/.config/summ-sync.env
ExecStart=/usr/local/bin/summ-sync --port 3911 --db %h/.local/share/summ-sync/sync.db --key ${SYNC_API_KEY}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

3. 启用并启动：

```bash
mkdir -p ~/.local/share/summ-sync
systemctl --user daemon-reload
systemctl --user enable --now summ-sync.service
```

4. 验证：

```bash
systemctl --user status summ-sync.service
curl -s -H "Authorization: Bearer <key>" http://localhost:3911/api/v1/sync/status
```

### 管理命令

```bash
systemctl --user status summ-sync     # 查看状态
systemctl --user restart summ-sync    # 重启
systemctl --user stop summ-sync       # 停止
systemctl --user disable summ-sync    # 取消开机自启
journalctl --user -u summ-sync -f     # 查看日志
```

## CLI 端配置

### 升级 CLI

```bash
cargo build --release -p todo
sudo cp target/release/todo /usr/local/bin/todo
```

### 初始化同步

```bash
todo sync init --server http://localhost:3911 --key <your-key>
```

这会：
- 生成 8 位 device ID 并保存到 `~/.todo/todo.db`
- 保存 server_url 和 api_key
- 验证服务器连通性
- 执行首次 pull + push

### 同步命令

```bash
todo sync            # 双向同步 (pull → push)
todo sync push       # 仅推送本地变更
todo sync pull       # 仅拉取远端变更
todo sync status     # 查看同步状态
```

### 冲突解决

使用 **Last Write Wins (LWW)** 策略 — `updated_at` 时间戳较新的版本优先。

## API 端点

所有端点需要 `Authorization: Bearer <key>` 头。

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/sync/push` | POST | 推送变更 |
| `/api/v1/sync/pull` | POST | 拉取变更 |
| `/api/v1/sync/status` | GET | 服务器状态 |

## 多设备接入

在新设备上重复 CLI 配置步骤：

```bash
# 1. 安装 todo 二进制
# 2. 初始化同步 (使用相同的 server URL 和 key)
todo sync init --server http://<server-host>:3911 --key <your-key>
# 3. 执行同步
todo sync
```

## 本机当前部署信息

| 项目 | 值 |
|------|------|
| 服务端 | `summ-sync` @ `/usr/local/bin/summ-sync` |
| 端口 | `3911` |
| 数据库 | `~/.local/share/summ-sync/sync.db` |
| 配置 | `~/.config/summ-sync.env` |
| Service | `~/.config/systemd/user/summ-sync.service` |
| CLI | `todo` @ `/usr/local/bin/todo` |
| Device ID | 自动生成 (存储在 `~/.todo/todo.db` 的 `sync_config` 表) |
| 同步协议 | v1, Bearer token auth, LWW conflict resolution |

## 故障排查

| 问题 | 排查 |
|------|------|
| 连接被拒 | `systemctl --user status summ-sync` 检查服务是否运行 |
| 认证失败 | 确认 CLI 和服务端使用相同的 API key |
| 数据不同步 | `todo sync status` 查看 last_sync 时间 |
| 端口冲突 | 修改 `--port` 并重新 init：`todo sync init --server http://localhost:<new-port> --key <key>` |
| 数据库锁定 | 确认只有 summ-sync 进程在访问 sync.db |
