# TinyVegeta Installation (Rust)

## Requirements

- macOS or Linux
- `tmux`
- Rust toolchain (only if installing via Cargo)
- At least one provider runtime:
  - `claude` CLI
  - `codex` CLI
  - `cline` CLI
  - `opencode` CLI
  - or Ollama / Grok API config

## Install

### From Cargo (recommended for latest main)

```bash
cargo install --git https://github.com/AlbanBeluli/tinyvegeta --force
```

### From source

```bash
git clone https://github.com/AlbanBeluli/tinyvegeta.git
cd tinyvegeta
cargo build --release
cp target/release/tinyvegeta ~/.cargo/bin/tinyvegeta
```

## First-time setup

```bash
tinyvegeta setup
```

Setup creates:

- `~/.tinyvegeta/settings.json`
- `~/tinyvegeta-workspace/*` agent workspaces
- default board pack (CEO + specialist agents)

## Start/Stop

```bash
tinyvegeta start
tinyvegeta stop
tinyvegeta restart
tinyvegeta status
```

## Provider switching

```bash
tinyvegeta provider cline
tinyvegeta provider cline --model claude-sonnet-4-20250514
tinyvegeta restart
```

For CLI providers (`claude`, `codex`, `cline`, `opencode`):

- model `default` means TinyVegeta does not force `--model`
- provider CLI's own selected/default model is used

## Telegram pairing

1. DM bot to receive pairing code
2. Approve:

```bash
tinyvegeta pairing pending
tinyvegeta pairing approve <CODE>
```
