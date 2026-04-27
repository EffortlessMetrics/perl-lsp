# perl-pragma

Pragma state tracking for Perl source analysis.

## Overview

`perl-pragma` walks a `perl-ast` AST and builds a range-indexed pragma map so callers can query the effective lexical pragma state at any byte offset in a file.

The tracker models far more than a strict/warnings-only surface. It currently tracks:

- strict categories (`vars`, `subs`, `refs`)
- warnings (global plus selectively disabled categories)
- `utf8`
- `encoding`
- `locale` (including optional scope argument)
- feature flags from explicit `use feature`/`no feature` and version bundles (`use vX.Y` / `use 5.xxx`)
- lexical `use builtin` imports

Lexical scope restoration is handled for ordinary blocks and other block-like forms, including eval blocks, package block form, and phase blocks.

## Public API

- **`PerlVersion`** -- parsed major/minor Perl version used for version pragma semantics.
- **`PragmaState`** -- effective lexical state snapshot, including strict/warnings, utf8/encoding/locale, feature flags, and builtin imports.
- **`PragmaTracker`** -- walks an AST via `build()` to produce a sorted `Vec<(Range<usize>, PragmaState)>`, and offers `state_for_offset()` to query it.
- **Version/feature helpers** -- `parse_perl_version`, `version_implies_strict`, `version_implies_warnings`, and `features_enabled_by_version`.

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
`perl-parser-core` and `perl-lsp-diagnostics` for scope-aware pragma analysis.

## License

MIT OR Apache-2.0
