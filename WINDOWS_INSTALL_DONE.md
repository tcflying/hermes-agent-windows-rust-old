# Hermes Agent — Windows 本地安装完成

## 状态：✅ 已安装

### 安装位置
- **根目录**: `D:\claude-project\hermes-agent`
- **虚拟环境**: `venv` (Python 3.11.14)
- **配置目录**: `C:\Users\Administrator\AppData\Local\hermes`
- **命令链接**: `~/.local/bin/hermes` → `cli.py`

### 已装依赖
```
✓ pywinpty==2.0.15          (Windows ConPTY — chat tab 支持)
✓ psutil==7.2.2              (进程检测、诊断)
✓ ptyprocess==0.7.0          (POSIX PTY — WSL2 中用)
✓ pywin32==311               (Windows API)
✓ openai==2.24.0, httpx==0.28.1, ...  (核心库)
```

### Windows 特有修复已集成
1. **Gateway 状态检测** — `_get_process_start_time` 用 `psutil.Process.create_time()` 替代 `/proc/stat`
2. **诊断快照** — `shutdown_forensics` 用 `psutil.process_iter()` 收集进程树（以前 Windows 返回空）
3. **Chat Tab** — 新建 `conpty_bridge.py` 支持 Windows ConPTY，`web_server.py` 自动 fallback
4. **所有修复已测试通过**

---

## 快速开始

### 1. 激活虚拟环境（每次新终端）
```bash
cd D:\claude-project\hermes-agent
source venv/Scripts/activate
```

或用 PowerShell：
```powershell
cd D:\claude-project\hermes-agent
.\venv\Scripts\Activate.ps1
```

### 2. 配置 API 密钥
编辑 `C:\Users\Administrator\AppData\Local\.env`：
```env
OPENROUTER_API_KEY=sk-or-v1-...
```

或运行：
```bash
hermes login
```

### 3. 启动交互式 Chat
```bash
hermes chat
```

或用 TUI 仪表板（支持 PTY）：
```bash
hermes dashboard
```

或启动 Gateway（后台 Telegram/Discord/Slack）：
```bash
hermes gateway start
```

---

## 验证安装

### 检查配置
```bash
hermes status
```

应输出类似：
```
Gateway: not running
Cron: ready
Config: ~/.hermes/config.yaml
```

### 检查模型
```bash
hermes model
```

选择 OpenRouter 或其他提供商。

### 运行诊断
```bash
hermes doctor
```

输出系统环境、依赖版本、配置状态。

---

## 常用命令

| 命令 | 功能 |
|------|------|
| `hermes chat` | 交互式 AI 对话 |
| `hermes chat -m model:name` | 用指定模型 |
| `hermes dashboard` | Web 仪表板（支持 PTY chat tab） |
| `hermes gateway start` | 启动 Telegram/Discord/Slack Gateway |
| `hermes cron` | 查看定时任务 |
| `hermes skills` | 管理技能库 |
| `hermes config` | 编辑配置 |
| `hermes --version` | 版本号 |

---

## Windows 特有注意事项

### PTY/Chat Tab
- 若 Web 仪表板 chat tab 提示 "ConPTY unavailable"，检查：
  1. `pywinpty` 已装（✓ 已装）
  2. Windows 10 build 17763+ （检查 `winver`）
  3. 若都满足，重启 dashboard

### 进程管理
- 后台 Gateway 进程用 `taskkill /PID <pid> /T /F` 终止（不支持 SIGTERM）
- 状态检测现已能识别 stale PID（之前总是返回 "running"）

### 路径处理
- 接受 Windows 反斜杠（`D:\path`）和 Unix 正斜杠（`D:/path`）
- `.env` 和 `config.yaml` 放在 `C:\Users\...\AppData\Local\hermes`

---

## 故障排查

### "hermes: command not found"
激活 venv：
```bash
source venv/Scripts/activate
```

### "ModuleNotFoundError: No module named 'pywinpty'"
重装依赖：
```bash
source venv/Scripts/activate
cd D:\claude-project\hermes-agent
uv pip install --upgrade -e '.[pty]'
```

### Gateway 崩溃或频繁重启
查看日志：
```bash
hermes logs --follow
```

检查诊断：
```bash
hermes doctor
```

### Chat Tab 不工作
```bash
hermes status
```

若显示 "ConPTY unavailable"，检查 `~\.hermes\logs` 中的错误。

---

## 代码修改清单

**本次 Windows 适配修改**:

1. `gateway/status.py:111` — `_get_process_start_time()` 加 psutil fallback
2. `gateway/shutdown_forensics.py:226` — Windows 进程快照实现
3. `hermes_cli/conpty_bridge.py` — 新建 ConPTY Bridge（160 行）
4. `hermes_cli/web_server.py:3135` — PTY import chain (pty_bridge → conpty_bridge)

所有改动已测试通过，无修改 `pyproject.toml`（`pywinpty` 已在 `[pty]` extra）。

---

## 下一步

### 推荐配置
1. 设置 `.env` 中的 API 密钥
2. 运行 `hermes model` 选择默认模型
3. 运行 `hermes chat` 首次测试
4. 若需后台服务，运行 `hermes gateway start`

### 可选功能
- **Telegram/Discord/Slack**: `hermes whatsapp` / `hermes slack` 初始化
- **MCP 集成**: `hermes mcp` 添加外部工具
- **定时任务**: `hermes cron create`
- **技能库**: `hermes skills search`

---

## 支持与反馈

- **GitHub Issues**: https://github.com/nousresearch/hermes-agent/issues
- **Discord**: https://discord.gg/NousResearch
- **文档**: https://hermes-agent.nousresearch.com/docs/

---

**安装时间**: 2026-05-14  
**版本**: 0.13.0  
**Python**: 3.11.14  
**Windows 修复状态**: ✅ 完成
