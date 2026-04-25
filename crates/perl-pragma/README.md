# perl-pragma

Pragma state tracking for Perl source analysis.

## Overview

`perl-pragma` walks a `perl-ast` AST to track `use strict`, `no strict`,
`use warnings`, and `no warnings` statements. It builds a range-indexed
pragma map so callers can query the effective pragma state at any byte offset
in the source.

## Public API

- **`PragmaState`** -- tracks `strict_vars`, `strict_subs`, `strict_refs`,
  and `warnings` booleans. Provides `all_strict()` and `Default`.
- **`PragmaTracker`** -- walks an AST via `build()` to produce a sorted
  `Vec<(Range<usize>, PragmaState)>`, and offers `state_for_offset()` to
  query it.

## Benchmarks

Run the Criterion benchmark suite for build-heavy and query-heavy pragma workloads:

```bash
cargo bench -p perl-pragma
```

Benchmarks include stable names that can be diffed over time:

- `build_small_file`
- `build_large_file`
- `query_random_offsets`
- `query_monotonic_offsets`
- `final_state_lookup`
- `version_compat_walk_style`
- `scope_analyzer_walk_style`

Criterion reports per-benchmark timing statistics (including time per iteration and total sample estimates), which can be compared between runs.

## Workspace Role

Tier 1 leaf crate. Depends only on `perl-ast`. Consumed by
`perl-parser-core` and `perl-lsp-diagnostics` to provide scope-aware
pragma analysis for parsing and diagnostic flows.

## License

MIT OR Apache-2.0
