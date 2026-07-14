# Context — #1916 pragma `:default` feature bundle baseline

## Problem

`PragmaState::default()` seeded `features` empty. Perl's `:default` bundle
(`indirect`, `multidimensional`, `bareword_filehandles`,
`apostrophe_as_package_separator`, `smartmatch`) is enabled before any
`use feature`/`use VERSION`, so `has_feature` was wrong at file scope.

## Key facts (verified against perldoc.perl.org/feature)

- `:default` bundle == `version::DEFAULT_FEATURES` (already defined in-crate).
- `use v5.36` disables `indirect`, `multidimensional` (and removes `switch`).
- `use v5.38` additionally disables `bareword_filehandles`; adds `module_true`.
- `use v5.42` disables `apostrophe_as_package_separator`, `smartmatch`.
- bare `no feature` resets to `DEFAULT_FEATURES`; `use vX.Y` *replaces* features.

## Files

- `crates/perl-pragma/src/lib.rs` — custom `Default`, `all_strict()`.
- `crates/perl-pragma/src/version.rs` — `DEFAULT_FEATURES` (reused, unchanged).
- Tests: `tests/default_feature_bundle_tests.rs` (new),
  `tests/comprehensive_unit_tests.rs` (two contract tests strengthened).

## Consumers verified green

`perl-parser-core` (HIR `PragmaStateFact`), `perl-lsp-rs-core`,
`tree-sitter-perl-rs`. No consumer queries default-on features by name; all use
`has_feature("say")`-style checks (`say` is not in `:default`), so behavior is
unchanged for them while the model becomes spec-faithful.
