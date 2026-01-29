# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-inlay-hints` is a **Tier 4 LSP feature crate** providing inlay hints for type annotations and parameter names.

**Purpose**: LSP inlay hints provider for Perl — displays inline hints for parameter names, types, and other contextual information.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-inlay-hints      # Build this crate
cargo test -p perl-lsp-inlay-hints       # Run tests
cargo clippy -p perl-lsp-inlay-hints     # Lint
cargo doc -p perl-lsp-inlay-hints --open # View documentation
```

## Architecture

### Dependencies

- `perl-semantic-analyzer` - Type/symbol information
- `perl-position-tracking` - Position handling
- `perl-parser-core` - AST access
- `lsp-types` - LSP inlay hint types

### LSP Capabilities

| Method | Purpose |
|--------|---------|
| `textDocument/inlayHint` | Get hints for range |
| `inlayHint/resolve` | Resolve hint details |

### Hint Types

| Type | Example | Description |
|------|---------|-------------|
| Parameter | `func(«name:» "value")` | Parameter names |
| Type | `my $x«: Int»` | Inferred types |
| Chained | `$obj->method()«->Result»` | Return types |

### Hint Kinds

```rust
InlayHintKind::Type      // Type annotations
InlayHintKind::Parameter // Parameter names
```

## Usage

```rust
use perl_lsp_inlay_hints::InlayHintsProvider;

let provider = InlayHintsProvider::new(analyzer);

// Get hints for visible range
let hints = provider.hints(document, range)?;

for hint in hints {
    println!("{} at {:?}", hint.label, hint.position);
}
```

### Inlay Hint Structure

```rust
InlayHint {
    position: Position { line: 5, character: 10 },
    label: InlayHintLabel::String("filename:".to_string()),
    kind: Some(InlayHintKind::PARAMETER),
    padding_left: Some(false),
    padding_right: Some(true),
    // ...
}
```

### Built-in Integration

Uses `perl-builtins` for parameter names:

```perl
open(«handle:» $fh, «mode:» "<", «filename:» $file);
#     ^^^^^^^^      ^^^^^^       ^^^^^^^^^^
#     Inlay hints for built-in parameter names
```

## Important Notes

- Hints are computed lazily for visible range
- Uses PHF maps from `perl-builtins` for fast lookup
- Type hints are best-effort (Perl is dynamically typed)
- Configurable via client settings
