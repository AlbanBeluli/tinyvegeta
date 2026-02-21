# HEARTBEAT.md

Living system health and autonomous maintenance document.
Executed by `tinyvegeta heartbeat` and the sovereign runtime.

## Purpose

Heartbeat is not a ping. It is TinyVegeta's autonomic maintenance loop:

- Detect drift, staleness, failures, and improvement opportunities
- Execute high-leverage fixes safely
- Persist event/decision/outcome telemetry
- Keep the system healthy and evolving continuously

## Core Loop

1. Read and load context
- Load `SOUL.md`, `constitution/LAWS.md`, `MEMORY.md`, `AGENTS.md`, `BRAIN.md`, and this file.
- Load queue stats, board schedules, sovereign settings, and recent operational memory.

2. Vital signs check
- Run `tinyvegeta doctor --fix` on cadence.
- Evaluate queue pressure and system backlog.
- Verify tmux daemon state and recover if missing.
- Verify provider availability for configured agents.
- Track agent freshness (last-success recency).
- Verify disk capacity and SQLite health.

3. Stale/broken/overdue detection
- Flag stale agents and repeated failures.
- Handle old pending pairing requests.
- Detect placeholder/stale items in `BRAIN.md`.

4. Highest-leverage action
- Execute maintenance that has highest health impact first:
  - queue pressure mitigation
  - stale-agent reset flags
  - memory compaction
  - schedule execution and followups

5. Log and persist
- Write heartbeat event/outcome to SQLite operational memory.
- Append audit record to `~/.tinyvegeta/audit/heartbeat.jsonl`.
- Update status fields:
  - `heartbeat.last_timestamp`
  - `heartbeat.health_score`
  - `heartbeat.last_actions`
  - `heartbeat.last_warnings`

6. Sleep and backoff
- Default loop cadence from runtime settings.
- Backoff automatically when repeated failures occur.

## Emergency Actions

- On critical degradation: mark warnings, persist telemetry, and keep recovery attempts active.
- Never bypass constitution/protected policy constraints.
- All autonomous changes must be auditable and reversible.

## Metrics

- Health score (0-100)
- Last heartbeat timestamp
- Actions taken this cycle
- Warnings this cycle
- Queue depth
- Agent freshness/failure signals

## Manual Runs

```bash
tinyvegeta heartbeat
tinyvegeta heartbeat --agent assistant --verbose
tinyvegeta logs heartbeat
```
