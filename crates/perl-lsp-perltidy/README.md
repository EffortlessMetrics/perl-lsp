# perl-lsp-perltidy

Standalone SRP microcrate for Perl formatting integration. The existing
`PerlTidyFormatter` remains a subprocess-backed compatibility adapter; the
`native` module defines the Rust-native formatter contract for the native-first
replacement lane.

## Features

- `PerlFormatter` trait and native `FormatResult` / edit / diagnostic model
- `PerlTidyConfig` for serializable formatter configuration
- `PerlTidyFormatter` for subprocess-backed formatting with memoized results
- `BuiltInFormatter` fallback for environments without `perltidy`
- Range formatting and simple formatting suggestion generation
- Argument-injection-safe file formatting via `--` separator

## Workspace Role

Tier 2 tooling microcrate in the Perl LSP workspace. `perl-lsp-tooling` re-exports this crate for backward compatibility.

## License

MIT OR Apache-2.0
