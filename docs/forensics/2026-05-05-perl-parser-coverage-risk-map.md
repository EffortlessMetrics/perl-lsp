# Perl Parser Coverage Risk Map (2026-05-05)

## Scope

This map is intentionally limited to parser-facing crates:

- `crates/perl-parser`
- `crates/perl-parser-core`

The goal is to separate high-impact uncovered logic from low-impact facade surface so future test work can improve parser trust before parser percentage.

## Classification rubric

- **Critical parser behavior**: syntax and statement/expression parsing where wrong branches produce wrong trees.
- **Recovery/error handling**: broken-input behavior and salvage logic that determines live-edit quality.
- **Span/position correctness**: byte/line/UTF-16 mapping and exact ranges consumed by editor providers.
- **Incremental parsing**: cache invalidation and reparse path correctness.
- **Facade/re-export glue**: public surface aggregation with little direct behavior.
- **Deprecated compatibility surface**: transitional aliases and legacy exports.
- **Generated/static data**: tables and static declarations where direct unit coverage value is low.

## Risk map

### 1) Critical parser behavior (highest priority)

Representative paths to prioritize for branch coverage growth:

- `crates/perl-parser-core/src/engine/parser/statements.rs`
- `crates/perl-parser-core/src/engine/parser/expressions.rs`
- `crates/perl-parser-core/src/syntax/heredoc.rs`
- `crates/perl-parser-core/src/syntax/quotes.rs`

Why this matters:

- Branch gaps here produce wrong AST structure even when parse succeeds.
- Heredoc and quote-like branches are already known cluster candidates in parser status tracking.

### 2) Recovery/error handling (highest priority)

Representative paths:

- `crates/perl-parser-core/src/engine/parser/recovery.rs`
- `crates/perl-parser-core/src/engine/parser/statements.rs` (error-restart arms)
- `crates/perl-parser/src/parser_recovery.rs`

Why this matters:

- Branch gaps here drive spillover failures and post-error symbol loss in live editing.
- A clean parse percentage cannot detect broken salvage behavior.

### 3) Span/position correctness (high priority)

Representative paths:

- `crates/perl-parser/src/position_mapper.rs`
- `crates/perl-parser/src/span_utils.rs`
- `crates/perl-parser-core/src/ast/span.rs`

Why this matters:

- Wrong byte/line/UTF-16 mapping causes bad diagnostics, hover ranges, and goto definition placement.
- Coverage should emphasize edge-case branches (CRLF boundaries, multibyte UTF-8, empty spans).

### 4) Incremental parsing (high priority)

Representative paths:

- `crates/perl-parser/src/incremental.rs`
- `crates/perl-parser/src/incremental_cache.rs`

Why this matters:

- Branch gaps can cause stale or divergent parse results between full and incremental paths.
- These are correctness-sensitive even if they are not syntax-specific.

### 5) Facade/re-export glue (low priority)

Representative paths:

- `crates/perl-parser/src/lib.rs`
- `crates/perl-parser/src/compat.rs`

Why this matters less:

- These modules are mostly `pub use` surface and compatibility routing.
- Low branch coverage here should not be treated as equivalent to parser-core branch gaps.

### 6) Deprecated compatibility surface (low priority)

Representative paths:

- `crates/perl-parser/src/deprecated.rs`
- compatibility aliases in `crates/perl-parser/src/lib.rs`

Why this matters less:

- These branches are migration aids.
- They should remain tested enough to avoid breakage, but not dominate parser coverage goals.

### 7) Generated/static data (exclude or de-prioritize)

Representative paths:

- generated keyword/operator tables
- static lookup maps without behavioral branching

Why this matters less:

- Line/branch deltas here rarely correlate with parser trust.
- Keep compilation coverage, but avoid gating parser quality on these files.

## Practical targeting guidance

Use `just coverage-parser` to generate `target/coverage/parser.lcov`, then prioritize branch improvements in this order:

1. Recovery + heredoc/quote parsing.
2. Statement/expression branch arms with known wrong-tree potential.
3. Span/UTF-16 edge branches.
4. Incremental invalidation/equivalence branches.
5. Facade/deprecated branches only when needed for API safety.

## Baseline policy link

Parser-specific floors and critical-file branch floors live in:

- `.ci/coverage/parser-baseline.json`

This keeps parser lane interpretation separate from global coverage numbers.
