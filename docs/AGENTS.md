# Agents (Rust)

Agents are defined in `~/.tinyvegeta/settings.json`.

## Agent config

Each agent supports:

- `name`
- `provider` (`claude|codex|cline|opencode|ollama|grok`)
- `model`
- `working_directory`
- `is_sovereign`

## Context loading

Per message, context is loaded from:

1. `<agent_workdir>/SOUL.md`
2. `~/.tinyvegeta/SOUL.md`
3. `~/ai/tinyvegeta/SOUL.md` (or `TINYVEGETA_DEFAULT_SOUL`)

Also loads `MEMORY.md` and `AGENTS.md` when present.

## Workspaces

Default root: `~/tinyvegeta-workspace/`

Each agent gets:

- `SOUL.md`
- `MEMORY.md`
- optional role overlay from default pack templates

## Commands

```bash
tinyvegeta agent list
tinyvegeta agent show <agent_id>
tinyvegeta agent pack list
tinyvegeta agent pack install default
```

## Routing

- `@agent_id message` routes to that agent
- `@team_id message` routes to team leader
- no prefix routes to default/first configured agent
