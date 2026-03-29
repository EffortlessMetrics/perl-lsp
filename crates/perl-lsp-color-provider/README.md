# perl-lsp-color-provider

[![Crates.io](https://img.shields.io/crates/v/perl-lsp-color-provider.svg)](https://crates.io/crates/perl-lsp-color-provider)
[![Documentation](https://docs.rs/perl-lsp-color-provider/badge.svg)](https://docs.rs/perl-lsp-color-provider)

Document-color detection and presentation helpers for Perl source code.

## When to use this crate

Use `perl-lsp-color-provider` when you want `textDocument/documentColor` or
`textDocument/colorPresentation` support for Perl code. It recognizes:

- hex colors such as `#ff00aa`
- ANSI escape sequences
- named CSS colors in quoted strings
- `Term::ANSIColor` calls

## Quick example

```rust
use perl_lsp_color_provider::detect_colors;

let colors = detect_colors(r#"my $accent = "#ff00aa";"#);
assert_eq!(colors.len(), 1);
```

## Public API

- `detect_colors`: scans Perl source for color literals
- `color_to_presentations`: returns editor-facing replacement formats
- `ColorInformation` and `Color`: simple range and RGBA value types

## License

MIT OR Apache-2.0
