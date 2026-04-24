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

## Workspace Role

Tier 1 leaf crate. Depends only on `perl-ast`. Consumed by
`perl-parser-core` and `perl-lsp-diagnostics` to provide scope-aware
pragma analysis for parsing and diagnostic flows.

## Benchmarking

Run the benchmark suite (Criterion) for build/query-heavy pragma workloads:

```bash
cargo bench -p perl-pragma --bench pragma_benchmarks
```

Or run all benches for this crate:

```bash
cargo bench -p perl-pragma
```

The suite includes stable benchmark IDs so total runtime and per-iteration
cost can be compared over time:

- `build_small_file`
- `build_large_file`
- `query_random_offsets`
- `query_monotonic_offsets`
- `final_state_lookup`
- `version_compat_walk_style`
- `scope_analyzer_walk_style`

## License

MIT OR Apache-2.0
