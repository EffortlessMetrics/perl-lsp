# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

- **Crate**: `perl-lsp-rename` (v0.9.1)
- **Tier**: Listed under Tier 2 in workspace `Cargo.toml` (single-level workspace dependency)
- **Purpose**: LSP rename provider for Perl -- validates rename requests, resolves symbol references, and produces text edits for single-file renames.

## Commands

```bash
cargo build -p perl-lsp-rename           # Build
cargo test -p perl-lsp-rename            # Run tests
cargo clippy -p perl-lsp-rename          # Lint
cargo doc -p perl-lsp-rename --open      # View docs
```

## Architecture

### Dependencies

| Dependency | Role |
|-----------|------|
| `perl-parser-core` | AST `Node`, `SourceLocation`, `Parser` types |
| `perl-semantic-analyzer` | `SymbolExtractor`, `SymbolTable`, `SymbolKind` |

Dev-only: `perl-tdd-support` (test helpers `must`, `must_some`).

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `rename::types` | `src/rename/types.rs` | `TextEdit`, `RenameResult`, `RenameOptions` structs |
| `rename::validate` | `src/rename/validate.rs` | `can_rename_symbol()` (blocks specials/builtins), `validate_name()` (identifier rules, keyword check, conflict detection) |
| `rename::resolve` | `src/rename/resolve.rs` | `find_symbol_at_position()`, `get_symbol_range_at_position()`, `extract_symbol_from_source()` |
| `rename::apply` | `src/rename/apply.rs` | `adjust_location_for_sigil()`, `find_occurrences_in_text()`, `apply_rename_edits()`, comment/string detection helpers |
| `rename` (mod) | `src/rename/mod.rs` | `RenameProvider` struct with `prepare_rename()` and `rename()` methods; re-exports |

### Key Types

- **`RenameProvider`** -- constructed with `(&Node, String)`, holds a `SymbolTable` and source text. Two public methods: `prepare_rename(position)` and `rename(position, new_name, options)`.
- **`RenameOptions`** -- flags for `rename_in_comments`, `rename_in_strings`, `validate_new_name` (default `true`).
- **`RenameResult`** -- `edits: Vec<TextEdit>`, `is_valid: bool`, `error: Option<String>`.

### LSP Methods Supported

- `textDocument/prepareRename` -- via `RenameProvider::prepare_rename()`
- `textDocument/rename` -- via `RenameProvider::rename()`

## Usage

```rust
use perl_lsp_rename::{RenameProvider, RenameOptions};
use perl_parser_core::Parser;

let code = "my $count = 0; $count += 1;";
let ast = Parser::new(code).parse()?;
let provider = RenameProvider::new(&ast, code.to_string());

// Check if rename is valid at byte position
if let Some((range, name)) = provider.prepare_rename(4) {
    println!("Can rename '{}' at {:?}", name, range);
}

// Perform rename
let result = provider.rename(4, "total", &RenameOptions::default());
if result.is_valid {
    for edit in &result.edits {
        println!("Replace {:?} with '{}'", edit.location, edit.new_text);
    }
}
```

## Important Notes

- Special variables (`$_`, `$!`, `$1`, etc.) and built-in functions (`print`, `push`, `die`, etc.) are blocked from renaming.
- Sigils are preserved: renaming `$foo` to `bar` produces edits that replace only the identifier portion, keeping `$`.
- Edits are sorted by position and deduplicated before being returned.
- The `apply_rename_edits()` helper applies edits in reverse order to preserve byte offsets.
- Name validation rejects: empty strings, leading digits, non-alphanumeric/underscore characters, Perl keywords, and (for subroutines) existing symbol names.
