# perl-tidy

`perl-tidy` is the canonical Rust-native Perl formatter library. It owns
parse-gated formatting, layout configuration, diagnostics, and edit
production without depending on an LSP server or an external executable.

```rust
use perl_tidy::{FormatConfig, NativeFormatter, PerlFormatter};

let result = NativeFormatter::new()
    .format_document("my$x=1;
", &FormatConfig::default());
assert_eq!(result.formatted, "my $x = 1;
");
```

External `Perl::Tidy` compatibility lives in `perl-tidy-perltidy`.
Protocol-coordinate conversion and live-document admission belong to the
consuming editor/LSP adapter.
