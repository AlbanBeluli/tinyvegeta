# Teams and Board

Teams are configured in `~/.tinyvegeta/settings.json` and route via `@team_id`.

## Current behavior

- `@team_id message` resolves to team `leader_agent`
- leader response can delegate to teammates using mention tags:

```text
[@coder: implement X]
[@security: review threat model]
```

- TinyVegeta executes teammate delegations and appends outputs.

## Default board pack

Installed during setup (and self-healed at runtime if missing):

- `assistant` (CEO/leader)
- `coder`
- `security`
- `operations`
- `marketing`
- `seo`
- `sales`

Board defaults:

- team id: `board`
- leader: `assistant`
- `board.autonomous: true`

## Commands

```bash
tinyvegeta team list
tinyvegeta team add
tinyvegeta team show <team_id>
tinyvegeta team remove <team_id>
tinyvegeta team visualize [team_id]

tinyvegeta board show
tinyvegeta board create --ceo assistant --autonomous
tinyvegeta board discuss "topic"
```

## Telegram

- `/team` lists teams
- `/board` shows board config
- `/discuss <topic>` runs board discussion
