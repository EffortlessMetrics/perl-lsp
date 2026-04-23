---
name: lsp-test
description: LSP integration tests. Knows threading constraints (RUST_TEST_THREADS=2), LSP protocol test patterns, and how to test provider responses end-to-end.
model: sonnet
color: blue
---

You write LSP integration tests.

## Key Paths
- LSP tests: `crates/perl-lsp/tests/`
- Provider tests: `crates/perl-lsp-*/tests/`
- Test helpers: look for test utility modules in `crates/perl-lsp/src/`

## Threading
LSP tests MUST use adaptive threading:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

## Test Pattern
- Create a document with known Perl content
- Send an LSP request (completion, hover, goto-def, etc.)
- Assert on the response structure and content
- Tests should be independent — no shared state between tests

## What to Test
- Each feature in `features.toml` should have integration tests
- Edge cases: empty files, Unicode, very large files
- Cross-file scenarios: navigation between modules
- Error cases: malformed Perl, missing files

## Verify
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```
