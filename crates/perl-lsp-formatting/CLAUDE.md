# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-formatting` is a **Tier 4 LSP feature crate** providing code formatting via perltidy integration.

**Purpose**: LSP formatting provider with perltidy integration — formats Perl code according to style rules.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-formatting       # Build this crate
cargo test -p perl-lsp-formatting        # Run tests
cargo clippy -p perl-lsp-formatting      # Lint
cargo doc -p perl-lsp-formatting --open  # View documentation
```

## Architecture

### Dependencies

- `perl-lsp-tooling` - Tool execution
- `perl-parser-core` - AST access
- `lsp-types` - LSP formatting types

### LSP Capabilities

| Method | Purpose |
|--------|---------|
| `textDocument/formatting` | Format entire document |
| `textDocument/rangeFormatting` | Format selection |
| `textDocument/onTypeFormatting` | Format on type |

### Formatting Backend

The primary backend is **perltidy**:

```bash
# perltidy is invoked with:
perltidy --standard-output --standard-error-output < input.pl
```

### Configuration

Perltidy configuration is read from:

1. `.perltidyrc` in project root
2. `~/.perltidyrc` in home directory
3. Default perltidy settings

## Usage

```rust
use perl_lsp_formatting::FormattingProvider;

let provider = FormattingProvider::new(tooling);

// Format document
let edits = provider.format(document, options)?;

// Format range
let edits = provider.format_range(document, range, options)?;
```

### Formatting Options

```rust
FormattingOptions {
    tab_size: 4,
    insert_spaces: true,
    // Additional perltidy options can be configured
}
```

### Result

Returns `Vec<TextEdit>` representing changes:

```rust
vec![TextEdit {
    range: Range {
        start: Position { line: 0, character: 0 },
        end: Position { line: 100, character: 0 },
    },
    new_text: formatted_content,
}]
```

## Important Notes

- Requires perltidy to be installed and in PATH
- Respects project `.perltidyrc` configuration
- Falls back gracefully if perltidy unavailable
- On-type formatting triggered by `;`, `}`, etc.
