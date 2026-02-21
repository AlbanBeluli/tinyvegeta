# Sovereign Runtime (Rust v1)

This is the initial Rust implementation of a continuously running sovereign loop.

## Project Structure

```text
constitution/
  LAWS.md                  # immutable constitutional laws
src/sovereign/
  mod.rs                   # loop engine + action executor + guards + audit
```

## Runtime Loop

Each cycle:

1. Think: call provider with constitution + runtime context + mission.
2. Act: parse JSON actions and execute guarded operations.
3. Observe: audit-log and memory-log outcomes.
4. Repeat: sleep and continue.

Heartbeat daemon is spawned in parallel from CLI command so schedules continue while loop sleeps.

## Command

```bash
tinyvegeta sovereign --agent assistant --goal "improve reliability" --dry-run
tinyvegeta sovereign --agent assistant --goal "ship improvements" --max-cycles 20
```

## Safety Controls

Configured via `settings.sovereign`:

- `constitution_path`: optional override path.
- `protected_files`: optional blocked write targets.
- `loop_sleep_seconds`
- `max_actions_per_cycle`
- `max_self_modifications_per_hour`
- `allow_tool_install` (defaults to `true`)
- `allow_self_modify` (defaults to `true`)

Always blocked:

- dangerous shell patterns (`rm -rf /`, fork bomb, disk format patterns)

## Audit

Every cycle/action is appended to:

```text
~/.tinyvegeta/audit/sovereign.jsonl
```

This log is creator-readable and intended for forensic review.
