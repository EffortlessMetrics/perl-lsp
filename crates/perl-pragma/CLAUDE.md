# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-pragma` is a **Tier 1 leaf crate** providing pragma (use/no) handling and validation.

**Purpose**: Pragma handling and validation — recognizes and validates Perl pragmas like `use strict`, `use warnings`, `use feature`.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-pragma           # Build this crate
cargo test -p perl-pragma            # Run tests
cargo clippy -p perl-pragma          # Lint
cargo doc -p perl-pragma --open      # View documentation
```

## Architecture

### Dependencies

- `perl-ast` - AST node types

### Common Pragmas

| Pragma | Purpose |
|--------|---------|
| `strict` | Enable strict variable checking |
| `warnings` | Enable runtime warnings |
| `feature` | Enable language features |
| `utf8` | Enable UTF-8 source |
| `vars` | Pre-declare global variables |
| `constant` | Define constants |
| `lib` | Modify @INC |
| `parent` | Establish ISA relationship |
| `base` | Legacy parent pragma |

### Feature Bundles

```perl
use feature ':5.10';    # Enable 5.10 features
use feature ':5.36';    # Enable 5.36 features
use v5.36;              # Same as above
```

### Key Functions

```rust
// Check if a module is a pragma
is_pragma(module_name: &str) -> bool

// Get pragma effects
pragma_effects(name: &str, args: &[...]) -> PragmaEffects
```

## Usage

```rust
use perl_pragma::{is_pragma, PragmaKind};

if is_pragma("strict") {
    // Handle pragma import
}

match PragmaKind::from_name("warnings") {
    Some(PragmaKind::Warnings) => { /* ... */ },
    _ => { /* regular module */ },
}
```

## Important Notes

- Pragmas affect lexical scope
- Some pragmas have arguments that modify their behavior
- The `no` keyword disables pragmas
