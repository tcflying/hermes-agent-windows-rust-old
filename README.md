# Hermes Rust Rewrite

**Status:** EARLY STAGE - Initial scaffolding

## What is this?

A complete Rust rewrite of [hermes-agent](https://github.com/NousResearch/hermes-agent) — a ~300K LOC Python AI agent framework — targeting Windows as the primary platform with a full-featured desktop GUI.

**Note:** This is NOT a working application yet. This is the initial project scaffolding.

## Current Status

- ✅ Rust workspace structure created
- ✅ Cargo dependencies configured (Tokio, Axum, Reqwest, Rusqlite, Ratatui, etc.)
- ✅ 8 workspace crates initialized
- ✅ Basic CLI binary (`hermes`) runs and shows help
- ✅ Tool registry core (stub)
- ✅ Session database (stub with SQLite/FTS5)
- ✅ Config loader (stub)
- ✅ Gateway router (stub)
- ✅ Platform adapters stubs (Telegram, Discord, Slack)

## What's NOT Implemented Yet

Everything meaningful. This is a skeleton. The actual Agent Loop, Tools, Gateway, CLI, UI, etc. are stubs.

## Architecture

```
hermes-rs/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── agent/             # Core AI agent loop (~9K LOC target)
│   ├── tool-registry/      # Central tool registry
│   ├── session/            # SQLite + FTS5 session storage
│   ├── config/             # Configuration management
│   ├── gateway/            # Messaging platform gateway
│   │   └── platforms/      # 16 platform adapters
│   ├── cli/                # CLI entry point + slash commands
│   ├── ui/                 # React frontend (Tauri WebView)
│   └── utils/              # Shared utilities
```

## Building

```powershell
# Requires Rust 1.93+
cargo build --release

# Run
cargo run
```

## Technology Stack

| Component | Choice |
|-----------|--------|
| GUI | Tauri 2.x (WebView2) |
| Frontend | React + TypeScript |
| Async Runtime | Tokio |
| Web Framework | Axum |
| HTTP Client | Reqwest |
| SQLite | Rusqlite (bundled) |
| CLI | Clap (derive) |
| TUI | Ratatui |
| OAuth | oauth2 + arctic-oauth |

## Design Document

See `docs/plans/2026-04-11-hermes-rust-rewrite-design.md` for the full design.

## Next Steps

This is the end of the initial scaffolding session. The next step is **Brainstorming Phase 2** — detailed design review and approval, followed by implementation planning.

To continue development:
1. Review the design document at `docs/plans/2026-04-11-hermes-rust-rewrite-design.md`
2. Decide on key questions (GUI framework, feature scope, test strategy)
3. Proceed to Phase 2: Writing detailed implementation plans
