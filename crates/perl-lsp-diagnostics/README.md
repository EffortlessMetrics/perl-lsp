# perl-lsp-diagnostics

LSP diagnostics provider for Perl. Generates editor-visible diagnostics from
parse errors, scope analysis, lint checks, and workspace-wide dead code detection.

## Features

- **Parse error diagnostics** -- converts parser errors into positioned diagnostics
- **Scope analysis** -- undeclared variables, unused variables/parameters, shadowing, redeclaration
- **Lint passes** -- common mistakes (assignment in condition, numeric undef), deprecated syntax (`defined @array`, `$[`), missing `use strict`/`use warnings`
- **Dead code detection** -- workspace-wide unused subroutine/variable/constant/package detection (non-WASM only)
- **Deduplication** -- removes duplicate diagnostics by range, severity, code, and message
- **ERROR node classification** -- classifies AST error nodes with suggestions and explanations

## Public API

| Export | Description |
|--------|-------------|
| `DiagnosticsProvider` | Core provider -- builds diagnostics from AST and parse errors |
| `Diagnostic`, `DiagnosticSeverity`, `DiagnosticTag`, `RelatedInformation` | Diagnostic types |
| `common_mistakes::check_common_mistakes` | Assignment-in-condition and numeric-undef checks |
| `deprecated::check_deprecated_syntax` | Deprecated syntax detection |
| `strict_warnings::check_strict_warnings` | Missing pragma advisories |
| `detect_dead_code` | Workspace-wide dead code detection (non-WASM) |

## Workspace Role

Internal feature crate consumed by `perl-lsp` to publish diagnostics to editors.
Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## License

MIT OR Apache-2.0
