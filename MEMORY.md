# TinyVegeta Project Memory

> This file tracks project context, decisions, and progress for future Cline sessions.

## Project Overview

**TinyVegeta** is a multi-agent, multi-team, Telegram-first 24/7 AI assistant.
- **Repo:** https://github.com/AlbanBeluli/tinyvegeta
- **Version:** 1.0.0 (Rust Rewrite)
- **Tech Stack:** Rust (Axum, Teloxide, Leptos), tmux

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     tinyvegeta (Rust Binary)                     │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  CLI Cmd   │  │    Queue    │  │   Memory    │              │
│  │  Dispatch  │  │  Processor  │  │   System    │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  Telegram   │  │   AI CLI    │  │  Heartbeat  │              │
│  │    Bot      │  │  Invocation │  │   Daemon    │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Web Server (Axum REST API)                   │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

- **File-based queue** for reliable message handling
- **Three-layer memory** (global/agent/task)
- **Six AI providers**: Claude, Codex, Cline, OpenCode, Ollama, Grok
- **Agent workspaces**: `~/tinyvegeta-workspace/{agent_id}/`
- **Runtime data**: `~/.tinyvegeta/` (settings, queue, memory, logs, pairing)

---

## Session History

### 2026-02-21 - Proactive Brain System (Files + Runtime Wiring)

Executed proactive system rollout in `~/ai/tinyvegeta`:

1. Baseline files created
   - `BRAIN.md` (live working memory)
   - `IDENTITY.md`
   - `USER.md`
   - `TOOLS.md`
   - `HEARTBEAT.md`
   - `CLIENTS.md`
   - `PLAYBOOK.md`

2. Workspace structure created
   - `memory/`
   - `skills/` with starter packs:
     - `x-post-writer`
     - `website-dev`
     - `script-polish`
     - `security-auditor`
   - `content/`, `consulting/`, `drafts/`, `crm/`

3. Context loader expanded (`src/context.rs`)
   - Added loading for: `BRAIN.md`, `IDENTITY.md`, `USER.md`, `TOOLS.md`,
     `HEARTBEAT.md`, `CLIENTS.md`, `PLAYBOOK.md`
   - Enforced BRAIN-first prompt ordering each session.

4. Proactive heartbeat checks (`src/heartbeat/daemon.rs`)
   - Added periodic `BRAIN.md` scan for stale/placeholder/overdue markers.
   - Auto-appends daily `[auto-check YYYY-MM-DD]` line.
   - Logs event/decision/outcome to SQLite operational memory.
   - Stores quick status keys:
     - `brain.last_check`
     - `brain.last_summary`

5. Telegram `/brain` command set (`src/telegram/client.rs`)
   - `/brain show`
   - `/brain status`
   - `/brain add <text>`

### 2026-02-21 - Heartbeat Production Upgrade

Converted `heartbeat.md` into a production autonomic-maintenance spec and wired runtime behavior:

- Updated heartbeat runtime checks in `src/heartbeat/daemon.rs`:
  - periodic `doctor --fix`
  - queue pressure checks
  - tmux session recovery attempt
  - per-agent freshness and failure-rate checks
  - provider availability checks
  - disk space and SQLite size/vacuum checks
  - stale pairing request cleanup
  - daily global memory compaction
  - sovereign runtime liveness checks when enabled
- Added heartbeat audit trail:
  - `~/.tinyvegeta/audit/heartbeat.jsonl`
- Added heartbeat status memory keys:
  - `heartbeat.last_timestamp`
  - `heartbeat.health_score`
  - `heartbeat.last_actions`
  - `heartbeat.last_warnings`
- Added SQLite helper support in `src/memory/sqlite.rs`:
  - failed outcomes in last hour
  - DB path accessor
  - vacuum helper
- Updated heartbeat task loader (`src/heartbeat/tasks.rs`) to prefer `HEARTBEAT.md` with fallback to `heartbeat.md`.
- Added CLI support for verbose single-agent heartbeat:
  - `tinyvegeta heartbeat --agent assistant --verbose`

### 2026-02-20 - Deterministic Routing + SQLite Ops Memory + Execution Contracts

Implemented requested reliability upgrades in Rust runtime:

1. Deterministic routing (typed task schema)
   - Added `src/task.rs` with `RoutedTask` schema:
     - `intent`
     - `owner`
     - `priority`
     - `deadline`
   - Added hard-rule routing in queue processor for messages without explicit `@agent`.
   - Routing decisions are persisted as SQLite `decisions`.

2. SQLite operational memory
   - Added `src/memory/sqlite.rs` and `src/memory/mod.rs` export.
   - Added persisted tables:
     - `events`
     - `decisions`
     - `outcomes`
   - Added session summarization and global memory summary key:
     - `session.<session_id>.summary`

3. Execution contracts
   - Added `src/agent.rs`:
     - per-agent/provider execution contract (`timeout_seconds`, `retries`, `retry_backoff_ms`)
     - failure reason codes (`timeout`, `unauthorized`, `provider_unavailable`, `cli_missing`, `unknown`)
   - Applied contracts in:
     - queue message processing
     - heartbeat task execution
     - task spawn path for provider completions

4. Observability in `status`
   - Upgraded `tinyvegeta status` to include:
     - queue depth (`incoming`, `processing`, `outgoing`, `total`)
     - per-agent health state
     - last success timestamp
     - last error summary

### 2026-02-20 - Codex Sandbox Root Cause Fix

Root cause of "blocked by runtime filesystem policy" was a hard-coded Codex provider sandbox:

- File: `src/providers/codex.rs`
- Previous behavior: forced `--sandbox workspace-write`
- New behavior: forces `--sandbox danger-full-access`

This removes workspace-only write restrictions for Codex runs in TinyVegeta.

### 2026-02-19 - Telegram Sovereign Remote Controls

Added Telegram remote control for sovereign runtime in `src/telegram/client.rs`:

- New command: `/sovereign`
  - `/sovereign status`
  - `/sovereign start [@agent] [goal...] [--dry-run]`
  - `/sovereign stop`
- Bot command menu updated to include `sovereign`.
- Help text updated with sovereign command usage.
- Runtime process tracking persisted in global memory:
  - `sovereign.process.pid`
  - `sovereign.process.meta`
- Stale PID cleanup implemented in status/stop flows.

### 2026-02-19 - Sovereign Runtime v1 Scaffold

Implemented first-pass autonomous sovereign runtime in Rust:

- Added `src/sovereign/mod.rs`:
  - Continuous loop: Think -> Act -> Observe -> Repeat
  - JSON action parsing/execution (`shell`, `write_file`, `memory_set`, `schedule_set`, `skill_create`, `replicate_agent`)
  - Safety rails: dangerous command blocklist, optional protected file blocking, self-modification rate limit
  - Audit logging to `~/.tinyvegeta/audit/sovereign.jsonl`
- Added immutable laws file: `constitution/LAWS.md`
- Added docs: `docs/SOVEREIGN_RUNTIME.md`
- Added CLI command:
  - `tinyvegeta sovereign --agent <id> --goal <text> [--max-cycles N] [--dry-run]`
- Added config section `settings.sovereign` with policy controls:
  - `constitution_path`, `protected_files`, `loop_sleep_seconds`,
    `max_actions_per_cycle`, `max_self_modifications_per_hour`,
    `allow_tool_install`, `allow_self_modify`
- Integrated heartbeat daemon parallel execution while sovereign loop runs.

### 2026-02-18 - Complete Rust Rewrite

**Major Milestone**: Full rewrite from TypeScript/Node.js to Rust.

#### What Was Rewritten

| Chunk | Description |
|-------|-------------|
| 1 | Foundation & CLI - Cargo project, clap commands, config, logging, tmux |
| 2 | Queue System - File-based queue, routing, conversation tracking |
| 3 | Memory System - Three-layer memory, file locking, search |
| 4 | Telegram Bot - Teloxide bot, commands, pairing system |
| 5 | AI Providers - Claude, Codex, Cline, OpenCode, Ollama, Grok |
| 6 | Heartbeat - Cron scheduling, task spawning |
| 7 | Web Server - Axum REST API, JWT auth |
| 8 | Polish & Release - Doctor, update, uninstall, cleanup |

#### Key Decisions Made

1. **Single Binary Deployment**
   - Replaced all TypeScript/Shell with single 4.7MB Rust binary
   - Zero runtime dependencies (except tmux for daemon mode)

2. **Provider Architecture**
   - Created `Provider` trait for extensibility
   - CLI providers: Claude, Codex, Cline, OpenCode
   - HTTP providers: Ollama (local), Grok (X.AI API)

3. **Memory System**
   - Three scopes: global, agent, task
   - File-based with locking
   - Search and snapshots

4. **Telegram Bot**
   - Teloxide framework
   - Commands: /help, /agent, /team, /board, /reset, /triage, /doctor, /provider, /memory, /logs, /gateway, /releasecheck, /soul, /sovereign
   - Pairing system with approval codes

5. **Web Server**
   - Axum with CORS
   - REST API for agents, teams, memory
   - JWT authentication ready

#### What Was Removed

- All TypeScript source files (`src/lib/**/*.ts`)
- Node.js dependencies (`package.json`, `node_modules/`)
- Shell scripts (`lib/*.sh`, `scripts/*.sh`, `tinyvegeta.sh`)
- Build artifacts (`dist/`)
- Old test files (`tests/*.mjs`)
- Agent skills (`.agents/`)

#### Final Structure

```
tinyvegeta/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli/mod.rs
│   ├── config.rs
│   ├── error.rs
│   ├── logging.rs
│   ├── tmux.rs
│   ├── core/
│   │   ├── mod.rs
│   │   ├── queue.rs
│   │   ├── routing.rs
│   │   └── conversation.rs
│   ├── memory/
│   │   ├── mod.rs
│   │   ├── store.rs
│   │   └── lock.rs
│   ├── telegram/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── commands.rs
│   │   ├── handler.rs
│   │   └── pairing.rs
│   ├── providers/
│   │   ├── mod.rs
│   │   ├── provider.rs
│   │   ├── claude.rs
│   │   ├── codex.rs
│   │   ├── cline.rs
│   │   ├── opencode.rs
│   │   ├── ollama.rs
│   │   └── grok.rs
│   ├── sovereign/
│   │   └── mod.rs
│   ├── heartbeat/
│   │   ├── mod.rs
│   │   ├── daemon.rs
│   │   ├── scheduler.rs
│   │   └── tasks.rs
│   └── web/
│       ├── mod.rs
│       ├── server.rs
│       ├── router.rs
│       ├── auth.rs
│       └── api/
│           ├── mod.rs
│           ├── agents.rs
│           ├── teams.rs
│           └── memory.rs
├── README.md
├── MEMORY.md
└── docs/
```

---

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point |
| `src/cli/mod.rs` | All CLI commands (clap) |
| `src/config.rs` | Settings parsing |
| `src/core/queue.rs` | Message queue |
| `src/core/routing.rs` | Message routing |
| `src/memory/store.rs` | Memory operations |
| `src/telegram/client.rs` | Telegram bot |
| `src/providers/mod.rs` | AI provider factory |
| `src/heartbeat/daemon.rs` | Heartbeat scheduler |
| `src/web/server.rs` | REST API server |

---

## CLI Commands

```bash
# Daemon
tinyvegeta start              # Start daemon
tinyvegeta stop               # Stop daemon
tinyvegeta status             # Show status
tinyvegeta attach             # Attach to tmux

# Agents
tinyvegeta agent list
tinyvegeta agent show <id>

# Teams
tinyvegeta team list
tinyvegeta team show <id>

# Memory
tinyvegeta memory set <key> <value> [scope] [scope_id]
tinyvegeta memory get <key>
tinyvegeta memory list
tinyvegeta memory search <query>
tinyvegeta memory stats

# Queue
tinyvegeta queue stats
tinyvegeta queue incoming
tinyvegeta queue enqueue <message>

# Providers
tinyvegeta provider
tinyvegeta provider <name>

# Services
tinyvegeta telegram
tinyvegeta heartbeat [--agent <id>]
tinyvegeta web [--port PORT]

# Diagnostics
tinyvegeta doctor [--fix]
tinyvegeta releasecheck

# Other
tinyvegeta update
tinyvegeta uninstall --yes [--purge-data]
```

---

## API Endpoints

```
GET    /health              # Health check

GET    /api/agents          # List agents
POST   /api/agents          # Create agent
GET    /api/agents/:id      # Get agent
DELETE /api/agents/:id      # Delete agent

GET    /api/teams           # List teams
POST   /api/teams           # Create team
GET    /api/teams/:id       # Get team
DELETE /api/teams/:id       # Delete team

GET    /api/memory          # List memory
POST   /api/memory          # Set memory
GET    /api/memory/:key     # Get memory
DELETE /api/memory/:key     # Delete memory
GET    /api/memory/search   # Search memory
GET    /api/memory/stats    # Stats
```

---

## Current State

- ✅ Rust rewrite complete
- ✅ Binary size: 4.7MB
- ✅ All features working
- ✅ Old TypeScript code removed
- ✅ Single binary deployment
- ✅ Queue processor implemented (messages now processed!)
- ✅ Setup wizard with model selection
- ✅ Provider-specific model options
- ✅ Sovereign runtime v1 scaffold with constitutional guardrails + audit

---

## 2026-02-18 Follow-up Fixes (Post-install Runtime Behavior)

### Why this was needed

After install/reinstall, Telegram responses sometimes still introduced as Codex, and provider switching did not reliably honor CLI default model behavior.

### Fixes shipped

- `f860f5f` - SOUL fallback loading + removed telegram processing ack path.
- `d7b23b9` - stronger TinyVegeta identity instruction in system prompt.
- `86abb05` - runtime identity guard in queue processor (prevents Codex self-intro leak).
- `1204b56` - codex provider uses `workspace-write`; avoid false read-only claims.
- `ef279d3` - provider switching + model behavior fixed:
  - `tinyvegeta provider <name>` updates active agent provider/model.
  - CLI providers (`claude`, `codex`, `cline`, `opencode`) use model `default` unless explicitly set.
  - For CLI providers, `default` means no forced `--model`; provider CLI default is used.

### Operational instructions

```bash
# Upgrade to latest main
cargo install --git https://github.com/AlbanBeluli/tinyvegeta --force

# Switch provider and use provider CLI default model
tinyvegeta provider cline
tinyvegeta restart

# Optional: force specific model
tinyvegeta provider cline --model claude-sonnet-4-20250514
tinyvegeta restart
```

### Verification checklist

1. `which tinyvegeta` points to expected binary.
2. `~/.tinyvegeta/settings.json` has desired provider for `agents.assistant.provider`.
3. If using CLI default model flow, `agents.assistant.model` is `"default"`.
4. Telegram `who are you` should return TinyVegeta identity.

### Path-awareness note

Observed issue pattern: responses sounded "dumb" about filesystem context (acting like workspace was empty or wrong role active).

Root causes and fixes:

- Non-deterministic default agent routing caused occasional wrong specialist persona responses.
- Memory/context retrieval now includes agent/team scopes and should reinforce active workspace state.
- Default routing is now deterministic and prefers `assistant` unless explicit `@agent` or `@team`.

Operator checks:

```bash
cat ~/.tinyvegeta/settings.json | jq '.workspace.path, .agents.assistant.working_directory'
tinyvegeta memory explain "where are you working" --agent assistant --team board 6
tinyvegeta board show
```

If response still ignores workspace context, verify the message did not route to another specialist agent and that assistant `SOUL.md` exists in its working directory.

---

## Recent Fixes (2026-02-18)

### 10-Point Runtime Hardening (Implemented)

1. Default-agent hardening
- Added `settings.routing.default_agent`.
- Enforced in routing resolution (`assistant` fallback only if valid).
- Startup now validates settings before daemon launch.

2. Workspace-awareness prompt guard
- Every prompt now injects runtime context block:
  - `agent_id`
  - `working_directory`
  - `workspace_root`
  - `team_id`
  - `board_id`

3. Board decisions pipeline
- CEO decisions are parsed into structured fields (`decision`, `owners`, `deadlines`, `risks`).
- Schema validation added before persisting to team memory.
- Added `board decisions export --format markdown|json`.

4. Delegation lifecycle
- Delegations now progress through `open -> in_progress -> done|blocked`.
- Follow-up scanner detects overdue delegation items and records follow-up actions.

5. Memory quality controls
- Added `tinyvegeta memory compact`.
- Added per-scope memory limits and pruning.

6. Retrieval quality
- `Memory::relevant` now uses hybrid ranking:
  - token match
  - lightweight semantic embedding similarity
  - recency
  - importance

7. Team command completeness
- Added non-interactive team create:
  - `tinyvegeta team add --id --members --leader`
- Added `tinyvegeta team update`.

8. Board schedules operational
- Implemented schedule persistence/list/remove.
- Heartbeat now executes board `daily` / `digest` schedules.
- Added schedule run logs + retry counters in memory.

9. Tests expanded
- Added/updated tests for:
  - team routing to leader
  - identity guard behavior
  - runtime context injection
  - provider default-model arg handling
  - decision parser/schema validation

10. Operational visibility
- `tinyvegeta doctor` now checks:
  - missing SOUL/MEMORY in agent workspaces
  - working_directory/workspace mismatches
  - missing provider CLIs
  - Cline auth status
  - stale tmux session state
  - board/team graph consistency

### Queue Processor Added
- Messages were being queued but not processed
- Added `run_queue_processor()` to `cmd_start_internal()`
- Polls incoming queue, calls AI provider, sends Telegram response
- Added `Queue::remove_incoming()` for direct removal

### Setup Wizard Improvements
- Provider-specific model selection with 4-5 options each
- Default models updated:
  - Codex: `gpt-5.3-codex` (recommended)
  - Claude: `sonnet` (default)
  - Ollama: `llama3.3` (latest)
  - Grok: `grok-2` (latest)
- Users can select from list or enter custom model

### Bug Fixes
- Fixed deprecated teloxide API (`msg.from` → `msg.from.as_ref()`)
- Fixed `start-internal` command missing
- Fixed pairing approve/unpair to actually work
- Suppressed unused code warnings with `#![allow(dead_code)]`

---

## Supported Models

| Provider | Default | Other Options |
|----------|---------|---------------|
| Claude | sonnet | opus, sonnet-3.5, haiku |
| Codex | gpt-5.3-codex | o3, o4-mini, gpt-4.1 |
| Cline | default | claude-sonnet, gpt-4o |
| OpenCode | default | claude-sonnet, gpt-4o |
| Ollama | llama3.3 | llama3.1, codellama, mistral, deepseek-coder |
| Grok | grok-2 | grok-2-mini, grok-beta |

---

---

## Future Roadmap

### Potential Enhancements

1. **Leptos SSR Frontend**
   - Replace REST API + SPA with server-side rendered UI
   - Better performance and SEO

2. **WebSocket Real-time**
   - Replace SSE with WebSocket for real-time updates
   - Bidirectional communication

3. **SQLite Database**
   - Persist tasks and audit logs
   - Better querying

4. **Multi-user Auth**
   - Full user management
   - Role-based access

5. **Provider Streaming**
   - Stream responses from AI providers
   - Better UX for long responses

6. **Memory Visualization**
   - Web UI for memory dashboard
   - Memory graph visualization

---

## Build & Deploy

```bash
# Development
cargo build

# Release
cargo build --release

# Run
./target/release/tinyvegeta --help

# Install
sudo cp target/release/tinyvegeta /usr/local/bin/
```

---

## Notes

- User prefers Vegeta-themed branding
- Telegram-only (no WhatsApp/Discord)
- File-based queue for reliability
- Memory persistence across restarts
- Expert-level agent templates
- tmux required for daemon mode

---

## Operator Update (2026-02-18, Late)

- Confirmed fix: Telegram no longer forwards raw Cline JSON (`task_started`, event stream lines).
- Added strict default-agent routing config (`routing.default_agent`) + startup validation.
- Added runtime prompt context guard with active paths/team/board identifiers.
- Added board decision struct persistence + decision export commands.
- Added delegation lifecycle statuses and overdue follow-up automation.
- Added memory compaction + scope pruning + hybrid retrieval ranking.
- Added non-interactive team add/update command flow.
- Added working board schedule persistence/execution with retry logs.
- Upgraded doctor checks for SOUL/path/provider/tmux/Cline auth diagnostics.
- Added remote Telegram command surface for ops:
  - `/doctor`
  - `/provider [name]`
  - `/memory stats|search`
  - `/board discuss <topic>`
  - `/logs <type> [lines]`

## Operator Update (2026-02-19)

- Added Telegram task lifecycle feedback for normal messages:
  - queued (`📥`), started (`⚙️`), complete (`✅`), failed (`❌`).
- Added Telegram compatibility commands from legacy flow:
  - `/gateway [status|restart]`
  - `/releasecheck`
  - `/models` (provider alias)
  - `/reset @agent [@agent2...]`
  - `/soul`, `/soul show`, `/soul cancel` with SOUL-owner lock.
- Added Telegram auto-triage toggle and routing (`/triage on|off|status`).
- Implemented CLI stubs:
  - `send`
  - `logs`
  - `reset`
  - `model`
  - `channels reset telegram`
  - `agent add/remove/reset`
- Implemented task subsystem commands with persistence (`~/.tinyvegeta/tasks.json`):
  - `task create/list/show/start/stop/watch/assign/delete/stats`
- Added Telegram attachment ingestion to queue context:
  - photo/document/audio/voice/video/video-note/sticker download to `~/.tinyvegeta/files`
  - file refs injected into queued message text
- Added queue-chain delegation recursion metadata:
  - chain depth marker + pending handoff hint + depth-based loop guard
