# tree-sitter-perl-c

[![Crates.io](https://img.shields.io/crates/v/tree-sitter-perl-c.svg)](https://crates.io/crates/tree-sitter-perl-c)
[![docs.rs](https://docs.rs/tree-sitter-perl-c/badge.svg)](https://docs.rs/tree-sitter-perl-c)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Conventional tree-sitter Perl grammar binding (C FFI), maintained for compatibility and comparison against the native v3 parser.

> [!NOTE]
> `tree-sitter-perl-c` is maintained for testing, experimentation, and
> in-house accuracy comparisons. For a production traditional tree-sitter
> Perl grammar, use [`ts-parser-perl`](https://crates.io/crates/ts-parser-perl).

## Overview

This crate compiles a vendored snapshot of the upstream [tree-sitter-perl] C
grammar (`parser.c` + `scanner.c`) via the `cc` crate and exposes a
`tree_sitter::Language` so Perl source code can be parsed with the official
tree-sitter runtime.

The crate is self-contained: the C sources live under `c-src/` and are shipped
in the published package. There is no `bindgen` or `libclang` dependency — the
single symbol we need (`tree_sitter_perl`) is declared by hand in `src/lib.rs`.

## Which crate should I use?

| Need | Recommended crate |
|---|---|
| Production traditional tree-sitter Perl grammar | [`ts-parser-perl`](https://crates.io/crates/ts-parser-perl) |
| C FFI snapshot for testing, experimentation, in-house accuracy checks, or baseline benchmarking | `tree-sitter-perl-c` |
| Native v3 Rust parser facade, no C toolchain | [`tree-sitter-perl-rs`] |

`tree-sitter-perl-c` vendors a snapshot of the upstream C grammar so this
repository can compare parser behavior, run compatibility checks, and keep a
stable baseline for experiments. It is not intended to be the primary production
Cargo package for the traditional tree-sitter Perl grammar.

Choose `tree-sitter-perl-c` when you need:

- **Compatibility testing** — compare parse output against a vendored C grammar snapshot
- **Experimentation** — inspect or modify the C FFI path without depending on it as the primary production package
- **In-house accuracy checks** — compare behavior against the native v3 parser
- **Baseline benchmarking** — measure parse throughput of the C grammar vs. the native v3 parser

Choose [`ts-parser-perl`](https://crates.io/crates/ts-parser-perl) when you want
the production traditional tree-sitter Perl grammar.

Choose [`tree-sitter-perl-rs`] when you want the native v3 Rust parser facade.

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

Typed parse helpers expose a small error surface:

```rust
use tree_sitter_perl_c::{try_parse_perl_file, ParsePerlError};

match try_parse_perl_file("script.pl") {
    Ok(tree) => println!("{}", tree.root_node().kind()),
    Err(ParsePerlError::Io(err)) => eprintln!("read failure: {err}"),
    Err(ParsePerlError::LanguageSetup(_)) => eprintln!("parser setup failed"),
    Err(ParsePerlError::ParseReturnedNone) => eprintln!("parse cancelled or timed out"),
    Err(_) => eprintln!("unknown parse failure"),
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
| `try_parse_perl_bytes(code)` | Typed parse API (`ParsePerlError`) for byte slices |
| `try_parse_perl_code(code)` | Typed parse API (`ParsePerlError`) for `&str` |
| `try_parse_perl_file(path)` | Typed parse API (`ParsePerlError`) for file paths |
| `ParsePerlError` | Distinguishes setup, parse-none, and IO failures |
| `get_scanner_config()` | Returns `"c-scanner"` |

## Binaries

- `parse_c` — parse a Perl file using the byte-oriented API (non-UTF-8 safe), then:
  - exits `0` when the parse tree has no error nodes
  - exits `1` when reading/parsing fails or the tree contains syntax errors
  - supports triage flags:
    - `--root-kind` to print the root node kind
    - `--has-error` to print `true`/`false` for parse errors
    - `--sexp` to print the full tree-sitter s-expression
- `bench_parser_c` — benchmark parse throughput and emit stable `key=value` output (requires `--features test-utils`)

### `bench_parser_c` modes

`bench_parser_c` supports both one-shot and parser-reuse flows:

- `--mode cold` (default): create a fresh parser for every iteration
- `--mode warm`: reuse one parser across all iterations
- `--iterations N` / `-n N`: run N parse iterations
- `--input str|bytes` (default `str`): parse through UTF-8 string or raw-byte path
- `--cold` / `--warm`: shorthand for `--mode cold|warm`

Example:

```bash
cargo run -p tree-sitter-perl-c --bin bench_parser_c --features test-utils -- \
  examples/perl/simple.pl --mode warm --iterations 200 --input bytes
```

Output is intentionally stable for run-to-run diffing:

```text
mode=warm
input=bytes
iterations=200
total_us=12345
avg_us=61
has_error=false
```

Examples:

```bash
# Basic parse check (succeeds only when there are no parse errors)
cargo run -p tree-sitter-perl-c --bin parse_c -- fixtures/sample.pl

# Triage output for debugging parser behavior
cargo run -p tree-sitter-perl-c --bin parse_c -- --root-kind --has-error --sexp fixtures/sample.pl
```

## Snapshot provenance and refresh

Snapshot provenance and the refresh workflow are tracked in
[`UPSTREAM_SNAPSHOT.md`](UPSTREAM_SNAPSHOT.md).

That document records:

- upstream repository/reference
- generator version used for `parser.c`
- file fingerprints for auditability
- the exact local refresh + validation checklist

## Vendored files vs local wrapper code

**Vendored from upstream snapshot (`c-src/`):**

- `parser.c`, `scanner.c`
- `bsearch.h`, `tsp_unicode.h`
- `tree_sitter/{parser.h,array.h,alloc.h}`

**Maintained locally in this crate:**

- `src/lib.rs` (Rust FFI wrapper + helpers)
- `build.rs` (C compilation/link wiring)
- `tests/` and `src/bin/` (integration and sanity tooling)
- crate docs (`README.md`, `ROADMAP.md`, `UPSTREAM_SNAPSHOT.md`)

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

- [`ts-parser-perl`](https://crates.io/crates/ts-parser-perl) — recommended Cargo package for the production traditional tree-sitter Perl grammar
- [`tree-sitter-perl`](https://github.com/tree-sitter-perl/tree-sitter-perl) — upstream grammar
- [`tree-sitter-perl-rs`] — sibling crate, facade over the native v3 Rust parser
- [perl-parser] — the native v3 recursive-descent Perl parser

[`tree-sitter-perl-rs`]: https://crates.io/crates/tree-sitter-perl-rs
[perl-parser]: https://docs.rs/perl-parser

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
