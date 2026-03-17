---
name: parser-fix
description: Run a TDD parser-fix workflow for perl-parser-core or related parser crates. Use when a parser bug has a concrete failing construct and needs tests, a minimal fix, and verification.
argument-hint: "<bug description>"
---

# Parser Fix

Fix a parser bug with a failing test first. Bug: **$ARGUMENTS**

## Workflow

1. Find the relevant parser code under `crates/perl-parser-core/src/engine/parser/`
2. Add failing tests first in the right parser test file
3. Implement the smallest fix that makes the new tests pass
4. Verify the parser crates cleanly
5. Return a concise receipt for the handoff or PR

## Root-Cause Search

Start with the file surface most likely to own the construct:

- `variables.rs`
- `statements.rs`
- `expressions/postfix.rs`
- `expressions/precedence.rs`
- `declarations.rs`

Understand why the construct fails before editing.

## Test-First Rule

Add tests before the fix. Prefer crate-local parser tests that parse a Perl
snippet and assert success or the intended AST shape.

## Verification

```bash
cargo fmt --all
cargo clippy -p perl-parser-core --lib
cargo test -p perl-parser-core
cargo test -p perl-parser
```

Keep the fix narrow. If the failure turns out to be primarily lexer or corpus
work, route it into the right worker instead of silently widening the slice.
