# perl-builtins

Builtin function signatures and metadata for Perl parser and LSP tooling.

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace.

## Overview

Provides two complementary lookup mechanisms for 200+ Perl built-in functions (including file test operators):

- **`builtin_signatures`** -- `HashMap`-based store with `OnceLock` lazy init. Each entry carries multiple signature variants and a documentation string (`BuiltinSignature`). Used for signature help and hover.
- **`builtin_signatures_phf`** -- Compile-time `phf::Map` tables (`BUILTIN_SIGS`, `BUILTIN_FULL_SIGS`) for O(1) lookups with zero runtime allocation. Exposes `get_param_names`, `is_builtin`, and `builtin_count` helpers. Used for inlay hints and completion.

## Categories covered

I/O, strings, arrays, hashes, file/directory ops, file test operators (`-e`, `-f`, ...), processes, math, sockets/network, IPC, user/group, time, modules, control flow, tied variables, and more.

## Usage

```rust
use perl_builtins::builtin_signatures_phf::{is_builtin, get_param_names};

assert!(is_builtin("print"));
assert_eq!(get_param_names("open"), &["FILEHANDLE", "MODE", "FILENAME"]);
```

## License

MIT OR Apache-2.0
