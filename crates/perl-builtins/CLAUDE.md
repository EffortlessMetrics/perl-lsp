# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

- **Crate**: `perl-builtins` v0.9.1
- **Tier**: Tier 1 leaf crate (no internal workspace dependencies)
- **Purpose**: Supply signature information for 200+ Perl built-in functions (including 27 file test operators) to enable code completion, signature help, hover info, and inlay hints in the LSP.

## Commands

```bash
cargo build -p perl-builtins          # Build this crate
cargo test -p perl-builtins           # Run tests
cargo clippy -p perl-builtins         # Lint
cargo doc -p perl-builtins --open     # View documentation
```

## Architecture

### Dependencies

- `phf` (with `macros` feature) -- compile-time perfect hash maps

### Modules

#### `builtin_signatures` (HashMap-based, lazy init)

- **`BuiltinSignature`** struct -- `signatures: Vec<&'static str>`, `documentation: &'static str`
- **`create_builtin_signatures()`** -- returns `&'static HashMap<&'static str, BuiltinSignature>` via `OnceLock`
- Rich docs and multiple signature variants per function
- Used for detailed signature help and hover

#### `builtin_signatures_phf` (compile-time PHF maps)

- **`BUILTIN_SIGS: phf::Map<&str, &[&str]>`** -- function name to parameter name slices
- **`BUILTIN_FULL_SIGS: phf::Map<&str, &[&str]>`** -- function name to full signature strings (subset)
- **`get_param_names(name) -> &'static [&'static str]`** -- parameter names, empty slice if unknown
- **`is_builtin(name) -> bool`** -- checks if name is a known builtin
- **`builtin_count() -> usize`** -- total number of entries in `BUILTIN_SIGS`
- O(1) lookup, zero runtime allocation
- Used for fast inlay hints and completion

### Function categories

I/O, strings, arrays, hashes, file/directory, file test operators (`-e` through `-C`), processes, math, sockets/network, IPC (`msg*`, `sem*`, `shm*`), user/group, time, modules (`use`, `require`, `no`), control flow, tied variables, pack/unpack, regex, format, and miscellaneous.

## Usage

```rust
use perl_builtins::builtin_signatures::create_builtin_signatures;
use perl_builtins::builtin_signatures_phf::{is_builtin, get_param_names, builtin_count};

let sigs = create_builtin_signatures();
let open_sig = &sigs["open"];
// open_sig.signatures == ["open FILEHANDLE, MODE, FILENAME", ...]
// open_sig.documentation == "Opens a file"

assert!(is_builtin("print"));
assert_eq!(get_param_names("substr"), &["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"]);
```

## Important Notes

- When adding a new builtin, add it to **both** `builtin_signatures.rs` and `builtin_signatures_phf.rs` to keep the two stores in sync.
- `BUILTIN_FULL_SIGS` in the PHF module is intentionally a subset; only functions needing detailed multi-variant signatures are included.
- The `get_param_names` helper returns an empty slice (not an error) for unknown functions.
