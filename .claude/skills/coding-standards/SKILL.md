---
name: coding-standards
description: Load perl-lsp coding standards for code-writing or code-review work in this repo. Use when implementing, fixing, reviewing, or validating code.
user-invocable: false
---

# Coding Standards

Load these standards before writing or reviewing code in `perl-lsp`.

## Banned In Production Code

- no `unwrap()`, `expect()`, `panic!()`, `todo!()`, or `unimplemented!()`
- no `dbg!()`
- no `std::process::abort()`
- no `std::process::exit()` outside `bin/` and `lifecycle.rs`
- one allowed exception: `#[allow(clippy::expect_used)]` in `crates/perl-lsp/src/util/uri.rs`

Prefer `?`, `.ok_or_else()`, or explicit pattern matching.

## Common Patterns

- use `Option<Regex>` with `.ok()` for graceful regex init
- use fixed-size arrays for compile-time non-empty guarantees
- prefer `.first()` over `.get(0)`
- prefer `.push(char)` over `.push_str("x")` for a single character
- prefer `or_default()` over `or_insert_with(Vec::new)`
- avoid unnecessary `.clone()` on `Copy` types

## Test Standards

- prefer `Result<()>` return types in tests, or `perl_tdd_support::must` / `must_some`
- use descriptive names like `test_<what>_<scenario>_<expected>`
- for `perl-lsp` tests use `RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2`

## Commit Format

- use conventional commits: `type(scope): description`
- types: `fix`, `feat`, `test`, `docs`, `chore`, `perf`, `refactor`
- scope should usually be the crate or subsystem

## Default Verification

```bash
cargo fmt --all
cargo clippy -p <crate> --tests -- -D warnings
cargo test -p <crate>
```

Escalate to `nix develop -c just ci-gate` for broader multi-crate changes.

## Git Hygiene

- stage only files you intentionally changed
- never use `git add -A` or `git add .`
- normally exclude `Cargo.lock`, `.claude/` control-plane files, `docs/project/CURRENT_STATUS.md`, and `scripts/.ignored-baseline` unless they are the direct task
- check the staged set with `git diff --cached --name-only`

## Dual Indexing Reminder

Workspace symbol work should preserve both bare and qualified indexing:

```rust
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());
file_index.references.entry(qualified).or_default().push(symbol_ref);
```
