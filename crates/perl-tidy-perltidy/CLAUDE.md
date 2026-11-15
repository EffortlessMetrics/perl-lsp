# CLAUDE.md (perl-tidy-perltidy)

## Role

Explicit subprocess-backed compatibility adapter for `Perl::Tidy`.

## Owns

- `PerlTidyConfig` and command-line argument generation.
- `PerlTidyFormatter`, process invocation, and memoized external output.
- Legacy `BuiltInFormatter` / `FormatSuggestion` compatibility helpers.

## Must not own

- The Rust-native formatter engine.
- LSP request wiring or protocol-coordinate conversion.

## Focused validation

`cargo test -p perl-tidy-perltidy --all-targets`
