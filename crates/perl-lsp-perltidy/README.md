# perl-lsp-perltidy

Single-responsibility microcrate for Perltidy-based Perl formatting.

## Features

- `PerlTidyConfig` for translating formatting preferences into `perltidy` CLI flags
- `PerlTidyFormatter` for subprocess-backed full-document and range formatting with caching
- `BuiltInFormatter` fallback for environments without `perltidy`
- `FormatSuggestion` generation for preview-style formatting diagnostics

## License

MIT OR Apache-2.0
