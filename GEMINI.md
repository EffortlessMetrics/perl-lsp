# GEMINI.md — Gemini CLI Quick Start for perl-lsp

This repository already includes a full implementation-agent contract in
[`AGENTS.md`](AGENTS.md). If you're using **Gemini CLI**, follow that file as
source of truth and use this document as a Gemini-focused bridge.

## 1) Run preflight checks first

```bash
bash scripts/agent-preflight.sh
```

Fix any failures before editing files.

## 2) Confirm latest upstream context

`AGENTS.md` requires this check before starting work:

```bash
git log origin/master --oneline -20
```

If `origin/master` doesn't exist in your clone, use the local fallback:

```bash
git log --oneline -20
```

## 3) Keep PR scope narrow

- One concern per PR.
- Avoid unrelated cleanup.
- Do not disable or delete existing tests just to pass CI.

## 4) Verification expectations

For the crate you touched, run at least:

```bash
cargo test -p <crate>
cargo check --all-targets -p <crate>
cargo clippy -p <crate>
```

Project-wide fast gate before opening PR:

```bash
just pr-fast
```

## 5) Commit/PR format required by this repo

Single focused commit and title format:

```text
type(scope): description (#NNNN)
```

Use `(#0000)` when issue linkage is unknown.

PR body template:

```text
Problem: <one sentence>
Fix: <one sentence>
Verification: `cargo test -p <crate>` passes / `just pr-fast` passes
```

## 6) Safety rules (important)

- Do **not** use `git stash` in this repo (shared across worktrees).
- Prefer `git restore <file>` for discard and regular commits for WIP.
- Avoid `unwrap()`, `expect()`, `todo!()`, and similar banned shortcuts.

---

If any instruction here conflicts with `AGENTS.md`, follow `AGENTS.md`.
