# Rust Migration TODO (Gap List vs `~/geta`)

This is the actionable backlog of functionality that existed in `~/geta` (bash/ts) and is still missing or partial in `~/ai/tinyvegeta` (Rust).

## P0 - Core Missing/Stubbed Runtime

- [ ] Implement `cmd_send` (currently stubbed)
  - File: `src/cli/mod.rs`
  - Acceptance: `tinyvegeta send "..."` enqueues and receives response.

- [ ] Implement `cmd_logs` (currently stubbed)
  - File: `src/cli/mod.rs`
  - Acceptance: `tinyvegeta logs telegram|queue|heartbeat|daemon|all` tails filtered logs.

- [ ] Implement `cmd_reset` (currently stubbed)
  - File: `src/cli/mod.rs`
  - Acceptance: `tinyvegeta reset <agent...>` clears per-agent conversation state.

- [ ] Implement `agent add/remove/reset` command handlers (currently TODOs)
  - File: `src/cli/mod.rs`
  - Acceptance: full CRUD lifecycle for agent config + workspace + context init/reset.

- [ ] Implement `channels reset telegram` (currently stubbed)
  - File: `src/cli/mod.rs`
  - Acceptance: re-prompts/writes telegram token safely.

- [ ] Implement `web --stop` (currently prints not implemented)
  - File: `src/cli/mod.rs`
  - Acceptance: cleanly stops running web server process.

## P0 - Telegram Operational Commands Parity

- [ ] Add `/gateway [status|restart]` alias behavior (legacy parity)
  - File: `src/telegram/client.rs`
  - Note: `/status` + `/restart` exist; add alias for migration compatibility.

- [ ] Add `/releasecheck` in Telegram
  - File: `src/telegram/client.rs`
  - Acceptance: returns release readiness summary.

- [ ] Add `/reset @agent_id [@agent_id2 ...]` in Telegram
  - File: `src/telegram/client.rs`
  - Acceptance: per-agent reset from chat.

- [ ] Add `/soul` workflow in Telegram with owner lock
  - File: `src/telegram/client.rs`
  - Acceptance:
    - `/soul`, `/soul @agent`, `/soul show [@agent]`, `/soul cancel`
    - first authorized sender becomes `pairing.soul_owner_sender_id`.

## P0 - Queue + Team Chain Behavior Parity

- [ ] Implement queue-driven recursive teammate handoff chain (not just one-pass leader delegation)
  - Files: `src/cli/mod.rs`, `src/board.rs`, `src/core/*`
  - Acceptance:
    - `[@teammate: ...]` creates new queue messages.
    - Supports fan-out and multi-step chains.

- [ ] Add loop guard + pending indicator
  - Acceptance:
    - max chain cap (e.g. 15 messages),
    - prevent duplicate re-mention spirals while responses pending.

- [ ] Persist team chat history to `~/.tinyvegeta/chats/{team_id}/...`
  - Acceptance: board/team conversations are queryable and replayable.

## P1 - Telegram File/Attachment Support (Major Missing Feature)

- [ ] Download and attach Telegram media/files into queue payload
  - File: `src/telegram/client.rs`
  - Acceptance:
    - supports photos/documents/audio/voice/video/stickers,
    - stores in `~/.tinyvegeta/files/`,
    - includes file refs in agent prompt context.

## P1 - Triage Parity

- [ ] Implement real `/triage on|off|status` state (currently static text)
  - Files: `src/telegram/client.rs`, config model
  - Acceptance:
    - persistent triage toggle in settings,
    - auto-route plain messages when enabled,
    - triage event logging.

- [ ] Port triage heuristics (`findAgentForTriage`) from TS
  - Acceptance: plain-language routing to likely agent with low false positives.

## P1 - Task System (Currently All TODO)

- [ ] Implement `task create/list/show/start/stop/watch/assign/delete/stats`
  - File: `src/cli/mod.rs` + storage module
  - Acceptance: end-to-end task lifecycle with persistence.

- [ ] Integrate tasks with board delegation lifecycle
  - Acceptance: delegations create/update task records.

## P1 - Memory Commands Parity

- [ ] Implement `memory snapshot` subcommands
  - File: `src/cli/mod.rs` + memory module

- [ ] Implement `memory inherit` subcommands
  - File: `src/cli/mod.rs` + memory module

- [ ] Implement `memory export`
  - File: `src/cli/mod.rs`

## P2 - Doctor Parity (Safe Remediation from Legacy)

- [ ] Add queue quarantine for malformed JSON + zero-byte cleanup
  - Acceptance: bad files moved to `logs/queue-quarantine`.

- [ ] Add restrictive permissions fix pass
  - Acceptance: runtime dirs/file permissions hardened on `doctor --fix`.

- [ ] Add pairing file auto-create compatibility checks
  - Acceptance: repairs missing/invalid pairing file safely.

- [ ] Add board default schedule auto-provision on fix (if board exists and no schedules)
  - Acceptance: installs sane daily/digest defaults.

## P2 - Web Feature Gap (Large)

- [ ] Decide scope: minimal Rust web API parity vs full legacy dashboard parity
  - Legacy had:
    - auth/JWT/CSRF/audit/backup,
    - todos + subtasks/comments/time tracking,
    - analytics/search/preferences,
    - file upload/download/editor,
    - SSE + WebSocket.

- [ ] Implement chosen subset with explicit milestone plan (v1/v2/v3)
  - Files: `src/web/*`

## P2 - Team Visualizer Gap

- [ ] Rebuild `team visualize` in Rust (TUI or web fallback)
  - Legacy source: `src/visualizer/team-visualizer.tsx`
  - Acceptance: live chain graph + queue depth + agent status.

## P3 - Update/Release Parity

- [ ] Implement real `releasecheck` (currently static success text)
  - Acceptance: validates runtime wiring, provider availability, queue health.

- [ ] Add optional startup update-check behavior (legacy had hourly)
  - Acceptance: opt-out via env flag.

## Suggested Execution Order

1. P0 runtime stubs + Telegram parity commands
2. P0 queue-chain behavior + chat history
3. P1 file attachments + triage + tasks
4. P2 doctor remediation parity
5. P2 web + visualizer scope decision and staged delivery
6. P3 release/update polish
