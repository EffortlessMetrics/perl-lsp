---
name: coding-standards
description: Load perl-lsp coding standards for code-writing or code-review work in this repo. Use when implementing, fixing, reviewing, or validating code.
user-invocable: false
---

# Coding Standards

This skill is the full on-demand coding standards reference for agents. Hooks may
inject a condensed summary automatically, but that Layer 0 context is not a
replacement for the complete standards below.

## Banned In Production Code

- `unwrap()`, `expect()` → use `?`, `.ok_or_else()`, or pattern matching
- `panic!()`, `todo!()`, `unimplemented!()` → return `Result`/`Option`
- `dbg!()` → use `tracing::debug!`
- `std::process::abort()` → never
- `std::process::exit()` → only in `bin/` directories and `lifecycle.rs`
- Exception: one `#[allow(clippy::expect_used)]` in `crates/perl-lsp/src/util/uri.rs`

## Patterns

- Regex init: `Option<Regex>` with `.ok()` for graceful degradation
- Non-empty collections: fixed-size arrays `[T; N]` for compile-time guarantees
- `.first()` over `.get(0)`
- `.push(char)` not `.push_str("x")` for single chars
- `or_default()` not `or_insert_with(Vec::new)`
- No unnecessary `.clone()` on Copy types

## Test Standards

- Return `Result<()>` from tests, or use `perl_tdd_support::must` / `must_some`
- Descriptive names: `test_<what>_<scenario>_<expected>`
- LSP tests: `RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2`

## Commit Format

- Conventional: `type(scope): description`
- Types: `fix`, `feat`, `test`, `docs`, `chore`, `perf`, `refactor`
- Scope: crate name, subsystem, or repo area being changed

## Verification

Run `cargo fmt --all && cargo clippy -p <crate> --tests -- -D warnings && cargo test -p <crate>`

Escalate to `nix develop -c just ci-gate` for changes spanning 3+ crates.

## Git Commit Hygiene

- Only `git add` files you intentionally created or modified
- Never use `git add -A` or `git add .`
- Exclude from commits unless your task requires them:
  - `Cargo.lock` unless dependencies changed
  - `.claude/` infrastructure files unless that is the task
  - `docs/project/CURRENT_STATUS.md`
  - `scripts/.ignored-baseline`
- Before committing, run `git diff --cached --name-only`

## Dual Indexing

```rust
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());
file_index.references.entry(qualified).or_default().push(symbol_ref);
```
