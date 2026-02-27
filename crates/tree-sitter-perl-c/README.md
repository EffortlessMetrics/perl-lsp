# tree-sitter-perl-c

Tree-sitter Perl grammar with C scanner -- legacy FFI bindings for benchmarking.

## Overview

This crate compiles the C-based tree-sitter Perl grammar (parser.c + scanner.c)
via `cc` and generates Rust bindings with `bindgen`. It is excluded from the
default workspace build because it requires `libclang-dev`.

## Public API

- `language()` -- returns the tree-sitter `Language` for Perl
- `try_create_parser()` -- creates a `tree_sitter::Parser` (returns `Result`)
- `create_parser()` -- creates a parser, ignoring language-set errors
- `parse_perl_code(code)` -- parses a `&str` into a `tree_sitter::Tree`
- `parse_perl_file(path)` -- reads and parses a file
- `get_scanner_config()` -- returns `"c-scanner"`

## Binaries

- `parse_c` -- parse a Perl file and exit
- `bench_parser_c` -- parse a Perl file and print timing (requires `test-utils` feature)

## Build Requirements

Requires a C compiler and `libclang` headers for `bindgen`:

```bash
# Debian/Ubuntu
apt install build-essential libclang-dev

# macOS
xcode-select --install
```

## License

MIT OR Apache-2.0
