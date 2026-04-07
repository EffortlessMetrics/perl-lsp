# tree-sitter-perl-c

[![crates.io](https://img.shields.io/crates/v/tree-sitter-perl-c.svg)](https://crates.io/crates/tree-sitter-perl-c)
[![docs.rs](https://docs.rs/tree-sitter-perl-c/badge.svg)](https://docs.rs/tree-sitter-perl-c)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.92-blue)](https://www.rust-lang.org/)

Tree-sitter Perl grammar with C scanner — the legacy C-FFI implementation.

## Overview

This crate compiles a vendored snapshot of the C-based tree-sitter Perl
grammar (`parser.c` + `scanner.c`) via the `cc` crate and exposes a
`tree_sitter::Language` so Perl source code can be parsed with the
official tree-sitter runtime.

The crate is self-contained: the C sources live under `c-src/` and are
shipped in the published package. There is no `bindgen` or `libclang`
dependency — the single symbol we need (`tree_sitter_perl`) is declared
by hand in `src/lib.rs`.

## Public API

- `language()` — returns the tree-sitter `Language` for Perl
- `try_create_parser()` — creates a `tree_sitter::Parser` (returns `Result`)
- `create_parser()` — creates a parser, ignoring language-set errors
- `parse_perl_code(code)` — parses a `&str` into a `tree_sitter::Tree`
- `parse_perl_file(path)` — reads and parses a file
- `get_scanner_config()` — returns `"c-scanner"`

## Binaries

- `parse_c` — parse a Perl file and exit
- `bench_parser_c` — parse a Perl file and print timing (requires `test-utils` feature)

## Build Requirements

Requires a C compiler. No `libclang` or other FFI-generator toolchain is
needed.

```bash
# Debian/Ubuntu
apt install build-essential

# macOS
xcode-select --install
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
