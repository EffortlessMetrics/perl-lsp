# ADR-XXXX: Replace match-arm panic! with assert_matches! in test code

## Status
Proposed

## Context

Issue [#3258](https://github.com/EffortlessMetrics/perl-lsp/issues/3258) requests replacing `panic!()` calls in test match-arm error handling with `assert_matches!()` macro from `std::assert_matches`. This is a spinoff from comprehensive test-code quality audit (#3237).

### The Problem

The codebase has `panic!` calls in test match-arm catch handlers that predate Rust 1.75's `assert_matches!` stabilization. These patterns:
- Are less idiomatic than `assert_matches!`
- Provide string-formatted panic messages instead of structured assertion failures
- Are inconsistent with the project's production code quality bar (AGENTS.md bans `panic!`)

### Issue Claims vs Reality

| Metric | Issue Claims | Actual Findings |
|--------|-------------|-----------------|
| Total occurrences | 180 | ~126 |
| Files affected | 24 | ~14 |
| Crates affected | 10 | 5-6 |
| perl-builtins | 19 | 0 (not match-arm catches) |
| perl-dap | 39 | 40 |
| perl-parser-core | 73 | 87 |
| perl-lsp | 8 | 3 |

The issue's counts were from a different codebase state or included non-match-arm panics.

## Decision

Replace match-arm `panic!` calls in test code with `assert_matches!` from `std::assert_matches`, using the following scope rules:

### Scope: What's Changed

1. **Target patterns**:
   - `_ => panic!("...")` in test match arms
   - `other => panic!("...")` in test match arms

2. **Target crates**:
   - `perl-parser-core` (~87 occurrences, ~70%)
   - `perl-dap` (~40 occurrences, ~32%)
   - `perl-lsp` (~3 occurrences, ~2%)

3. **PR Strategy**: 2 PRs
   - PR 1: `perl-parser-core` (largest scope)
   - PR 2: `perl-dap` + `perl-lsp` (combined)

### Scope: What's Excluded

1. **perl-test-must helpers**: `must()`, `must_some()`, `must_err()` in `crates/perl-test-must/src/lib.rs` are intentionally designed to panic as test infrastructure. They are excluded entirely.

2. **Setup failure patterns**: `Err(e) => panic!("Failed to load fixtures: {e}")` patterns represent test setup failures, not assertion failures. Using `assert_matches!` would silently pass if fixtures fail to load — semantically wrong. These are preserved as-is.

3. **Non-match-arm panics**: Standalone `panic!` calls in `if` conditions, for-loops, or other contexts are not in scope.

4. **`unreachable!()`**: Issue explicitly says do not touch (267 occurrences).

### Baseline File

Create `ci/panic_test_baseline.txt` with the actual pre-replacement count (~126), not `0` as the issue originally specified. The issue's instruction to use `0` was backwards — a baseline of `0` is meaningless for tracking.

### Refactoring Pattern

```rust
// BEFORE:
match value {
    ExpectedVariant { .. } => { /* check it */ }
    _ => panic!("Expected ExpectedVariant, got {:?}", other),
}

// AFTER:
use std::assert_matches::assert_matches;
assert_matches!(value, ExpectedVariant { .. });
```

For cases with variable binding and assertions:

```rust
// BEFORE:
match adapter.handle_request(1, "continue", None) {
    DapMessage::Response { success, .. } => assert!(success),
    other => panic!("expected Response for {expected_command}, got {:?}", other),
}

// AFTER:
use std::assert_matches::assert_matches;
let resp = adapter.handle_request(1, "continue", None);
assert_matches!(resp, DapMessage::Response { success: true, .. });
```

## Consequences

### Positive
- Establishes `assert_matches!` as the canonical pattern for enum variant assertions
- Produces better structured failure diagnostics when tests fail
- Extends the project's code quality bar (AGENTS.md) to test code
- Reduces inconsistency between production code (`panic!` banned) and test code

### Negative / Tradeoffs
- Some diagnostic panic messages (e.g., `"Expected X, got {:?}", value`) will be lost. Where diagnostic context is valuable, preserve via `eprintln!` or comments.
- `#[allow(clippy::panic)]` directives may become unnecessary but require per-file analysis before removal.
- Count uncertainty (±10 changes) could cause scope confusion during implementation.

### Risks
1. **Semantic mismatch**: `Err/Ok => panic!` patterns for setup failures must NOT be replaced with `assert_matches!`. Builder must evaluate each individually.
2. **Helper function restructuring**: Most panics are in inline test assertions, not helper functions. The ~3 panics in perl-test-must are excluded entirely.
3. **Allow-lint removal**: Only remove `#[allow(clippy::panic)]` after verifying all panics in that scope have been replaced.

## Alternatives Considered

### Alternative 1: Do Nothing
Leave existing `panic!` patterns as-is. Rejected because issue #3258 explicitly requests this change and it's aligned with the project's existing quality bar.

### Alternative 2: Inline-only Replacement
Only replace panics in truly inline test assertions (~35-40 cases). Rejected because the full scope (~126 cases) is manageable in 2 PRs and provides complete coverage.

### Alternative 3: Per-Helper-Function Refactoring First
Refactor helper functions before migrating to `assert_matches!`. Rejected because perl-test-must helpers are intentionally designed to panic and excluded, while other helper patterns are mechanically replaceable.

## References

- Issue: [#3258](https://github.com/EffortlessMetrics/perl-lsp/issues/3258)
- Parent Issue: [#3237](https://github.com/EffortlessMetrics/perl-lsp/issues/3237)
- AGENTS.md (project coding guidance): bans `panic!` in production code (line 95)
- `std::assert_matches::assert_matches`: stable since Rust 1.75