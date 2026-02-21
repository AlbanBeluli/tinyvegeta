# Troubleshooting (Rust)

## Wrong binary running

```bash
which tinyvegeta
```

If stale, reinstall:

```bash
cargo install --git https://github.com/AlbanBeluli/tinyvegeta --force
```

## Telegram replies but wrong identity/provider behavior

```bash
tinyvegeta restart
cat ~/.tinyvegeta/settings.json
```

Check `agents.assistant.provider` and `model`.

## Team/board missing

Runtime now self-heals defaults. Force once:

```bash
tinyvegeta agent pack install default
tinyvegeta restart
```

Verify:

```bash
tinyvegeta board show
tinyvegeta team list
```

## Queue stuck

```bash
tinyvegeta queue stats
tinyvegeta queue recover
```

## Provider not found

Install/check provider CLI:

```bash
claude --version
codex --version
cline --version
opencode --version
```

## tmux daemon issues

```bash
tinyvegeta stop
tinyvegeta start
tmux ls
```

If needed, recreate session manually:

```bash
tmux kill-session -t tinyvegeta
```
