<div align="center">

# why

**Ask your Linux system why.**

Explain where things came from, what owns them, what depends on them,
and what your machine is doing with them.

[![CI](https://github.com/aethctl/why/actions/workflows/ci.yml/badge.svg)](https://github.com/aethctl/why/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-black.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-black.svg)](https://www.rust-lang.org/)

</div>

```text
$ why git

╭─ WHY ───────────────────────────────────────────────────────────────╮
│ git                                                                │
│ Type: software package / command                                   │
├─ WHAT IS THIS? ────────────────────────────────────────────────────┤
│ Tracks changes to files and manages Git repositories. It is        │
│ provided by the git-2.55.0 package.                                │
├─ WHY IS IT HERE? ──────────────────────────────────────────────────┤
│ It is present because your Nix configuration declares it directly. │
├─ DETAILS ──────────────────────────────────────────────────────────┤
│ Package         git-2.55.0                                         │
│ Comes from      Declarative Nix configuration                      │
│ Declared in     /etc/nixos/configuration.nix:257                   │
╰────────────────────────────────────────────────────────────────────╯
```
## One question, different evidence

Most queries do not need a subcommand.

```bash
why git
why :8080
why 4312
why ./Cargo.toml
why HOME
why this
```

`why` works out what kind of thing you gave it and follows the evidence
available on the machine.

```text
$ why :8123

8123
port 8123 is currently listening

Protocol         tcp
Local address    127.0.0.1:8123
Process           python3
PID              4312
Executable       /nix/store/...-python3-3.14.7/bin/python3.14
Provided by      python3-3.14.7
```

## What it understands

| Subject | Example | What `why` explains |
| --- | --- | --- |
| Package | `why git` | provider, declaration, referrers, impact |
| Command | `why command git` | executable path and package owner |
| Service | `why service docker` | state, unit file, reverse relationships |
| Process | `why 4312` | command, package, project, ancestry |
| Port | `why :8080` | listener, PID, executable, package |
| File | `why ./config.kdl` | canonical path, owner, Git provenance |
| Environment | `why HOME` | current value and possible source |
| Shell | `why fetch` | alias or function definition and source |
| Project | `why this` | repo, stack, live processes, listening ports |

Explicit resolvers remain available when a name is ambiguous.

```bash
why package git
why command git
why shell fetch
why env XCURSOR_THEME
```

## Go deeper

Normal output stays concise. `--deep` asks for extra evidence.

```bash
why --deep git
why --deep 4312
why --deep ./some-file
```

Depending on the subject, deep mode can add package dependency closures,
child processes, open file descriptor counts, file ownership and permissions,
or richer service state.

For scripts and tooling:

```bash
why --json git
why --plain git
```

Every structured fact includes whether it is **confirmed** or **inferred**.

## Share a safe report

```bash
why --report git > why-report.md
```

Reports are Markdown and are designed for GitHub issues. Home paths and
credential-like values are scrubbed before output.

## Install

### Nix

Run without installing:

```bash
nix run github:aethctl/why -- git
```

Install into your profile:

```bash
nix profile install github:aethctl/why
```

### Cargo

```bash
cargo install --git https://github.com/aethctl/why why-linux
```

### From source

```bash
git clone https://github.com/aethctl/why
cd why
nix develop
cargo build --release
```

## Platform support

NixOS and Arch Linux are the first-class package targets for v0.1.

Generic Linux resolvers such as processes, ports, files, systemd services,
environment variables and project context work independently where their
underlying system interfaces are available.

## Design rules

`why` is intentionally read-only.

It does not repair, remove, disable, or rewrite your system. Its job is to
explain what is there and show the evidence behind that explanation.

The project follows a few rules:

- evidence before guesses
- concise output by default
- inferred claims are labelled
- distro-specific behavior lives behind shared concepts
- machine-readable output is a first-class interface
- sensitive values do not belong in bug reports

## Development

```bash
nix develop
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
nix build
```

## License

MIT
