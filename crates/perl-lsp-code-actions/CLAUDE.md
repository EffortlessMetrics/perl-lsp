# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-code-actions` is a **Tier 4 LSP feature crate** providing quick fixes and refactoring actions.

**Purpose**: LSP code actions provider for Perl — provides quick fixes, refactorings, and source actions.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-code-actions     # Build this crate
cargo test -p perl-lsp-code-actions      # Run tests
cargo clippy -p perl-lsp-code-actions    # Lint
cargo doc -p perl-lsp-code-actions --open # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST access
- `perl-position-tracking` - Position handling
- `perl-refactoring` - Refactoring operations
- `perl-workspace-index` - Cross-file operations
- `perl-semantic-analyzer` - Symbol information
- `perl-lsp-rename` - Rename actions
- `perl-lsp-diagnostics` - Diagnostic-associated fixes

### Code Action Kinds

| Kind | Examples |
|------|----------|
| `quickfix` | Fix diagnostic issues |
| `refactor` | Rename, extract, inline |
| `refactor.extract` | Extract variable/subroutine |
| `refactor.inline` | Inline variable/subroutine |
| `source` | Organize imports, format |
| `source.organizeImports` | Sort/clean use statements |

### Available Actions

| Action | Description |
|--------|-------------|
| **Add missing `my`** | Fix undeclared variable |
| **Add `use strict`** | Add strict pragma |
| **Add `use warnings`** | Add warnings pragma |
| **Extract variable** | Extract expression to variable |
| **Extract subroutine** | Extract code to subroutine |
| **Rename symbol** | Rename variable/subroutine |
| **Convert to modern syntax** | Modernize code patterns |

## Usage

```rust
use perl_lsp_code_actions::CodeActionsProvider;

let provider = CodeActionsProvider::new(analyzer, refactoring);

// Get code actions for range
let actions = provider.code_actions(document, range, context)?;

for action in actions {
    match action {
        CodeActionOrCommand::CodeAction(action) => {
            println!("{}: {:?}", action.title, action.kind);
        },
        _ => {}
    }
}
```

### Context-Aware Actions

```rust
// Actions depend on:
// - Cursor position
// - Selection range
// - Active diagnostics
// - Surrounding code context

let context = CodeActionContext {
    diagnostics: vec![...],  // Current diagnostics at range
    only: Some(vec![CodeActionKind::QUICKFIX]),  // Filter
    trigger_kind: Some(CodeActionTriggerKind::AUTOMATIC),
};
```

## Important Notes

- Quick fixes are associated with specific diagnostics
- Refactorings use `perl-refactoring` crate
- Actions return `WorkspaceEdit` for multi-file changes
- Lazy resolution supported for performance
