# tree-sitter-perl-c

[![Crates.io](https://img.shields.io/crates/v/tree-sitter-perl-c.svg)](https://crates.io/crates/tree-sitter-perl-c)
[![docs.rs](https://docs.rs/tree-sitter-perl-c/badge.svg)](https://docs.rs/tree-sitter-perl-c)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Conventional tree-sitter Perl grammar binding (C FFI), maintained for compatibility and comparison against the native v3 parser.

## Overview

This crate compiles a vendored snapshot of the upstream [tree-sitter-perl] C
grammar (`parser.c` + `scanner.c`) via the `cc` crate and exposes a
`tree_sitter::Language` so Perl source code can be parsed with the official
tree-sitter runtime.

The crate is self-contained: the C sources live under `c-src/` and are shipped
in the published package. There is no `bindgen` or `libclang` dependency — the
single symbol we need (`tree_sitter_perl`) is declared by hand in `src/lib.rs`.

## This crate vs. `tree-sitter-perl-rs`

| | `tree-sitter-perl-c` (this crate) | `tree-sitter-perl-rs` |
|---|---|---|
| **Backend** | Upstream C grammar (FFI) | Facade over native v3 Rust parser |
| **Best for** | Compatibility testing, non-Rust tooling, baseline benchmarking | New Rust projects, embedded use, no C toolchain |
| **Build dep** | C compiler required | Pure Rust |
| **Grammar source** | Upstream tree-sitter-perl | Native v3 recursive-descent |

Choose `tree-sitter-perl-c` when you need:

- **Compatibility testing** — compare parse output against the C reference grammar
- **Non-Rust tree-sitter tooling** — the C grammar snapshot can be consumed by
  language bindings in Python, Node.js, etc.
- **Baseline benchmarking** — measure parse throughput of the C grammar vs. the
  native v3 parser

Choose [`tree-sitter-perl-rs`] for new Rust projects.

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
tree-sitter-perl-c = "0.12"
```

Parse Perl source:

```rust
use tree_sitter_perl_c::parse_perl_code;

fn main() {
    let tree = parse_perl_code("my $x = 42;").unwrap();
    println!("{}", tree.root_node().to_sexp());
    // Prints the tree-sitter s-expression for the parsed Perl
}
```

For repeated parsing, reuse a configured parser with the helper APIs:

```rust
use tree_sitter_perl_c::{parse_perl_code_with_parser, try_create_parser};

let mut parser = try_create_parser().unwrap();

for snippet in &["my $x = 1;", "print $x;"] {
    let tree = parse_perl_code_with_parser(&mut parser, snippet).unwrap();
    assert!(!tree.root_node().has_error());
}
```

## Public API

| Function | Description |
|----------|-------------|
| `language()` | Returns the tree-sitter `Language` for Perl |
| `try_create_parser()` | Creates a `tree_sitter::Parser` (returns `Result`) |
| `create_parser()` | Creates a parser, silently ignoring language-set errors |
| `parse_perl_bytes(code)` | Parses raw bytes (including non-UTF-8 Perl source) |
| `parse_perl_bytes_with_parser(parser, code)` | Parses raw bytes with a caller-provided configured parser |
| `parse_perl_code(code)` | Parses a `&str` into a `tree_sitter::Tree` |
| `parse_perl_code_with_parser(parser, code)` | Parses a `&str` with a caller-provided configured parser |
| `parse_perl_file(path)` | Reads and parses a file (non-UTF-8 safe) |
| `get_scanner_config()` | Returns `"c-scanner"` |

## Binaries

- `parse_c` — parse a Perl file using the byte-oriented API (non-UTF-8 safe), then:
  - exits `0` when the parse tree has no error nodes
  - exits `1` when reading/parsing fails or the tree contains syntax errors
  - supports triage flags:
    - `--root-kind` to print the root node kind
    - `--has-error` to print `true`/`false` for parse errors
    - `--sexp` to print the full tree-sitter s-expression
- `bench_parser_c` — parse a Perl file and print timing (requires `--features test-utils`)

Examples:

```bash
# Basic parse check (succeeds only when there are no parse errors)
cargo run -p tree-sitter-perl-c --bin parse_c -- fixtures/sample.pl

# Triage output for debugging parser behavior
cargo run -p tree-sitter-perl-c --bin parse_c -- --root-kind --has-error --sexp fixtures/sample.pl
```

## Build Requirements

Only a C compiler is required. No `libclang` or other FFI-generator toolchain
is needed.

```bash
# Debian/Ubuntu
apt install build-essential

# macOS
xcode-select --install

# Windows: MSVC (via Visual Studio) or MinGW-w64 both work
```

## Links

- [tree-sitter-perl upstream grammar][tree-sitter-perl]
- [`tree-sitter-perl-rs`] — sibling crate, facade over the native v3 Rust parser
- [perl-parser] — the native v3 recursive-descent Perl parser

[tree-sitter-perl]: https://github.com/tree-sitter-perl/tree-sitter-perl
[`tree-sitter-perl-rs`]: https://crates.io/crates/tree-sitter-perl-rs
[perl-parser]: https://docs.rs/perl-parser

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
