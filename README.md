# why

Ask your Linux system why.

`why` explains where things on a Linux system came from, what owns them,
what depends on them, and where they are configured.

## Prototype

```bash
why command git
why package git
why service docker
why file /run/current-system/sw/bin/git
why process 1234
why port 8080
why --json command git
```

Current resolvers:

- commands
- packages
- systemd services
- files and symlinks
- processes through `/proc`
- listening TCP and UDP ports

The first release is intentionally small. The core model is designed to grow
into settings, devices, configuration provenance, dependency trees, and history.

## Principles

- Evidence before guesses
- Plain explanations over raw command output
- Read-only by default
- Useful interactively and from JSON
- Distribution-aware without becoming distribution-specific spaghetti

## Development

This repository includes a Nix development shell.

```bash
nix develop
cargo run -- command git
cargo test
cargo clippy -- -D warnings
```

## Status

Early prototype. NixOS and Arch Linux are the initial targets.
