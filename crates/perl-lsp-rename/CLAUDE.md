# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-rename` is a **Tier 4 LSP feature crate** providing rename refactoring support.

**Purpose**: LSP rename provider for Perl — enables safe renaming of variables, subroutines, and other symbols.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-rename           # Build this crate
cargo test -p perl-lsp-rename            # Run tests
cargo clippy -p perl-lsp-rename          # Lint
cargo doc -p perl-lsp-rename --open      # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST access
- `perl-semantic-analyzer` - Symbol resolution

### LSP Capabilities

| Method | Purpose |
|--------|---------|
| `textDocument/rename` | Perform rename |
| `textDocument/prepareRename` | Validate rename is possible |

### Renameable Symbols

| Symbol Type | Scope |
|-------------|-------|
| Lexical variables (`my`) | Current scope |
| Package variables (`our`) | Package scope |
| Subroutines | Package or workspace |
| Labels | Current subroutine |
| Constants | Package scope |

## Usage

```rust
use perl_lsp_rename::RenameProvider;

let provider = RenameProvider::new(analyzer);

// Check if rename is valid
let prepare = provider.prepare_rename(document, position)?;
match prepare {
    Some(range) => println!("Can rename: {:?}", range),
    None => println!("Cannot rename at this position"),
}

// Perform rename
let edits = provider.rename(document, position, "new_name")?;
```

### Prepare Rename Response

```rust
PrepareRenameResponse::RangeWithPlaceholder {
    range: Range { start, end },
    placeholder: "$old_name".to_string(),
}
```

### Workspace Edit

```rust
WorkspaceEdit {
    changes: Some(HashMap::from([
        (uri1, vec![TextEdit { range, new_text: "$new_name" }]),
        (uri2, vec![TextEdit { range, new_text: "$new_name" }]),
    ])),
    // ...
}
```

## Rename Rules

| Rule | Example |
|------|---------|
| Variable keeps sigil | `$foo` → `$bar` (not `$foo` → `bar`) |
| Subroutine no sigil | `foo` → `bar` |
| Cross-file for `our`/subs | Updates all usages |
| Scope-limited for `my` | Only current scope |

## Important Notes

- Prepare rename validates before allowing rename
- Cross-file rename requires workspace indexing
- Preserves sigils for variables
- Validates new name is valid Perl identifier
