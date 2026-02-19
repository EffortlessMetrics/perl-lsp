# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-lsp-code-actions` is a **Tier 2 LSP feature crate** (per workspace Cargo.toml grouping) that provides quick fixes and refactoring code actions for the Perl LSP server.

**Purpose**: Generate LSP code actions from diagnostics and AST analysis -- quick fixes for common Perl mistakes, and refactoring operations for code improvement.

**Version**: 0.9.0 (workspace-inherited)

## Commands

```bash
cargo build -p perl-lsp-code-actions          # Build this crate
cargo test -p perl-lsp-code-actions           # Run tests
cargo clippy -p perl-lsp-code-actions         # Lint
cargo doc -p perl-lsp-code-actions --open     # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` -- AST types (`Node`, `NodeKind`, `Parser`, `SourceLocation`)
- `perl-position-tracking` -- Position handling (declared but used transitively)
- `perl-refactoring` -- Refactoring infrastructure (declared but used transitively)
- `perl-workspace-index` -- Workspace symbol index (declared but used transitively)
- `perl-semantic-analyzer` -- Semantic analysis (declared but used transitively)
- `perl-lsp-rename` -- `TextEdit` type for edit payloads
- `perl-lsp-diagnostics` -- `Diagnostic` and `DiagnosticSeverity` types
- `serde`, `thiserror` -- Serialization and error handling

### Key Types and Modules

| Type / Module | Location | Purpose |
|---------------|----------|---------|
| `CodeActionsProvider` | `code_actions.rs` | Main provider: maps diagnostic codes to quick fix functions |
| `EnhancedCodeActionsProvider` | `enhanced/mod.rs` | Advanced provider: AST-aware refactorings |
| `CodeAction` | `types.rs` | Result type with title, kind, edits, diagnostics |
| `CodeActionKind` | `types.rs` | Enum: QuickFix, Refactor, RefactorExtract, RefactorInline, RefactorRewrite, Source, SourceOrganizeImports, SourceFixAll |
| `CodeActionEdit` | `types.rs` | Wrapper around `Vec<TextEdit>` |
| `QuickFixDiagnostic` | `types.rs` | Internal simplified diagnostic with byte-offset range |
| `quick_fixes` | `quick_fixes.rs` | All quick fix implementations |
| `refactors` | `refactors.rs` | Basic refactoring actions (delegates to enhanced) |
| `ast_utils` | `ast_utils.rs` | AST walking helpers: `find_node_at_range`, `find_statement_start`, `get_indent_at` |
| `enhanced::extract_variable` | `enhanced/extract_variable.rs` | Smart extract-to-variable with name suggestion |
| `enhanced::extract_subroutine` | `enhanced/extract_subroutine.rs` | Extract block to subroutine with parameter detection |
| `enhanced::loop_conversion` | `enhanced/loop_conversion.rs` | C-style for to foreach conversion |
| `enhanced::import_management` | `enhanced/import_management.rs` | Organize and add missing `use` statements |
| `enhanced::postfix` | `enhanced/postfix.rs` | Convert `if` blocks to postfix form |
| `enhanced::error_checking` | `enhanced/error_checking.rs` | Add `or die` to file operations |
| `enhanced::helpers` | `enhanced/helpers.rs` | `Helpers` struct for position finding and text utilities |

### Quick Fix Codes Handled

`undefined-variable`, `undeclared-variable`, `unused-variable`, `assignment-in-condition`, `missing-strict`, `missing-warnings`, `deprecated-defined`, `numeric-undef`, `unquoted-bareword`, `parse-error-*` (semicolon, string, parenthesis, bracket, brace), `unused-parameter`, `variable-shadowing`.

## Usage

```rust
use perl_lsp_code_actions::{CodeActionsProvider, EnhancedCodeActionsProvider};
use perl_parser_core::Parser;

// Diagnostic-driven quick fixes
let provider = CodeActionsProvider::new(source.to_string());
let actions = provider.get_code_actions(&ast, (start, end), &diagnostics);

// AST-driven refactorings
let enhanced = EnhancedCodeActionsProvider::new(source.to_string());
let refactorings = enhanced.get_enhanced_refactoring_actions(&ast, (start, end));
```

## Important Notes

- Quick fixes are keyed on `Diagnostic.code` string values
- `CodeActionsProvider::get_code_actions` also calls into `EnhancedCodeActionsProvider` via `refactors::get_refactoring_actions`
- The `enhanced` module recursively walks the AST to collect all applicable actions within a range
- Actions produce `CodeActionEdit` containing `Vec<TextEdit>` with byte-offset `SourceLocation`
- Dev-dependency `perl-tdd-support` provides `must`/`must_some` test helpers (avoids `unwrap`)
- Doctests are disabled (`doctest = false` in Cargo.toml)
