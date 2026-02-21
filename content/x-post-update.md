# TinyVegeta Update Post (X)

TinyVegeta just got a major reliability + autonomy upgrade.

What changed:
- Deterministic routing: tasks are now assigned with hard rules (`intent`, `owner`, `priority`, `deadline`) instead of random drift.
- SQLite operational memory: events, decisions, and outcomes are now persisted and queryable.
- Execution contracts: per-agent timeout/retry/failure codes to prevent silent hangs.
- Better observability: `tinyvegeta status` now shows queue depth + agent health + last success/error.
- Telegram parity upgrades: `/doctor`, `/provider`, `/memory`, `/logs`, `/soul`, `/sovereign`, `/brain`.
- Real task lifecycle feedback in Telegram: queued -> started -> complete/fail.
- Media ingestion: Telegram attachments are downloaded and passed into task context.
- Sovereign runtime v1: continuous think -> act -> observe loop with audit logging.
- Full local filesystem operation fixed: removed workspace-only execution restriction.
- Production heartbeat upgrade: active maintenance loop with health scoring + auto checks + audit trail.

New proactive stack:
- `SOUL.md`, `BRAIN.md`, `IDENTITY.md`, `USER.md`, `TOOLS.md`, `HEARTBEAT.md`, `CLIENTS.md`, `PLAYBOOK.md`
- Skills + workspace structure for autonomous execution and follow-up.

This release is focused on one thing: make TinyVegeta ship reliably, not just talk.

Install/update:
`cargo install --git https://github.com/AlbanBeluli/tinyvegeta --force`

Then restart:
`tinyvegeta restart`
