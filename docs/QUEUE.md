# Queue System (Rust)

TinyVegeta uses a file-based queue under `~/.tinyvegeta/queue`.

## Layout

```text
~/.tinyvegeta/queue/
  incoming/
  processing/
  outgoing/
```

## Flow

1. Telegram client enqueues to `incoming/`
2. internal queue processor reads each message
3. target agent is resolved (`@agent` or `@team` leader)
4. provider is invoked in agent workspace
5. response is sent back to Telegram
6. message file is removed from `incoming/`

## Message format

`src/core/queue.rs` defines `MessageData` and `QueueFile`.

Key fields:

- `channel`, `sender`, `sender_id`, `message`
- `agent` (optional routing target)
- `response_channel`, `response_chat_id`

## Useful commands

```bash
tinyvegeta queue stats
tinyvegeta queue incoming
tinyvegeta queue processing
tinyvegeta queue outgoing
tinyvegeta queue recover
```

## Notes

- Queue processor currently runs in `start-internal` with Telegram + heartbeat.
- `outgoing/` is retained in code but current Telegram path replies directly after processing.
