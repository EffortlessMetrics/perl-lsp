# Aider Instructions

This file provides quick-start guidance for contributors using [Aider](https://aider.chat/) on this repository.

## Session startup

Before changing code, load core context into Aider:

```text
/read AGENTS.md
/read README.md
/read CONTRIBUTING.md
/read docs/project/ROADMAP.md
/read docs/project/CURRENT_STATUS.md
/read features.toml
```

If you are changing a specific crate, also load that crate's `Cargo.toml` and relevant `src/` files.

## Ground rules

- Keep scope narrow: one concern per PR.
- Prefer canonical sources over hardcoded project metrics:
  - `Cargo.toml` for workspace/package metadata
  - `docs/project/CURRENT_STATUS.md` for live metrics
  - `docs/project/ROADMAP.md` for planning state
  - `features.toml` for capability catalog
- Avoid banned patterns (`unwrap`, `expect`, `todo`, `panic`, debug prints in library code).
- Do not use `git stash` in this repository workflow.

## Verification commands

Run at minimum for the crate you changed:

```bash
cargo test -p <crate>
cargo check --all-targets -p <crate>
cargo xtask fmt
cargo clippy -p <crate>
just pr-fast
```

Before merge, use the canonical local gate:

```bash
nix develop -c just ci-gate
```

## Commit/PR format

- Commit title: `type(scope): description (#NNNN)`
- If issue number is unknown, use `(#0000)`.
- Keep PR body short:

```text
Problem: <one sentence>
Fix: <one sentence>
Verification: <command/results>
```

## Notes for Aider users

- Use `/drop` and targeted `/read` to keep context focused.
- Ask Aider to summarize planned edits before applying them when working across multiple files.
- Prefer small, reviewable patches over broad rewrites.
