# why

Ask your Linux system why.

`why` explains where things on a Linux system came from, what owns them,
what depends on them, and where they are configured.

## Current prototype

```bash
why command git
why package git
why service docker
why --json command git
```

The first release is intentionally small. The core model is designed to grow
into files, processes, ports, devices, settings, and deeper provenance.

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

Early prototype. The command, service, and package resolvers are being built
first, with NixOS and Arch Linux as the initial targets.
