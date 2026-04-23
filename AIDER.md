# AIDER.md

Guidance for contributors using [Aider](https://aider.chat/) on this repository.

## Start here

1. Read [`AGENTS.md`](AGENTS.md) first (this is the canonical implementation-agent playbook).
2. Run:
   ```bash
   git log --oneline -20
   ```
   so you don't duplicate recently merged work.
3. Keep each PR scoped to one concern.

## Required coding/PR standards

Follow all rules in [`AGENTS.md`](AGENTS.md), including:

- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `dbg!()`.
- Tests should return `Result<()>` or use `perl_tdd_support::must` helpers.
- Use `static LazyLock<Regex>` for regexes, not per-call construction.
- Add justification comments for every `#[allow(...)]`.
- Do not hold locks across `.await`.

## Verification commands (before opening a PR)

```bash
cargo test -p <crate>
cargo check --all-targets -p <crate>
cargo xtask fmt
cargo clippy -p <crate>
just pr-fast
```

## Commit and PR format

- Single focused commit.
- Commit title format:
  `type(scope): description (#NNNN)`
- If issue number is unknown, use `#0000`.

PR body template:

```text
Problem: <one sentence>
Fix: <one sentence>
Verification: `cargo test -p <crate>` passes / `just pr-fast` passes
```

## Truth sources (do not hardcode metrics)

- [`Cargo.toml`](Cargo.toml) — workspace members, package versions
- [`docs/project/CURRENT_STATUS.md`](docs/project/CURRENT_STATUS.md) — evidence-backed metrics
- [`docs/project/ROADMAP.md`](docs/project/ROADMAP.md) — canonical roadmap
- [`features.toml`](features.toml) — LSP capability catalog
