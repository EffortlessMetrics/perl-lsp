# perl-lsp-perltidy

Standalone SRP microcrate for Perl formatting integration. The `native` module
contains the Rust-native formatter used by the default LSP formatting path.
`PerlTidyFormatter` remains available as an explicit subprocess-backed
compatibility adapter for projects that still need exact `perltidy` behavior.

## Features

- `PerlFormatter` trait and native `FormatResult` / edit / diagnostic model
- `NativeFormatter` and `FormatDoc` document IR for deterministic native
  pretty-printing
- `PerlTidyConfig` for serializable formatter configuration
- `PerlTidyFormatter` for explicit subprocess-backed compatibility formatting
  with memoized results
- `BuiltInFormatter` legacy fallback for direct crate consumers that still opt
  into the old adapter path without `perltidy`
- Range formatting and simple formatting suggestion generation
- Argument-injection-safe file formatting via `--` separator

## Workspace Role

Tier 2 tooling microcrate in the Perl LSP workspace. `perl-lsp-tooling` re-exports this crate for backward compatibility.

## License

MIT OR Apache-2.0
