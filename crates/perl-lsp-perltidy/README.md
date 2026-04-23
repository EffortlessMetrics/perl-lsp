# perl-lsp-perltidy

Standalone SRP microcrate for `perltidy`-based Perl formatting integration.

## Features

- `PerlTidyConfig` for serializable formatter configuration
- `PerlTidyFormatter` for subprocess-backed formatting with memoized results
- `BuiltInFormatter` fallback for environments without `perltidy`
- Range formatting and simple formatting suggestion generation
- Argument-injection-safe file formatting via `--` separator

## Workspace Role

Tier 2 tooling microcrate in the Perl LSP workspace. `perl-lsp-tooling` re-exports this crate for backward compatibility.

## License

MIT OR Apache-2.0
