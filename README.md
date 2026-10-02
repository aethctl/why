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
why HOME
why this
```

`why this` understands the current project and can surface its branch,
repository state, detected stack, live project processes, and listening ports.

Explicit resolvers remain available when a name is ambiguous:

```bash
why command git
why package git
why service docker
why file /run/current-system/sw/bin/git
why process 1234
why port 8080
why env XCURSOR_THEME
why --json git
why --plain this
```

Current resolvers cover commands, packages, systemd services, files and symlinks,
processes through `/proc`, listening ports, environment variables, Git-backed
file provenance, and current-project context.

Facts carry evidence metadata in JSON. Terminal output marks inferred facts
instead of presenting them as confirmed. Credential-like environment values
are redacted automatically.

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
