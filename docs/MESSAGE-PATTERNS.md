# Message Patterns (Current)

## Direct agent

```text
@coder fix login bug
```

Routes to `coder` and processes in coder workspace.

## Team-to-leader

```text
@board plan launch week
```

Resolves team leader (`assistant` by default).

## CEO delegation tags

Leader can delegate in response with:

```text
[@operations: draft rollout checklist]
[@marketing: campaign brief]
[@security: pre-launch risk review]
```

TinyVegeta executes delegated tasks and appends a `Board Delegation Results` block.

## Board discussion

CLI and Telegram discussion flows synthesize member inputs through the leader:

- `tinyvegeta board discuss "topic"`
- `/discuss <topic>`
