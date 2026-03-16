---
description: Review code for security issues — banned constructs, input validation, path traversal, supply chain
argument-hint: "<PR-number-or-branch>"
---

# Security Review

Review PR **$ARGUMENTS** through a security lens.

## Checklist

- [ ] No `unwrap()/expect()/panic!()` in production code
- [ ] No `unsafe` blocks without documentation and necessity justification
- [ ] Path inputs validated against traversal (no `..` escape)
- [ ] UTF-16 / UTF-8 position conversions are symmetric
- [ ] No `std::process::exit()` outside `bin/` and `lifecycle.rs`
- [ ] No hardcoded secrets or credentials
- [ ] File operations use safe path handling
- [ ] External input sanitized at system boundaries
- [ ] `deny.toml` policy not weakened

## Key Standards

- **Exception**: `perl-lsp/src/util/uri.rs` has one allowed `#[allow(clippy::expect_used)]`
- Regex: use `Option<Regex>` with `.ok()` for graceful degradation
- Tests may use `unwrap()` if they return `Result<()>`

## Process

1. Get changed files: `gh pr diff $ARGUMENTS --stat`
2. Review each changed file against the checklist
3. Pay extra attention to files handling external input (LSP requests, DAP commands, file I/O)
4. Report findings with severity (critical/high/medium/low) and line references
