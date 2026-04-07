# perl-heredoc-anti-patterns

Anti-pattern detection for problematic Perl heredoc constructs.

This crate scans Perl source code for seven categories of heredoc-related
anti-patterns that make static parsing difficult or impossible, and produces
structured diagnostics describing each finding with severity, explanation,
suggested fix, and documentation references. Detected patterns include
heredocs inside `format` declarations, heredocs declared inside `BEGIN`
blocks, dynamic (variable-expanded) heredoc delimiters, source filter
modules (`Filter::Simple`, etc.), heredocs inside regex code blocks, heredocs
inside `eval` strings, and heredocs written to tied handles.

Part of the [perl-lsp](https://github.com/EffortlessMetrics/perl-lsp)
workspace. Used by `perl-lsp-diagnostics` to surface these patterns as LSP
diagnostics in editors.

## Usage

```rust
use perl_heredoc_anti_patterns::AntiPatternDetector;

let detector = AntiPatternDetector::new();
let diagnostics = detector.detect_all(perl_source_code);
let report = detector.format_report(&diagnostics);
println!("{}", report);
```

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
