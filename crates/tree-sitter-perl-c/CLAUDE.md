# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`tree-sitter-perl-c` is the **conventional tree-sitter grammar/binding crate**
for Perl, maintained for compatibility and comparison against the native v3
parser. It wraps the C-based tree-sitter Perl grammar via a hand-written FFI
declaration.

**Purpose**: Compile the vendored C parser (`parser.c`) and external
scanner (`scanner.c`) from `c-src/` via the `cc` crate and expose a
tree-sitter `Language` for compatibility testing and benchmarking against
the native Rust parser.

**Version**: tracks the workspace (currently `0.12.3`).

This crate is a workspace member and is published to crates.io. It
requires only a C compiler at build time — no `libclang` / `bindgen`
toolchain is involved.

## Commands

```bash
cargo build -p tree-sitter-perl-c                          # Build (needs C toolchain)
cargo test -p tree-sitter-perl-c                           # Run tests
cargo clippy -p tree-sitter-perl-c                         # Lint
cargo doc -p tree-sitter-perl-c --open                     # View documentation
cargo run -p tree-sitter-perl-c --bin parse_c -- input.pl  # Parse a Perl file
cargo run -p tree-sitter-perl-c --bin bench_parser_c --features test-utils -- input.pl  # Benchmark
```

## Architecture

### Build Pipeline (`build.rs`)

1. `cc` compiles `c-src/parser.c` and `c-src/scanner.c`
2. The compiled static library is linked as `tree-sitter-perl-c`
3. The single FFI symbol `tree_sitter_perl()` is declared by hand in
   `src/lib.rs` — no `bindgen` is involved

### Vendored C Sources

The `c-src/` directory contains a snapshot of the upstream tree-sitter
Perl grammar:

- `parser.c` — the tree-sitter-generated LR parser
- `scanner.c` — the external scanner
- `tsp_unicode.h`, `bsearch.h` — scanner helpers
- `tree_sitter/parser.h`, `tree_sitter/array.h`, `tree_sitter/alloc.h` —
  tree-sitter runtime headers required by `parser.c` and `scanner.c`

The `c-src/` directory IS the canonical source of truth for these files
inside this repository. This crate carries its own copy so the published
package is self-contained.

### Key Types and Functions (lib.rs)

| Function | Signature | Description |
|----------|-----------|-------------|
| `language()` | `-> Language` | Returns the C tree-sitter Perl language |
| `try_create_parser()` | `-> Result<Parser, LanguageError>` | Creates a configured parser |
| `create_parser()` | `-> Parser` | Creates a parser (ignores errors) |
| `parse_perl_code()` | `(&str) -> Result<Tree, Box<dyn Error>>` | Parses a Perl string |
| `parse_perl_file()` | `(P: AsRef<Path>) -> Result<Tree, Box<dyn Error>>` | Reads and parses a file |
| `get_scanner_config()` | `-> &'static str` | Returns `"c-scanner"` |

The `unsafe extern "C"` block declares `tree_sitter_perl() -> Language`
which is the entry point into the compiled C grammar.

### Dependencies

| Dependency | Role |
|------------|------|
| `tree-sitter` 0.26 | Runtime (`Language`, `Parser`, `Tree` types) |
| `cc` (build) | Compiles the vendored C sources |

### Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `c-scanner` | yes | Enables the C scanner path |
| `test-utils` | no | Required for the `bench_parser_c` binary |

### Binaries

- **`parse_c`** — takes a Perl file path, parses it with the C grammar, exits 0/1.
- **`bench_parser_c`** — takes a Perl file path, prints `status=success/failure error=<bool> duration_us=<N>`.

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

- Requires a C compiler only — no `libclang` / `bindgen` toolchain needed.
- Participates in the default workspace build and is on the publish allowlist.
- Conventional C-FFI reference implementation maintained for compatibility
  and comparison; active development for the native parser uses the v3
  Rust parser in `crates/perl-parser/`.
- The C sources under `c-src/` are a vendored snapshot. The old harness crate
  (`tree-sitter-perl-rs`) has been archived to `archive/crates/tree-sitter-perl-rs/`;
  `c-src/` is now the sole source of truth for upstream C grammar snapshots.
