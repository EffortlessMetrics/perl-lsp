# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-diagnostics` is a **Tier 2 LSP feature crate** providing diagnostic generation and linting for Perl code.

**Purpose**: Converts parse errors, scope analysis issues, and lint findings into structured diagnostics consumed by the LSP server.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-lsp-diagnostics      # Build this crate
cargo test -p perl-lsp-diagnostics       # Run tests
cargo clippy -p perl-lsp-diagnostics     # Lint
cargo doc -p perl-lsp-diagnostics --open # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` -- AST types, parse errors, error classifier, position tracking
- `perl-semantic-analyzer` -- scope analysis (`ScopeAnalyzer`, `SymbolTable`)
- `perl-workspace-index` -- workspace-wide symbol index for dead code detection
- `perl-diagnostics-codes` -- stable diagnostic code definitions
- `perl-pragma` -- `PragmaTracker` for pragma-aware analysis
- `perl-position-tracking` -- byte offset / line-column conversion

### Key Types and Modules

| Module | Public | Purpose |
|--------|--------|---------|
| `types` | `Diagnostic`, `DiagnosticSeverity`, `DiagnosticTag`, `RelatedInformation` | Core diagnostic data types |
| `diagnostics` | `DiagnosticsProvider` | Main provider: parse error conversion + scope analysis |
| `lints/common_mistakes` | `check_common_mistakes` | Assignment-in-condition, numeric comparison with undef |
| `lints/deprecated` | `check_deprecated_syntax` | `defined @array`, `$[` variable |
| `lints/strict_warnings` | `check_strict_warnings` | Missing `use strict` / `use warnings` |
| `dead_code` | `detect_dead_code` | Workspace-wide unused symbol detection (cfg: not wasm32) |
| `dedup` | (internal) | `deduplicate_diagnostics` -- sorts and removes duplicates |
| `error_nodes` | (internal) | ERROR node classification with suggestions |
| `walker` | (internal) | Pre-order AST walker for lint traversal |
| `scope` | (internal) | `scope_issues_to_diagnostics` -- maps `ScopeIssue` to `Diagnostic` |

### DiagnosticsProvider API

```rust
use perl_lsp_diagnostics::DiagnosticsProvider;

// Construct with an AST and source text
let provider = DiagnosticsProvider::new(&ast, source);

// Generate diagnostics from AST and parse errors
let diagnostics = provider.get_diagnostics(&ast, &parse_errors, &source_text);
```

`get_diagnostics` performs:
1. Parse error to diagnostic conversion
2. Pragma tracking via `PragmaTracker::build`
3. Scope analysis via `ScopeAnalyzer::analyze`
4. Scope issue to diagnostic mapping (severity, codes, related info, tags)

### Diagnostic Codes

| Code | Source | Severity |
|------|--------|----------|
| `parse-error` | Parser | Error |
| `undeclared-variable` | Scope | Error |
| `variable-redeclaration` | Scope | Error |
| `duplicate-parameter` | Scope | Error |
| `unquoted-bareword` | Scope | Error |
| `variable-shadowing` | Scope | Warning |
| `unused-variable` | Scope | Warning |
| `unused-parameter` | Scope | Warning |
| `parameter-shadows-global` | Scope | Warning |
| `uninitialized-variable` | Scope | Warning |
| `assignment-in-condition` | Lint | Warning |
| `numeric-undef` | Lint | Warning |
| `deprecated-defined` | Lint | Warning |
| `deprecated-array-base` | Lint | Warning |
| `missing-strict` | Lint | Information |
| `missing-warnings` | Lint | Information |
| `dead-code-*` | Workspace | Hint |

### Diagnostic Tags

| Tag | Applied to |
|-----|-----------|
| `Unnecessary` | `unused-variable`, `unused-parameter`, `dead-code-*` |
| `Deprecated` | `deprecated-defined`, `deprecated-array-base` |

## Important Notes

- `dead_code` module is gated behind `#[cfg(not(target_arch = "wasm32"))]`
- Lint functions take `&Node` (AST root) + output `&mut Vec<Diagnostic>` -- they walk the AST internally
- `check_common_mistakes` also requires a `&SymbolTable` for undef analysis
- The `walker::walk_node` is a simple pre-order traversal covering `Program`, `Block`, `If`, `While`, `Binary`, `FunctionCall`, and `ExpressionStatement` node kinds
- Several internal functions are `#[allow(dead_code)]` as they are building blocks for future integration
