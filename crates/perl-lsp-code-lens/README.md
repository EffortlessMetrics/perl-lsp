# perl-lsp-code-lens

[![Crates.io](https://img.shields.io/crates/v/perl-lsp-code-lens.svg)](https://crates.io/crates/perl-lsp-code-lens)
[![Documentation](https://docs.rs/perl-lsp-code-lens/badge.svg)](https://docs.rs/perl-lsp-code-lens)

Inline code-lens generation for Perl editors and language servers.

## When to use this crate

Use `perl-lsp-code-lens` when you want Perl-aware inline actions such as:

- run-test lenses for `.t` files and test subroutines
- reference-count lenses for packages and subroutines
- run-script lenses for executable Perl files with a shebang

This crate is primarily intended for the `perl-lsp` workspace and other Rust
language-server integrations. It is not a standalone editor plugin.

## Quick example

```rust
use perl_lsp_code_lens::{get_shebang_lens, is_test_file};

assert!(is_test_file("t/basic.t"));
assert!(get_shebang_lens("#!/usr/bin/env perl\nprint \"ok\\n\";\n").is_some());
```

## Public API

- `CodeLensProvider`: extracts lenses from a Perl AST
- `resolve_code_lens`: fills in deferred reference-count data
- `get_shebang_lens`: detects executable-script lenses from source text
- `is_test_file`: detects Perl test-file naming

## License

MIT OR Apache-2.0
