# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-incremental-parsing` is a **Tier 3 optimization crate** providing incremental parsing support for efficient document updates.

**Purpose**: Incremental parsing support for Perl — enables efficient re-parsing of only changed portions of documents.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-incremental-parsing        # Build this crate
cargo test -p perl-incremental-parsing         # Run tests
cargo clippy -p perl-incremental-parsing       # Lint
cargo doc -p perl-incremental-parsing --open   # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - Full parsing capability
- `perl-edit` - Edit operations
- `perl-lexer` - Tokenization

### External Dependencies

- `anyhow` - Error handling
- `lsp-types` - LSP change events
- `ropey` - Efficient text rope
- `serde_json` - Serialization
- `tracing` - Logging

### Key Types

| Type | Purpose |
|------|---------|
| `IncrementalParser` | Manages incremental parse state |
| `ParseCache` | Cached parse results |
| `ChangeSet` | Accumulated document changes |

### Incremental Strategy

1. **Change Detection** — Identify modified regions
2. **Invalidation** — Mark affected AST nodes as stale
3. **Re-parse** — Parse only invalidated regions
4. **Merge** — Combine new nodes with cached nodes

## Usage

```rust
use perl_incremental_parsing::IncrementalParser;

let mut parser = IncrementalParser::new(source);
let ast = parser.parse()?;

// Apply incremental change
parser.apply_change(TextDocumentContentChangeEvent {
    range: Some(Range { start, end }),
    text: "new text".to_string(),
    ..
});

// Re-parse incrementally
let updated_ast = parser.parse()?;  // Only re-parses affected regions
```

### LSP Integration

```rust
// Handle textDocument/didChange notification
fn on_change(params: DidChangeTextDocumentParams) {
    for change in params.content_changes {
        parser.apply_change(change);
    }
    let ast = parser.parse()?;
    // Update diagnostics, etc.
}
```

## Performance Characteristics

| Change Type | Re-parse Scope |
|-------------|----------------|
| Single character | Statement only |
| Line edit | Block scope |
| Multi-line | Affected scopes |
| Structural | Full re-parse |

## Important Notes

- Incremental parsing is optional (can always fall back to full parse)
- Trade-off between complexity and performance gain
- Most effective for large files with small edits
- Enabled via `incremental` feature in `perl-lsp`
