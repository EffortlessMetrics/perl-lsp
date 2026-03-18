---
name: coding-standards
description: Load perl-lsp coding standards for code-writing or code-review work in this repo. Use when implementing, fixing, reviewing, or validating code.
user-invocable: false
---

# Coding Standards

## BANNED in production code

- `unwrap()`, `expect()` → use `?`, `.ok_or_else()`, pattern matching
- `panic!()`, `todo!()`, `unimplemented!()` → return `Result`/`Option`
- `dbg!()` → use `tracing::debug!`
- `std::process::exit()` → only allowed in `bin/` and `lifecycle.rs`
- `std::process::abort()` → never
- One exception: `#[allow(clippy::expect_used)]` in `crates/perl-lsp/src/util/uri.rs`

## Key patterns

1. `.first()` over `.get(0)`
2. `.push(char)` over `.push_str("x")` for single chars
3. `or_default()` over `or_insert_with(Vec::new)`
4. `Option<Regex>` with `.ok()` for graceful regex degradation
5. `Result<()>` return types in tests (or `perl_tdd_support::must`/`must_some`)

## Before committing

Run `cargo fmt --all && cargo clippy -p <crate> --tests -- -D warnings && cargo test -p <crate>`
