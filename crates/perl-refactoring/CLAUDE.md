# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

- **Tier**: 3 (two-level internal dependencies)
- **Version**: 0.9.1
- **Purpose**: Refactoring and modernization utilities for Perl -- provides automated code transformations including import optimization, symbol rename, module extraction, inline variable, and legacy code modernization.

## Commands

```bash
cargo build -p perl-refactoring          # Build
cargo test -p perl-refactoring           # Run tests
cargo clippy -p perl-refactoring         # Lint
cargo doc -p perl-refactoring --open     # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` -- AST types, parser, `Node`, `NodeKind`, `SourceLocation`, `LineIndex`
- `perl-workspace-index` -- `WorkspaceIndex`, `SymKind`, `SymbolKey`, URI/path helpers

### Features

| Feature | Default | Description |
|---------|---------|-------------|
| `workspace_refactor` | yes | Enables `WorkspaceRefactor` in `RefactoringEngine` |
| `modernize` | no | Enables `PerlModernizer` in `RefactoringEngine` |
| `workspace-rename-tests` | no | Integration tests for workspace rename |

### Key Types and Modules

| Module | Key Types | Purpose |
|--------|-----------|---------|
| `refactor::refactoring` | `RefactoringEngine`, `RefactoringConfig`, `RefactoringResult`, `RefactoringType`, `RefactoringScope`, `BackupInfo` | Unified engine coordinating all refactoring operations with backup/rollback |
| `refactor::import_optimizer` | `ImportOptimizer`, `ImportAnalysis`, `UnusedImport`, `MissingImport`, `TextEdit` | Analyze and optimize Perl `use`/`require` statements |
| `refactor::modernize` | `PerlModernizer`, `ModernizationSuggestion` | Legacy pattern detection (bareword filehandles, two-arg open, missing pragmas) |
| `refactor::modernize_refactored` | `PerlModernizer`, `ModernizationSuggestion` | Structured pattern-based modernizer with explicit pattern metadata |
| `refactor::workspace_refactor` | `WorkspaceRefactor`, `RefactorError`, `FileEdit`, `RefactorResult` | Cross-file rename, extract module, move subroutine, inline variable, optimize imports |
| `refactor::workspace_rename` | `WorkspaceRename`, `WorkspaceRenameConfig`, `WorkspaceRenameResult`, `RenameStatistics` | Atomic workspace-wide symbol rename with progress reporting |

### Re-exports from lib.rs

The crate re-exports core types from `perl-parser-core` (`Node`, `NodeKind`, `Parser`, `ParseError`, etc.) and `perl-workspace-index` (`document_store`, `workspace_index`). Workspace modules (`workspace_refactor`, `workspace_rename`) are gated on `#[cfg(not(target_arch = "wasm32"))]`.

## Usage

```rust
use perl_refactoring::import_optimizer::ImportOptimizer;

let optimizer = ImportOptimizer::new();
let analysis = optimizer.analyze_content(source_code)?;
let edits = optimizer.generate_edits(source_code, &analysis);
```

```rust
use perl_refactoring::refactoring::{RefactoringEngine, RefactoringType, RefactoringScope};

let mut engine = RefactoringEngine::new();
let result = engine.refactor(
    RefactoringType::Rename { old_name: "foo".into(), new_name: "bar".into() },
    RefactoringScope::File(path),
)?;
```

## Important Notes

- All refactoring operations return edit collections rather than mutating files directly (except `WorkspaceRename::apply_edits`)
- `RefactoringEngine` keeps an operation history for rollback support
- Workspace rename is atomic: all changes succeed or all are rolled back
- `modernize` and `modernize_refactored` both expose a `PerlModernizer` struct with an `analyze(&str)` method but are separate implementations
- No `unwrap()`/`expect()` in production code per workspace lint policy
