# perl-lsp

[![Crates.io](https://img.shields.io/crates/v/perl-lsp.svg)](https://crates.io/crates/perl-lsp)
[![Documentation](https://docs.rs/perl-lsp/badge.svg)](https://docs.rs/perl-lsp)

A **Language Server Protocol (LSP) server for Perl** that provides comprehensive IDE features including diagnostics, completion, hover information, go-to-definition, and more.

## Features

- **Comprehensive IDE Support**: ~65% LSP 3.17 compliance with all advertised features working
- **Real-time Diagnostics**: Syntax checking with intelligent error recovery
- **Code Completion**: Variables, built-in functions (150+), keywords
- **Code Navigation**: Go-to-definition, find references, workspace symbols
- **Code Actions**: Quick fixes, refactoring suggestions, import optimization
- **Formatting Support**: Integration with Perl::Tidy
- **Modern Editor Integration**: Works with VSCode, Neovim, Emacs, and more

## Installation

### Quick Install

```bash
cargo install perl-lsp
```

### From Source

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl-rs
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp
```

### Package Managers

#### Homebrew (macOS)

```bash
brew tap tree-sitter-perl/tap
brew install perl-lsp
```

## Usage

### Command Line

```bash
# Start LSP server (for editor integration)
perl-lsp --stdio

# Health check
perl-lsp --health

# Show version
perl-lsp --version

# Enable logging
perl-lsp --stdio --log
```

### Editor Integration

The LSP server works with any LSP-compatible editor:

#### VSCode

Install the [Perl Language Server extension](https://marketplace.visualstudio.com/items?itemName=perl-lsp.perl-language-server).

#### Neovim

```lua
require'lspconfig'.perl_lsp.setup{}
```

#### Emacs (lsp-mode)

```elisp
(add-to-list 'lsp-language-id-configuration '(perl-mode . "perl"))
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "perl-lsp")
                  :major-modes '(perl-mode)
                  :server-id 'perl-lsp))
```

## Architecture

This LSP server is built on top of the [perl-parser](https://crates.io/crates/perl-parser) crate, which provides:

- **Native Recursive Descent Parser**: ~100% Perl 5 syntax coverage
- **High Performance**: 4-19x faster than legacy implementations  
- **Tree-sitter Compatible**: Standard AST format
- **Comprehensive Edge Case Handling**: All Perl parsing complexities supported

## Supported LSP Features

### ✅ Fully Working (Production Ready)

- **textDocument/publishDiagnostics**: Real-time syntax checking
- **textDocument/completion**: Code completion with context awareness
- **textDocument/hover**: Documentation and type information
- **textDocument/definition**: Go-to-definition support
- **textDocument/references**: Find all references
- **textDocument/documentSymbol**: File outline and navigation
- **textDocument/formatting**: Code formatting via Perl::Tidy
- **textDocument/foldingRange**: Code folding support
- **workspace/symbol**: Cross-file symbol search
- **textDocument/rename**: Symbol renaming
- **textDocument/codeAction**: Quick fixes, refactoring, and import optimization ("Organize Imports")
- **textDocument/semanticTokens**: Enhanced syntax highlighting

### Recently Graduated

- **textDocument/codeLens**: Reference counting, well-tested
- **callHierarchy/**: Call hierarchy support

See the [LSP Capability Documentation](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/blob/master/docs/LSP_ACTUAL_STATUS.md) for complete feature status.

## Performance

- **Parsing Speed**: 1-150 μs for typical files
- **Memory Usage**: Efficient Arc<str> storage
- **Response Time**: <50ms for all LSP operations
- **Scalability**: Handles large codebases efficiently

## Contributing

See [CONTRIBUTING.md](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/blob/master/CONTRIBUTING.md) for development setup and contribution guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Related Crates

- [perl-parser](https://crates.io/crates/perl-parser): Core parsing library
- [perl-lexer](https://crates.io/crates/perl-lexer): Context-aware tokenizer  
- [perl-corpus](https://crates.io/crates/perl-corpus): Test corpus for validation