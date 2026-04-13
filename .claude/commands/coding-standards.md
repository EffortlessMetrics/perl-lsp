---
description: Load perl-lsp coding standards for builders and reviewers
argument-hint: ""
user-invocable: false
---

# Coding Standards (perl-lsp)

Invoke this slash entrypoint to load project coding standards into your
context.

## Banned in Production Code (tests exempt)
- `unwrap()`, `expect()` → use `?`, `.ok_or_else()`, or pattern matching
- `panic!()`, `todo!()`, `unimplemented!()` → return `Result`/`Option`
- `dbg!()` → use `tracing::debug!`
- `std::process::abort()` → never
- `std::process::exit()` → only in `bin/` directories and `lifecycle.rs`
- **Exception**: One `#[allow(clippy::expect_used)]` in `crates/perl-lsp/src/util/uri.rs`

## Patterns
- Regex init: `Option<Regex>` with `.ok()` for graceful degradation
- Non-empty collections: fixed-size arrays `[T; N]` for compile-time guarantees
- `.first()` over `.get(0)`
- `.push(char)` not `.push_str("x")` for single chars
- `or_default()` not `or_insert_with(Vec::new)`
- No unnecessary `.clone()` on Copy types

## Test Standards
- Return `Result<()>` from tests, or use `perl_tdd_support::must`/`must_some`
- Descriptive names: `test_<what>_<scenario>_<expected>`
- LSP tests: `RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2`

## Commit Format
- Conventional: `type(scope): description`
- Types: `fix`, `feat`, `test`, `docs`, `chore`, `perf`, `refactor`
- Scope: crate name (e.g., `parser`, `lsp`, `dap`)

## Verification (per-crate default)
```bash
cargo xtask fmt
cargo clippy -p <crate> --tests -- -D warnings
cargo test -p <crate>
```

Escalate to `nix develop -c just ci-gate` only for changes spanning 3+ crates.

## Git Commit Hygiene
- Only `git add` files you intentionally created or modified
- **NEVER** use `git add -A` or `git add .` — these capture unintended changes
- Specifically **EXCLUDE** from commits:
  - `Cargo.lock` (unless your change modifies dependencies — worktree drift causes false conflicts)
  - `.claude/` infrastructure files (unless that's your task)
  - `docs/project/CURRENT_STATUS.md` (auto-generated)
  - `scripts/.ignored-baseline` (auto-generated)
- Use `git add <specific-files>` or `git add crates/<your-crate>/`
- Before committing, run `git diff --cached --name-only` to verify only intended files are staged
- If unintended files are staged, unstage with `git reset HEAD <file>`

## Dual Indexing (workspace features)
```rust
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());
file_index.references.entry(qualified).or_default().push(symbol_ref);
```
