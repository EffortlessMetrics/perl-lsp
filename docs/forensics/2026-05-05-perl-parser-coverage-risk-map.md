# perl-parser coverage risk map (2026-05-05)

## Scope

This forensic note defines parser-specific coverage priorities so coverage work improves parser trust rather than inflating a single global percentage.

- Crates in scope: `perl-parser`, `perl-parser-core`
- Artifact target: `target/coverage/parser.lcov`
- Baseline policy: `.ci/coverage/parser-baseline.json`

## Risk-classified coverage lanes

### 1) Critical parser behavior (highest priority)

Coverage deficits in these paths can directly produce wrong parse trees or wrong statement boundaries while still reporting a "clean parse":

- `crates/perl-parser-core/src/engine/parser/statements.rs`
- `crates/perl-parser-core/src/syntax/heredoc.rs`
- Quote-like operator handling in `crates/perl-parser-core/src/syntax/`

Coverage work in this lane should focus on branch behavior, not line-only coverage.

### 2) Recovery and error handling (highest priority)

Coverage gaps in recovery branches reduce live-edit stability and can cause cascade failures:

- Unclosed block and delimiter recovery branches
- Error-node boundary and resume-path branches
- Multi-error files where parser should salvage post-error statements

Coverage in this lane should prioritize malformed fixtures and spillover checks.

### 3) Span/position correctness (high priority)

Coverage should validate byte, line, and UTF-16 mapping behavior where parser facts cross the editor boundary:

- Position mapper pathways used by diagnostics and goto
- Node span assignment for delimiter-heavy constructs
- Error-region span boundaries during recovery

### 4) Incremental parsing (high priority)

Coverage should exercise invalidation and equivalence branches where edits mutate parse state:

- Minimal edit invalidation paths
- Fast-path reuse vs full reparse fallback
- Equivalence checks between incremental and fresh parse output

### 5) Facade and re-export glue (low priority)

The `perl-parser` facade intentionally re-exports broad surface area. Low coverage here is expected when branches are shallow and behavior lives downstream.

- `pub use` glue and compatibility aliases
- Thin forwarders into parser-core and semantic layers

This lane should not block parser trust improvements unless branches carry non-trivial behavior.

### 6) Deprecated compatibility surface (low priority)

Coverage in deprecated shims is useful only to avoid regressions during migration windows. It should not dominate parser-focused coverage effort.

### 7) Generated/static data (excluded/monitor only)

Generated tables and static registries should generally be excluded from parser branch-floor decisions unless they contain executable branching logic.

## Operating guidance

1. Use `just coverage-parser` for parser-only branch-aware lcov output.
2. Compare parser coverage changes against `.ci/coverage/parser-baseline.json` rather than global workspace thresholds.
3. Prioritize new tests in this order:
   1. Recovery cascade containment
   2. Heredoc and delimiter ambiguity
   3. Span and UTF-16 correctness
   4. Incremental invalidation/equivalence
   5. Facade glue only when behavioral branches are added

## Non-goals for this slice

- No parser behavior changes.
- No global coverage threshold changes.
- No all-crates coverage gate expansion.
