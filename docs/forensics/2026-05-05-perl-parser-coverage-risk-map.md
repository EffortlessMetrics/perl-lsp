# Parser Coverage Risk Map — 2026-05-05

## Scope

This map defines where parser coverage increases should focus first for the parser crate group (`perl-parser`, `perl-parser-core`) and where lower coverage is acceptable for now.

This document is intentionally for *coverage triage*, not parser behavior changes.
It does not add new enforceable coverage floors.

## Coverage command and baseline

- Recipe: `just coverage-parser`
- Measured scope: `perl-parser --lib` and `perl-parser-core --lib`
- Coverage artifact: `target/coverage/parser.lcov`
- Current enforced parser coverage gate: `.ci/coverage-baseline.txt`
- Advisory risk policy file: `.ci/coverage/parser-baseline.json`

The checked-in gate remains the existing parser-lib ratchet in `.ci/coverage-baseline.txt`.
This risk map records where parser-core and future all-target coverage should focus once those lanes are stable enough to ratchet.

Current blocker for broadening beyond lib coverage:

- `perl-parser --all-targets` coverage exposes existing integration failures in `tests/error_classifier_tests.rs`.

That blocker should be fixed in behavior/test-focused PRs before adding enforceable all-target floors.

## Risk classification

### 1) Critical parser behavior (highest priority)

Target high branch coverage because these code paths determine parse correctness and syntax tree shape.

- `crates/perl-parser-core/src/engine/parser/statements.rs`
- `crates/perl-parser-core/src/syntax/heredoc.rs`
- Parser expression and statement dispatch paths in `crates/perl-parser-core/src/engine/parser/`

Expected policy stance:

- Candidate per-file branch floors on known-risk files (`statements.rs`, `heredoc.rs`) after fresh coverage proof
- Trend upward over time; do not broaden crate-wide gates prematurely

### 2) Recovery and error handling (highest priority)

Coverage here correlates directly with live-edit reliability and post-error salvage behavior.

Focus areas:

- Error-node construction and resynchronization branches
- End-of-input/incomplete-block handling
- Recovery spillover boundaries in statement parsing

Expected policy stance:

- Prefer branch coverage over line coverage as the primary signal
- Add targeted malformed fixtures before raising floors

### 3) Span and position correctness (high priority)

Coverage should emphasize edge branches affecting byte/line/UTF-16 mapping and downstream LSP coordinates.

Focus areas:

- Span propagation across recovered nodes
- Position mapping helpers used by parser-facing APIs
- Newline/offset edge cases

Expected policy stance:

- Treat branch gaps as significant when they can produce wrong editor coordinates

### 4) Incremental parsing (high priority)

Coverage should prioritize invalidation and equivalence branches that can silently diverge from full parse behavior.

Focus areas:

- Incremental cache invalidation conditions
- Fast-path vs fallback parse equivalence branches
- Stitching boundaries after edits

Expected policy stance:

- Ratchet branch coverage only after deterministic fixture coverage exists

### 5) Facade and re-export glue (low priority)

Lower coverage is acceptable where code mostly re-exports or forwards parser-core behavior.

Focus areas:

- `pub use`-heavy facade modules
- Thin wrappers with no branching behavior of their own

Expected policy stance:

- Avoid spending high-value test budget chasing these lines first

### 6) Deprecated compatibility surface (low priority)

Coverage is useful for regression protection but should not dominate parser trust work.

Focus areas:

- Legacy aliases and compatibility exports
- Deprecated API adapters maintained for migration

Expected policy stance:

- Keep smoke/regression coverage; prioritize critical parser behavior first

### 7) Generated or static data (excluded/very low)

Generated tables and static mappings should generally be excluded from risk-weighted coverage goals.

Expected policy stance:

- Do not use generated/static files to justify parser trust improvements

## Operator guidance

When coverage is below target, prioritize work in this order:

1. Critical parser behavior
2. Recovery/error handling
3. Span/position correctness
4. Incremental parsing
5. Facade glue and compatibility surface

This keeps parser trust improvements aligned with user-visible correctness rather than raw percentage growth.
