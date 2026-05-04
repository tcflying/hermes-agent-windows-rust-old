# Windows Native Hermes Agent Handoff for Codex

You are taking over a native Windows Hermes Agent setup. Do not use Docker or
WSL for the primary installation unless explicitly asked later.

This is also a collaboration handoff: another Codex session may keep developing
from this branch. Treat the current changes as intentional Windows-native
compatibility work. Do not revert them while chasing unrelated issues; extend or
refine them in place.

## Goal

Install and verify Hermes Agent natively on Windows with:

- PowerShell 7 as the preferred operator shell.
- `HERMES_HOME=%LOCALAPPDATA%\hermes`.
- MiniMax model provider configured for `minimax-m2.7-highspeed`.
- Core tools working: terminal, file, browser, code_execution, memory, skills,
  todo, tts, vision.
- Dashboard/web/TUI dependencies present enough for smoke checks.

## Important Rules

- Do not print or commit API keys.
- Store secrets only in `%LOCALAPPDATA%\hermes\.env`.
- Do not use Docker for this setup.
- Assume another Codex session may be working in parallel. Check `git status`
  before editing, keep changes scoped, and do not revert edits you did not make.
- Prefer native Windows paths for user-facing config, but make Hermes terminal
  use Git Bash internally.
- Avoid WSL `C:\Windows\System32\bash.exe`; use Git for Windows Bash.
- Use PowerShell 7 (`pwsh`) for commands when available.

## Expected Paths

```powershell
$repo = "$env:LOCALAPPDATA\hermes\hermes-agent"
$home = "$env:LOCALAPPDATA\hermes"
```

Persist environment variables:

```powershell
[Environment]::SetEnvironmentVariable("HERMES_HOME", $home, "User")
[Environment]::SetEnvironmentVariable("PYTHONUTF8", "1", "User")
[Environment]::SetEnvironmentVariable("PYTHONIOENCODING", "utf-8", "User")
```

Add Hermes venv scripts to user PATH:

```powershell
$scripts = "$repo\venv\Scripts"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($userPath -split ";") -notcontains $scripts) {
  [Environment]::SetEnvironmentVariable("Path", "$userPath;$scripts", "User")
}
```

## Install/Repair Commands

From the repo root:

```powershell
py -3.11 -m venv venv
.\venv\Scripts\python.exe -m pip install -U pip
.\venv\Scripts\python.exe -m pip install -e ".[pty]"
.\venv\Scripts\python.exe -m pip install fastapi "uvicorn[standard]"
npm install
npx agent-browser install
```

For TUI:

```powershell
cd "$repo\ui-tui"
npm install
npm run build
cd $repo
```

## MiniMax Config

In `%LOCALAPPDATA%\hermes\.env`:

```dotenv
MINIMAX_CN_API_KEY=replace-with-real-key
MINIMAX_CN_BASE_URL=https://api.minimaxi.com/anthropic
```

Do not add inline comments after `.env` values.

In `%LOCALAPPDATA%\hermes\config.yaml`:

```yaml
model:
  default: minimax-m2.7-highspeed
  provider: minimax-cn
  api_mode: anthropic_messages
  base_url: https://api.minimaxi.com/anthropic
```

## Windows Fixes Expected in This Branch

This branch includes native Windows fixes for:

- Git Bash discovery and Windows/POSIX path conversion in local terminal
  execution.
- Windows pipe draining in terminal command waits.
- Stale PID handling across gateway, process registry, MCP, and browser cleanup.
- Browser tool preferring repo-local `agent-browser` over stale global npm
  shims.
- Browser `.cmd`/`.bat` launch shim via `cmd.exe /c`.
- Windows browser orphan cleanup avoiding stale PID self-kills.
- `execute_code` support on Windows via loopback TCP RPC instead of Unix
  domain sockets.
- Required Windows child env vars for Winsock: `SYSTEMROOT`, `WINDIR`,
  `COMSPEC`.
- Dashboard PTY bridge using `pywinpty`/ConPTY.
- Cross-platform TUI build script replacing POSIX-only `chmod`.

## Verification

Set runtime env for the current shell:

```powershell
$env:HERMES_HOME = "$env:LOCALAPPDATA\hermes"
$env:PYTHONUTF8 = "1"
$env:PYTHONIOENCODING = "utf-8"
```

Doctor:

```powershell
.\venv\Scripts\hermes.exe doctor
```

Expected: MiniMax connectivity OK, `agent-browser` OK, and core tools include
`browser`, `code_execution`, `terminal`, `file`, `memory`, `skills`, `todo`,
`tts`, and `vision`.

Core E2E:

```powershell
.\venv\Scripts\hermes.exe --toolsets terminal,file,code_execution,todo -z `
  "Use terminal to echo TERMINAL=OK, use file tool to write/read a small file proving FILE=OK, use execute_code to print CODE_EXEC=OK, then answer exactly: MODEL=minimax-m2.7-highspeed; TERMINAL=OK; FILE=OK; CODE_EXEC=OK"
```

Expected exact answer:

```text
MODEL=minimax-m2.7-highspeed; TERMINAL=OK; FILE=OK; CODE_EXEC=OK
```

Browser E2E:

```powershell
.\venv\Scripts\hermes.exe --toolsets browser -z `
  "Use browser_navigate to open https://example.com and answer exactly: BROWSER=OK; TITLE=Example Domain"
```

Expected exact answer:

```text
BROWSER=OK; TITLE=Example Domain
```

Dashboard/TUI smoke:

```powershell
@"
from hermes_cli.pty_bridge import PtyBridge
from hermes_cli import web_server
from pathlib import Path
print("PTY_AVAILABLE", PtyBridge.is_available())
print("WEB_IMPORT_OK", bool(web_server.app))
print("TUI_DIST", Path("ui-tui/dist/entry.js").exists())
"@ | .\venv\Scripts\python.exe -
```

Expected:

```text
PTY_AVAILABLE True
WEB_IMPORT_OK True
TUI_DIST True
```

## Normal Remaining Warnings

These are not native Windows install blockers:

- `browser-cdp`: requires `/browser connect` or `browser.cdp_url`.
- `messaging`: requires running gateway and configured platform.
- `kanban`: only appears in worker/gateway contexts.
- Discord, Feishu, Home Assistant, image generation, OpenRouter, web search
  providers, Tinker/W&B, Spotify, Yuanbao: require their own credentials.

## If Something Fails

Check:

- `$env:HERMES_HOME` points to `%LOCALAPPDATA%\hermes`.
- `%LOCALAPPDATA%\hermes\.env` has no inline comments after values.
- Git for Windows is installed and `C:\Program Files\Git\bin\bash.exe` exists.
- `node_modules\.bin\agent-browser.cmd` exists in the repo.
- `C:\Users\<user>\.agent-browser\browsers\...` contains the installed Chrome.
- `ui-tui\dist\entry.js` exists after `npm run build`.
