# perl-lsp-diagnostics

Diagnostics and linting providers for Perl LSP.

## Scope

- Converts parser and semantic issues into LSP diagnostics.
- Runs lint passes (common mistakes, deprecated patterns, strict warnings).
- Supports dead-code detection and diagnostic deduplication.

## Public Surface

- Core diagnostics provider/types from `diagnostics` and `types` modules.
- Lint entry points under `lints::*`.
- `detect_dead_code` for dead-code analysis.

## Workspace Role

Internal feature crate used by `perl-lsp` publish-diagnostics flow.

## License

MIT OR Apache-2.0.
