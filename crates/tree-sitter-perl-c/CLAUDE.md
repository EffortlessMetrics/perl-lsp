# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`tree-sitter-perl-c` is a **Tier 7 legacy/benchmarking crate** that wraps the
C-based tree-sitter Perl grammar via FFI.

**Purpose**: Compile the C parser (`parser.c`) and external scanner (`scanner.c`)
from `../../tree-sitter-perl/src/`, generate Rust bindings with `bindgen`, and
expose a tree-sitter `Language` for benchmarking against the native Rust parser.

**Version**: 0.9.0

This crate is **excluded** from the default workspace build because it requires
`libclang-dev` for `bindgen`.

## Commands

```bash
cargo build -p tree-sitter-perl-c                          # Build (needs C toolchain + libclang)
cargo test -p tree-sitter-perl-c                           # Run tests
cargo clippy -p tree-sitter-perl-c                         # Lint
cargo doc -p tree-sitter-perl-c --open                     # View documentation
cargo run -p tree-sitter-perl-c --bin parse_c -- input.pl  # Parse a Perl file
cargo run -p tree-sitter-perl-c --bin bench_parser_c --features test-utils -- input.pl  # Benchmark
```

## Architecture

### Build Pipeline (`build.rs`)

1. `cc` compiles `parser.c` and `scanner.c` from `../../tree-sitter-perl/src/`
2. `bindgen` generates Rust FFI bindings from `tree_sitter/parser.h`
3. The compiled static library is linked as `tree-sitter-perl-c`

### Key Types and Functions (lib.rs)

| Function | Signature | Description |
|----------|-----------|-------------|
| `language()` | `-> Language` | Returns the C tree-sitter Perl language |
| `try_create_parser()` | `-> Result<Parser, LanguageError>` | Creates a configured parser |
| `create_parser()` | `-> Parser` | Creates a parser (ignores errors) |
| `parse_perl_code()` | `(&str) -> Result<Tree, Box<dyn Error>>` | Parses a Perl string |
| `parse_perl_file()` | `(P: AsRef<Path>) -> Result<Tree, Box<dyn Error>>` | Reads and parses a file |
| `get_scanner_config()` | `-> &'static str` | Returns `"c-scanner"` |

The `unsafe extern "C"` block declares `tree_sitter_perl() -> Language` which is
the entry point into the compiled C grammar.

### Dependencies

| Dependency | Role |
|------------|------|
| `tree-sitter` 0.26 | Runtime (Language, Parser, Tree types) |
| `cc` (build) | Compiles C sources |
| `bindgen` (build) | Generates FFI bindings |
| `serde`, `proptest`, `walkdir`, `thiserror` | Carried forward; not used by lib.rs directly |

### Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `c-scanner` | yes | Enables the C scanner path |
| `test-utils` | no | Required for the `bench_parser_c` binary |

### Binaries

- **`parse_c`** -- takes a Perl file path, parses it with the C grammar, exits 0/1.
- **`bench_parser_c`** -- takes a Perl file path, prints `status=success/failure error=<bool> duration_us=<N>`.

## Usage

```rust
use tree_sitter_perl_c::{language, try_create_parser, parse_perl_code};

// Option 1: Use the high-level helper
let tree = parse_perl_code("my $x = 42;")?;
println!("root: {}", tree.root_node().to_sexp());

// Option 2: Get a configured parser for repeated use
let mut parser = try_create_parser()?;
let tree = parser.parse("print $x;", None).ok_or("parse failed")?;

// Option 3: Just get the Language for custom setup
let lang = language();
```

## Important Notes

- Requires a C compiler and `libclang-dev` headers to build
- Excluded from the default workspace (`workspace.exclude` in root `Cargo.toml`)
- Legacy crate kept for comparative benchmarking; active development uses the native v3 Rust parser
- C source files live in `../../tree-sitter-perl/src/` (the `tree-sitter-perl` directory)
