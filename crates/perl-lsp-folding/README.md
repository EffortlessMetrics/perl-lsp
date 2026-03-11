# perl-lsp-folding

Standalone SRP microcrate for extracting Perl folding ranges for LSP `textDocument/foldingRange`.

## Responsibilities

- Traverse Perl parser AST nodes to produce foldable regions.
- Group adjacent import statements into `imports` folding regions.
- Detect heredoc folding regions from lexer tokens.

## License

Licensed under either of Apache License, Version 2.0 or MIT at your option.
