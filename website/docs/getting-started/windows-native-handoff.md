---
sidebar_position: 3
title: Windows Native Handoff
---

# Windows Native Handoff

This handoff records a native Windows setup path for Hermes Agent. It is for
operators who intentionally do **not** want Docker or WSL for the primary CLI
workflow.

Native Windows support is more fragile than Linux/WSL because several parts of
the stack historically assumed POSIX behavior. The fixes in this branch cover
the common native-Windows breakpoints: Git Bash path conversion, Windows pipe
draining, stale PID files, npm command shims, local browser automation,
`execute_code`, and dashboard PTY support.

## Target Layout

Use a profile-scoped home outside the source checkout:

```powershell
$env:HERMES_HOME = "$env:LOCALAPPDATA\hermes"
[Environment]::SetEnvironmentVariable("HERMES_HOME", $env:HERMES_HOME, "User")
[Environment]::SetEnvironmentVariable("PYTHONUTF8", "1", "User")
[Environment]::SetEnvironmentVariable("PYTHONIOENCODING", "utf-8", "User")
```

Recommended source/install path:

```powershell
$repo = "$env:LOCALAPPDATA\hermes\hermes-agent"
git clone https://github.com/nousresearch/hermes-agent.git $repo
cd $repo
```

Create and install the Python environment:

```powershell
py -3.11 -m venv venv
.\venv\Scripts\python.exe -m pip install -U pip
.\venv\Scripts\python.exe -m pip install -e ".[pty]"
.\venv\Scripts\python.exe -m pip install fastapi "uvicorn[standard]"
```

Add the CLI to the user `PATH`:

```powershell
$scripts = "$repo\venv\Scripts"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($userPath -split ";") -notcontains $scripts) {
  [Environment]::SetEnvironmentVariable("Path", "$userPath;$scripts", "User")
}
```

PowerShell 7 is recommended for daily use. Making Windows Terminal default to
PowerShell 7 is helpful, but Hermes itself should not require PowerShell 7 if
the venv and environment variables are configured correctly.

## Required Windows Dependencies

Install these before running E2E checks:

- Git for Windows. Hermes native terminal support uses Git Bash and must avoid
  the WSL `C:\Windows\System32\bash.exe` shim.
- Node.js and npm.
- PowerShell 7 for a better operator shell.
- Python 3.11.

Browser automation needs repo-local Node dependencies and Chromium:

```powershell
cd $repo
npm install
npx agent-browser install
```

The TUI package also needs its own install and build:

```powershell
cd "$repo\ui-tui"
npm install
npm run build
```

The build script is cross-platform in this branch. On Windows it skips POSIX
`chmod`; on POSIX it still marks `dist/entry.js` executable.

## MiniMax Configuration

Store secrets only in `%LOCALAPPDATA%\hermes\.env`. Do not commit keys.

```dotenv
MINIMAX_CN_API_KEY=replace-with-your-key
MINIMAX_CN_BASE_URL=https://api.minimaxi.com/anthropic
```

Do not put inline comments after `.env` values. The `.env` loader treats the
full right-hand side as the value.

Use this `config.yaml` model section:

```yaml
model:
  default: minimax-m2.7-highspeed
  provider: minimax-cn
  api_mode: anthropic_messages
  base_url: https://api.minimaxi.com/anthropic
```

## Verification

Run all commands from the repo root unless stated otherwise:

```powershell
$env:HERMES_HOME = "$env:LOCALAPPDATA\hermes"
$env:PYTHONUTF8 = "1"
$env:PYTHONIOENCODING = "utf-8"
.\venv\Scripts\hermes.exe doctor
```

Expected core tool availability:

- `browser`
- `code_execution`
- `terminal`
- `file`
- `memory`
- `skills`
- `todo`
- `tts`
- `vision`

Warnings for missing third-party credentials are normal until those services
are configured: Discord, Feishu, Home Assistant, image generation, OpenRouter,
Tavily/Firecrawl, Tinker/W&B, Spotify, Yuanbao, and messaging platform tools.

Run a model and core-tool E2E:

```powershell
.\venv\Scripts\hermes.exe --toolsets terminal,file,code_execution,todo -z `
  "Use terminal to echo TERMINAL=OK, use file tool to write/read a small file proving FILE=OK, use execute_code to print CODE_EXEC=OK, then answer exactly: MODEL=minimax-m2.7-highspeed; TERMINAL=OK; FILE=OK; CODE_EXEC=OK"
```

Expected answer:

```text
MODEL=minimax-m2.7-highspeed; TERMINAL=OK; FILE=OK; CODE_EXEC=OK
```

Run a browser E2E:

```powershell
.\venv\Scripts\hermes.exe --toolsets browser -z `
  "Use browser_navigate to open https://example.com and answer exactly: BROWSER=OK; TITLE=Example Domain"
```

Expected answer:

```text
BROWSER=OK; TITLE=Example Domain
```

Run a dashboard/TUI smoke check:

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

## Known Remaining Warnings

`browser-cdp` is intentionally unavailable unless a CDP endpoint is connected.
Use `/browser connect` or set `browser.cdp_url` to a running Chrome DevTools
WebSocket endpoint. The regular local browser tool does not need this.

`kanban` tools are only exposed inside kanban worker contexts or when explicitly
configured for gateway dispatch.

`messaging` requires a running Hermes gateway and at least one configured
platform adapter.

Provider-specific toolsets require their own credentials and should not be
enabled during a basic native Windows handoff unless those credentials are
available.
