# why

Ask your Linux system why.

`why` explains where things on a Linux system came from, what owns them,
what depends on them, and where they are configured.

## Prototype

Most questions need no resolver name:

```bash
why git
why :8080
why 1234
why ./Cargo.toml
why this
```

Explicit resolvers remain available when a name is ambiguous:

```bash
why command git
why package git
why service docker
why file /run/current-system/sw/bin/git
why process 1234
why port 8080
why --json git
```

Current resolvers cover commands, packages, systemd services, files and symlinks,
processes through `/proc`, listening ports, Git-backed file provenance, and
basic project context.

Facts carry evidence metadata in JSON. Terminal output marks inferred facts
instead of presenting them as confirmed.

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
cargo run -- git
cargo test
cargo clippy -- -D warnings
```

## Status

Early prototype. NixOS and Arch Linux are the initial targets.
