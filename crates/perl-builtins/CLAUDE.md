# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-builtins` is a **Tier 1 leaf crate** providing Perl built-in function signatures and metadata for the perl-lsp ecosystem. It has no internal workspace dependencies.

**Purpose**: Supply signature information for 150+ Perl built-in functions to enable:
- Code completion with parameter hints
- Signature help (parameter documentation)
- Hover information
- Inlay hints for function arguments

## Commands

```bash
cargo build -p perl-builtins          # Build this crate
cargo test -p perl-builtins           # Run tests
cargo clippy -p perl-builtins         # Lint
cargo doc -p perl-builtins --open     # View documentation
```

## Architecture

### Two Implementations

1. **`builtin_signatures.rs`** - HashMap-based with `OnceLock` lazy init
   - `BuiltinSignature` struct with `signatures: Vec<&'static str>` and `documentation: &'static str`
   - Rich documentation for each function
   - Multiple signature variants per function
   - Used for detailed signature help

2. **`builtin_signatures_phf.rs`** - Perfect hash map (compile-time)
   - `BUILTIN_SIGS: phf::Map` - maps function name → parameter names
   - `BUILTIN_FULL_SIGS: phf::Map` - maps function name → full signature strings
   - O(1) lookup, zero runtime allocation
   - Used for fast inlay hints and completion

### Key Functions

```rust
// From builtin_signatures.rs
create_builtin_signatures() -> &'static HashMap<&'static str, BuiltinSignature>

// From builtin_signatures_phf.rs
get_param_names(function_name: &str) -> &'static [&'static str]
is_builtin(function_name: &str) -> bool
builtin_count() -> usize
```

## Function Categories

Built-ins are organized by category:
- I/O Functions (`print`, `open`, `read`, `write`, etc.)
- String Functions (`substr`, `index`, `split`, `join`, etc.)
- Array Functions (`push`, `pop`, `map`, `grep`, `sort`, etc.)
- Hash Functions (`keys`, `values`, `each`, `exists`, `delete`)
- File/Directory Functions (`stat`, `chmod`, `mkdir`, `opendir`, etc.)
- File Test Operators (`-e`, `-f`, `-d`, `-r`, `-w`, etc.)
- Process Functions (`fork`, `exec`, `system`, `kill`, etc.)
- Math Functions (`abs`, `sin`, `cos`, `sqrt`, `rand`, etc.)
- Network/Socket Functions
- User/Group Functions
- Time Functions

## Adding New Builtins

When adding a new Perl built-in function:

1. Add to `builtin_signatures.rs`:
```rust
signatures.insert(
    "function_name",
    BuiltinSignature {
        signatures: vec!["function_name ARG1, ARG2", "function_name ARG1"],
        documentation: "Brief description of what the function does",
    },
);
```

2. Add to `builtin_signatures_phf.rs`:
```rust
// In BUILTIN_SIGS
"function_name" => &["ARG1", "ARG2"],

// Optionally in BUILTIN_FULL_SIGS for detailed signatures
"function_name" => &["function_name ARG1, ARG2", "function_name ARG1"],
```

## Dependencies

- `phf` (with `macros` feature) - Perfect hash function for O(1) lookups
