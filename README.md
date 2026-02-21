# TinyVegeta

> Multi-agent, multi-team, Telegram-first, 24/7 AI assistant — now in Rust.

A single-binary AI assistant that runs multiple teams of agents collaborating simultaneously with isolated workspaces.

## Features

- 🦀 **Single Binary** - Zero runtime dependencies (just tmux for daemon mode)
- 🤖 **Multi-Agent** - Run multiple isolated AI agents with specialized roles
- 👥 **Multi-Team Collaboration** - Agents hand off work via chain execution and fan-out
- 📱 **Telegram-First** - Simplified single-channel operation with pairing security
- 🧠 **Three-Layer Memory** - Global, agent, and task-scoped memory with inheritance
- 🗂️ **Deterministic Task Routing** - Typed schema (`intent`, `owner`, `priority`, `deadline`) with hard assignment rules
- 🧾 **SQLite Operational Memory** - `events`, `decisions`, and `outcomes` persisted for session summaries
- 🛠️ **Execution Contracts** - Per-agent timeout/retry policies with failure reason codes
- ⚡ **Six AI Providers** - Claude, Codex, Cline, OpenCode, Ollama, Grok
- 🌐 **REST API** - Full-featured web server with JWT auth
- 💓 **Heartbeat Daemon** - Scheduled tasks and monitoring
- 📁 **File-Based Queue** - Reliable message handling, no race conditions
- 🛡️ **Sovereign Runtime (v1)** - Continuous think/act/observe loop with constitutional guards + audit logs

## Quick Start

### Prerequisites

- macOS or Linux
- tmux (for daemon mode)
- Rust 1.70+ (only for building from source)
- One or more AI providers:
  - [Claude Code CLI](https://claude.ai/code) (default)
  - [Codex CLI](https://github.com/openai/codex)
  - [Cline CLI](https://github.com/cline/cline)
  - [OpenCode CLI](https://github.com/opencode-ai/opencode)
  - [Ollama](https://ollama.com/) (local)
  - xAI API key for Grok (`XAI_API_KEY` or `GROK_API_KEY`)

### Installation

**Option 1: Download Binary**

```bash
# Download from releases
curl -fsSL https://github.com/AlbanBeluli/tinyvegeta/releases/latest/download/tinyvegeta-$(uname -s)-$(uname -m) -o tinyvegeta
chmod +x tinyvegeta
sudo mv tinyvegeta /usr/local/bin/
```

**Option 2: Cargo Install**

```bash
cargo install --git https://github.com/AlbanBeluli/tinyvegeta

# Reinstall/upgrade and overwrite existing binary
cargo install --git https://github.com/AlbanBeluli/tinyvegeta --force
```

**Option 3: Build from Source**

```bash
git clone https://github.com/AlbanBeluli/tinyvegeta.git
cd tinyvegeta
cargo build --release
sudo cp target/release/tinyvegeta /usr/local/bin/
```

### First Run

```bash
tinyvegeta setup
```

The setup wizard will guide you through:
1. Create `~/.tinyvegeta/` directory structure
2. Enter your Telegram bot token (from @BotFather)
3. Choose AI provider:
   - **Claude** (Anthropic CLI) - sonnet, opus, haiku
   - **Codex** (OpenAI CLI) - gpt-5.3-codex, o3, o4-mini
   - **Cline CLI** - default, claude-sonnet, gpt-4o
   - **OpenCode CLI** - default, claude-sonnet, gpt-4o
   - **Ollama** (local) - llama3.3, llama3.1, codellama, mistral, deepseek-coder
   - **Grok** (xAI API) - grok-2, grok-2-mini
4. Select model from list or enter custom

After setup:

```bash
tinyvegeta start
```

On setup/start, TinyVegeta now auto-bootstraps missing agent context files in each agent workspace:

- `SOUL.md`, `MEMORY.md`, `BRAIN.md`, `IDENTITY.md`, `USER.md`, `TOOLS.md`, `HEARTBEAT.md`, `CLIENTS.md`, `PLAYBOOK.md`

Shared workspace context (new):

- TinyVegeta also bootstraps shared files at workspace root (for all agents):
  - `~/tinyvegeta-workspace/BRAIN.md`
  - `~/tinyvegeta-workspace/IDENTITY.md`
  - `~/tinyvegeta-workspace/USER.md`
  - `~/tinyvegeta-workspace/TOOLS.md`
  - `~/tinyvegeta-workspace/HEARTBEAT.md`
  - `~/tinyvegeta-workspace/CLIENTS.md`
  - `~/tinyvegeta-workspace/PLAYBOOK.md`
- Loader preference for these files is now:
  1. workspace root (shared)
  2. agent folder (local override/fallback)
  3. `~/.tinyvegeta`
  4. project fallback (`~/ai/tinyvegeta`)

Then pair your Telegram:
1. DM your Telegram bot to get a pairing code
2. Approve from shell:

```bash
tinyvegeta pairing pending
tinyvegeta pairing approve <CODE>
```

Now you're ready to chat with your AI assistant!

## Commands

### Core Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta start` | Start daemon in tmux |
| `tinyvegeta stop` | Stop daemon |
| `tinyvegeta status` | Show daemon status + queue depth + per-agent health/last error/last success |
| `tinyvegeta attach` | Attach to tmux session |
| `tinyvegeta doctor [--fix]` | Run diagnostics |
| `tinyvegeta logs [type]` | View logs (telegram/queue/heartbeat/all) |
| `tinyvegeta sovereign [--agent <id>] [--goal <text>] [--max-cycles N] [--dry-run]` | Run autonomous sovereign loop |

### Agent Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta agent list` | List all agents |
| `tinyvegeta agent show <id>` | Show agent config |
| `tinyvegeta agent add` | Add new agent (interactive) |
| `tinyvegeta agent remove <id>` | Remove agent |
| `tinyvegeta agent reset <id>` | Reset agent conversation |
| `tinyvegeta agent default [id]` | Show/set default routing agent |

### Team Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta team list` | List all teams |
| `tinyvegeta team show <id>` | Show team config |
| `tinyvegeta team add` | Add new team (interactive) |
| `tinyvegeta team add --id <id> --members a,b --leader <id>` | Add team (non-interactive) |
| `tinyvegeta team update <id> [--members a,b] [--leader <id>] [--name <name>]` | Update team |
| `tinyvegeta team remove <id>` | Remove team |

### Memory Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta memory set <key> <value> [scope] [scope_id]` | Set memory |
| `tinyvegeta memory get <key> [scope] [scope_id]` | Get memory |
| `tinyvegeta memory list [scope]` | List memory entries |
| `tinyvegeta memory search <query>` | Search memory |
| `tinyvegeta memory delete <key> [scope] [scope_id]` | Delete memory |
| `tinyvegeta memory stats` | Show memory statistics |
| `tinyvegeta memory compact [scope] [scope_id]` | Compact/dedupe/prune memory |

**Memory Scopes:** `global`, `agent`, `team`, `task`

### Queue Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta queue stats` | Show queue statistics |
| `tinyvegeta queue incoming` | List incoming messages |
| `tinyvegeta queue enqueue <message>` | Enqueue a message |

### Provider Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta provider` | Show current provider |
| `tinyvegeta provider <name>` | Switch provider |
| `tinyvegeta provider <name> --model <model>` | Switch provider and model |

### Provider Model Behavior (Important)

- `tinyvegeta provider cline` (or `claude`, `codex`, `opencode`) now sets the active agent model to `default`.
- For CLI providers, `default` means TinyVegeta does **not** force `--model`; the provider CLI's own selected/default model is used.
- If you pass `--model`, TinyVegeta forces that exact model.
- For API providers (`ollama`, `grok`), model comes from TinyVegeta settings.
- Cline JSON event streams are parsed; Telegram receives only final assistant text (no raw `task_started` JSON).

### Codex Filesystem Policy (Important)

- TinyVegeta launches Codex with `--sandbox danger-full-access`.
- This allows file operations outside `~/tinyvegeta-workspace` (full laptop access in your local user context).

Examples:

```bash
# Use Cline and let Cline's own default model apply
tinyvegeta provider cline
tinyvegeta restart

# Force a specific Cline model
tinyvegeta provider cline --model claude-sonnet-4-20250514
tinyvegeta restart
```

### Identity + SOUL Troubleshooting

If Telegram answers with the wrong identity or generic Codex text:

1. Reinstall latest:
```bash
cargo install --git https://github.com/AlbanBeluli/tinyvegeta --force
```
2. Restart daemon:
```bash
tinyvegeta restart
```
3. Check active binary:
```bash
which tinyvegeta
```
4. Verify settings path and agent workspace:
```bash
cat ~/.tinyvegeta/settings.json
```

Notes:
- TinyVegeta loads `SOUL.md` from agent workspace first (`~/tinyvegeta-workspace/<agent>/SOUL.md`).
- Fallback path is `~/.tinyvegeta/SOUL.md`, then `~/ai/tinyvegeta/SOUL.md` (or `TINYVEGETA_DEFAULT_SOUL` if set).
- Telegram identity replies are guarded at runtime to prevent `"I'm Codex"` leakage.

### Path + Workspace Awareness

TinyVegeta should always reason from the **active agent workspace** and state that path-aware context when relevant.

Expected runtime context:

- Agent working directory: `settings.agents.<id>.working_directory`
- Workspace root: `settings.workspace.path`
- Runtime config: `~/.tinyvegeta/settings.json`
- Agent context files: `<working_directory>/SOUL.md`, `<working_directory>/MEMORY.md`

Behavior rules:

- No `@agent` prefix routes to deterministic default agent (`assistant` preferred).
- `@team_id` routes to the team leader.
- The assistant should not act like it's in an empty folder when files exist in its `working_directory`.

Quick verification:

```bash
cat ~/.tinyvegeta/settings.json | jq '.workspace.path, .agents.assistant.working_directory'
tinyvegeta board show
tinyvegeta memory explain "current workspace path" --agent assistant --team board 6
```

### Board Automation

```bash
# Board schedules (persisted in settings, executed by heartbeat)
tinyvegeta board schedule daily --time 09:00 --team-id board
tinyvegeta board schedule digest --time 18:00 --agent assistant
tinyvegeta board schedule list
tinyvegeta board schedule remove <schedule-id>

# Decisions export
tinyvegeta board decisions export --format markdown --file board-decisions.md
tinyvegeta board decisions export --format json --file board-decisions.json
```

### Doctor Coverage

`tinyvegeta doctor` now checks:
- `routing.default_agent` validity + resolvable default agent
- workspace path / agent working_directory consistency
- missing `SOUL.md`/`MEMORY.md` in agent workspaces
- board/team consistency (leader/member references)
- provider CLIs installed + Cline auth probe
- stale tmux daemon session state

### Service Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta telegram` | Run Telegram bot (foreground) |
| `tinyvegeta heartbeat [--agent <id>] [--verbose]` | Run heartbeat daemon or single-agent heartbeat check |
| `tinyvegeta web [--port PORT]` | Start web server |

### Deterministic Routing + Contracts

- Queue processing now uses deterministic routing when no explicit `@agent` is provided.
- Typed routing schema is applied: `intent`, `owner`, `priority`, `deadline`.
- Hard assignment rules map intents to specialist agents (`coder`, `security`, `operations`, `marketing`, `seo`, `sales`) with deterministic fallback.
- Provider calls run under execution contracts (timeout + retry + failure code classification).

### SQLite Operational Memory

Operational memory is now persisted in SQLite at:

```text
~/.tinyvegeta/memory/events.db
```

Tables:

- `events`
- `decisions`
- `outcomes`

Per-session summaries are produced from these records and written to global memory keys like:

- `session.<session_id>.summary`

### Sovereign Runtime (New)

Project structure:

```text
constitution/LAWS.md        # immutable laws
src/sovereign/mod.rs        # sovereign loop + guardrails + audit
docs/SOVEREIGN_RUNTIME.md   # design + configuration
```

Usage:

```bash
# Start safe validation run
tinyvegeta sovereign --agent assistant --goal "improve tinyvegeta safely" --dry-run --max-cycles 3

# Continuous loop (non-dry)
tinyvegeta sovereign --agent assistant --goal "ship autonomous improvements"
```

Audit:

- File: `~/.tinyvegeta/audit/sovereign.jsonl`
- Every thought/action is logged with cycle/status/details.

Config keys in `~/.tinyvegeta/settings.json`:

- `sovereign.constitution_path`
- `sovereign.protected_files`
- `sovereign.loop_sleep_seconds`
- `sovereign.max_actions_per_cycle`
- `sovereign.max_self_modifications_per_hour`
- `sovereign.allow_tool_install`
- `sovereign.allow_self_modify`

### Other Commands

| Command | Description |
|---------|-------------|
| `tinyvegeta update` | Update to latest version |
| `tinyvegeta uninstall --yes [--purge-data]` | Uninstall |

## Telegram In-Chat Commands

| Command | Description |
|---------|-------------|
| `/help` | Show help |
| `/agent` | List agents |
| `/team` | List teams |
| `/board` | Show board config |
| `/board discuss <topic>` | Run board discussion with formatted decision output |
| `/status` | Show daemon/tmux status |
| `/restart` | Restart TinyVegeta daemon remotely |
| `/doctor` | Run remote diagnostics summary |
| `/provider [name]` | Show current provider or switch provider |
| `/models [name]` | Alias for provider switching |
| `/memory stats` | Show memory statistics |
| `/memory search <query>` | Search memory quickly |
| `/brain show` | Show `BRAIN.md` |
| `/brain status` | Show proactive-check status |
| `/brain add <text>` | Append action/note to `BRAIN.md` |
| `/logs <telegram\|queue\|heartbeat\|all> [lines]` | Tail filtered logs |
| `/gateway [status\|restart]` | Alias gateway controls |
| `/releasecheck` | Run release readiness checks |
| `/sovereign status` | Show remote sovereign runtime status |
| `/sovereign start [@agent] [goal...] [--dry-run]` | Start sovereign runtime remotely |
| `/sovereign stop` | Stop sovereign runtime remotely |
| `/soul [@agent]` | Start SOUL edit mode |
| `/soul show [@agent]` | Show SOUL.md preview |
| `/soul cancel` | Cancel SOUL edit mode |
| `/reset @agent [@agent2...]` | Reset specific agent conversations |
| `/reset` | Reset conversation |
| `/triage [on\|off\|status]` | Manage auto-routing |
| `@agent_id message` | Route to specific agent |
| `@team_id message` | Route to team leader |

### Telegram Task Feedback

When you send a normal task message, Telegram now returns lifecycle feedback:

- queued (`📥 Task ... queued`)
- started (`⚙️ Task ... started`)
- completed (`✅ Task ... complete`)

This removes the need to ask separately whether a task started or finished.

### Proactive Brain Stack

TinyVegeta now supports a proactive workspace stack in `~/ai/tinyvegeta`:

- `SOUL.md`
- `IDENTITY.md`
- `USER.md`
- `TOOLS.md`
- `BRAIN.md` (live working memory)
- `MEMORY.md` (long-term memory)
- `HEARTBEAT.md`
- `CLIENTS.md`
- `PLAYBOOK.md`
- `AGENTS.md`

Workspace folders:

- `memory/`
- `skills/` (`x-post-writer`, `website-dev`, `script-polish`, `security-auditor`)
- `content/`
- `consulting/`
- `drafts/`
- `crm/`

### Heartbeat Runtime (Production)

Heartbeat is now wired as active maintenance, not a passive ping:

- Runs `doctor --fix` on cadence
- Checks queue pressure, tmux state, provider availability, disk space, SQLite size
- Flags stale agents and high failure rates
- Cleans stale pending pairing requests
- Runs global memory compaction on daily cadence
- Writes heartbeat records to:
  - SQLite ops memory (`events`/`outcomes`)
  - `~/.tinyvegeta/audit/heartbeat.jsonl`
- Updates global health fields visible in `status`:
  - `heartbeat.last_timestamp`
  - `heartbeat.health_score`
  - `heartbeat.last_actions`
  - `heartbeat.last_warnings`

### Telegram Media Ingestion

Telegram messages with attachments are now ingested and passed to agents:
- photos, documents, audio, voice, video, video notes, stickers
- files are stored under `~/.tinyvegeta/files`
- prompts include `[file: <path>]` references so agents can act on them

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     tinyvegeta (Rust Binary)                     │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  CLI Cmd    │  │    Queue    │  │   Memory    │              │
│  │  Dispatch   │  │  Processor  │  │   System    │              │
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

- **File-based queue** - Atomic operations, no race conditions
- **Three-layer memory** - Global, agent, and task scopes
- **Parallel agents** - Different agents process concurrently
- **Isolated workspaces** - Each agent has own directory

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

## Directory Structure

```
~/.tinyvegeta/                    # Runtime data
├── settings.json                 # Configuration
├── pairing.json                  # Sender allowlist
├── queue/                        # Message queue
│   ├── incoming/
│   ├── processing/
│   └── outgoing/
├── logs/                         # All logs
├── memory/                       # Memory store
│   ├── global.json
│   ├── agents/
│   └── tasks/
└── files/                        # Uploaded files

~/tinyvegeta-workspace/           # Agent workspaces
├── coder/
├── writer/
└── assistant/
```

## Configuration

Settings are stored in `~/.tinyvegeta/settings.json`:

```json
{
  "telegram": {
    "bot_token": "YOUR_BOT_TOKEN"
  },
  "workspace": {
    "path": "/Users/you/tinyvegeta-workspace"
  },
  "agents": {
    "assistant": {
      "name": "Assistant",
      "provider": "claude",
      "model": "sonnet",
      "working_directory": "/Users/you/tinyvegeta-workspace/assistant"
    }
  },
  "teams": {
    "dev": {
      "name": "Development Team",
      "agents": ["coder", "reviewer"],
      "leader_agent": "coder"
    }
  },
  "heartbeat_interval": 3600
}
```

## Troubleshooting

```bash
# Check status
tinyvegeta status

# Run diagnostics
tinyvegeta doctor --fix

# View logs
tinyvegeta logs all

# Reset queue
tinyvegeta stop
rm -rf ~/.tinyvegeta/queue/processing/*
tinyvegeta start
```

**Common Issues:**

- **Telegram not receiving** → Check bot token with `tinyvegeta doctor`
- **Messages stuck** → Clear processing queue
- **Provider not found** → Ensure CLI is installed and in PATH

## Latest Runtime Notes (2026-02-18)

- Telegram + Cline: TinyVegeta now strips Cline JSON event stream output and sends only final assistant text.
- Default routing is explicit via `routing.default_agent` and validated on startup.
- Prompt runtime context now includes:
  - `agent_id`
  - `working_directory`
  - `workspace_root`
  - `team_id`
  - `board_id`
- `tinyvegeta doctor` now verifies:
  - SOUL/MEMORY presence in each agent workspace
  - working_directory/workspace mismatches
  - provider CLI presence
  - Cline auth probe (with timeout)
  - stale tmux daemon session state

## Development

```bash
# Build
cargo build

# Release build (4.7MB binary)
cargo build --release

# Run tests
cargo test

# Check code
cargo clippy
```

## Credits

- Inspired by [OpenClaw](https://openclaw.ai/)
- Built with [Axum](https://github.com/tokio-rs/axum), [Teloxide](https://github.com/teloxide/teloxide)
- AI providers: [Claude](https://claude.ai/), [OpenAI](https://openai.com/), [xAI](https://x.ai/), [Ollama](https://ollama.com/)

## License

MIT

---

**TinyVegeta — The Prince of All AI Agents. Unlimited Power.**
