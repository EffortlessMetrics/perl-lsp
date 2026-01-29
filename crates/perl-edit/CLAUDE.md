# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-edit` is a **Tier 1 leaf crate** providing edit and text transformation utilities.

**Purpose**: Edit/text transformation utilities — represents and applies text edits for refactoring and code actions.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-edit             # Build this crate
cargo test -p perl-edit              # Run tests
cargo clippy -p perl-edit            # Lint
cargo doc -p perl-edit --open        # View documentation
```

## Architecture

### Dependencies

- `perl-position-tracking` - Position/span types

### Key Types

| Type | Purpose |
|------|---------|
| `TextEdit` | Single text replacement |
| `EditSet` | Collection of non-overlapping edits |
| `EditBuilder` | Fluent API for building edits |

### TextEdit Structure

```rust
pub struct TextEdit {
    /// Byte range to replace
    pub range: Range<usize>,
    /// New text to insert
    pub new_text: String,
}
```

### Edit Operations

| Operation | Description |
|-----------|-------------|
| Insert | Range with zero length |
| Delete | Empty new_text |
| Replace | Non-empty range and new_text |

## Usage

```rust
use perl_edit::{TextEdit, EditSet};

// Single edit
let edit = TextEdit {
    range: 10..15,
    new_text: "replacement".to_string(),
};

// Multiple edits (applied in reverse order)
let mut edits = EditSet::new();
edits.insert(edit1);
edits.insert(edit2);

// Apply to source
let result = edits.apply(source);
```

### Edit Builder

```rust
use perl_edit::EditBuilder;

let edits = EditBuilder::new()
    .replace(10..15, "new_text")
    .insert(20, "inserted")
    .delete(30..35)
    .build();
```

## Important Notes

- Edits must not overlap
- Edits are applied in reverse byte order (to preserve positions)
- LSP edit conversion is in `perl-lsp-*` crates
