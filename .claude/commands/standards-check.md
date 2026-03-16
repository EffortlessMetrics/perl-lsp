---
description: Review code for project coding standards, conventional commits, and crate boundary compliance
argument-hint: "<PR-number-or-branch>"
---

# Standards Check

Review PR **$ARGUMENTS** for project standards compliance.

## Coding Standards

- [ ] No `unwrap()/expect()/panic!()/todo!()/unimplemented!()/dbg!()` in production
- [ ] `std::process::exit()` only in `bin/` and `lifecycle.rs`
- [ ] `std::process::abort()` never
- [ ] Regex: `Option<Regex>` with `.ok()`
- [ ] `.first()` over `.get(0)`
- [ ] `.push(char)` over `.push_str("x")`
- [ ] `or_default()` over `or_insert_with(Vec::new)`
- [ ] No `.clone()` on Copy types
- [ ] `tracing::debug!` instead of `dbg!()`

## Commit Standards

- [ ] Conventional commits: `type(scope): description`
- [ ] Types: `fix`, `feat`, `test`, `docs`, `chore`, `perf`, `refactor`
- [ ] Scope: crate name (e.g., `parser`, `lsp`, `dap`)
- [ ] Messages match actual changes

## Crate Boundary Rules

- [ ] Changes respect tiered dependency structure
- [ ] No upward dependencies (tier N depending on tier N+1)
- [ ] No unintended new dependencies in `Cargo.toml`

## Process

1. Get changed files: `gh pr diff $ARGUMENTS --stat`
2. Review each file against coding standards checklist
3. Check commit messages: `gh pr view $ARGUMENTS --json commits`
4. Verify crate boundaries are respected
5. Report violations with file and line references
