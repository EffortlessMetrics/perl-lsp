# Perl Language Server

Lightning-fast Perl language support with 26+ IDE features powered by tree-sitter-perl.

## ✨ Features

### 🎯 Core Intelligence
- **Go to Definition** - Jump to any symbol declaration
- **Find References** - Find all usages across your project
- **Hover Documentation** - Instant docs for functions and variables
- **Auto-completion** - Smart suggestions for variables, functions, modules
- **Signature Help** - Real-time parameter hints while typing
- **Symbol Navigation** - Outline view and breadcrumbs

### 🔧 Refactoring & Code Actions
- **Rename** - Safe renaming across files
- **Extract Variable** - Pull out expressions into variables
- **Extract Subroutine** - Create functions from code blocks
- **Convert Loops** - Transform between for/while/foreach
- **Add Error Checking** - Insert error handling automatically
- **Organize Imports** - Sort and clean use statements

### 📊 Advanced Features
- **Semantic Highlighting** - Context-aware syntax coloring
- **Type Hierarchy** - Navigate inheritance with @ISA
- **Call Hierarchy** - Trace function calls up and down
- **CodeLens** - Inline reference counts
- **Inlay Hints** - Type annotations in editor
- **Document Highlights** - Highlight symbol occurrences

### 🐛 Diagnostics & Quality
- **Real-time Errors** - Syntax and semantic error detection
- **Undefined Variables** - Catch typos under `use strict`
- **Unused Variables** - Find dead code
- **Missing Pragmas** - Suggest strict/warnings
- **Code Folding** - Collapse code blocks
- **Format on Type** - Auto-formatting as you code

## 🚀 Performance

- **1-150μs** typical parse times
- **<50ms response time** for all operations
- **100% Perl 5 coverage** including edge cases
- **Sub-millisecond** position conversions (v0.8.0)

## 📦 Installation

The extension automatically downloads the correct language server for your platform:
- Windows (x64, ARM64)
- macOS (Intel, Apple Silicon)
- Linux (x64, ARM64)

Manual installation options:
```bash
# Homebrew (macOS/Linux)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash

# From source
cargo install --git https://github.com/EffortlessMetrics/perl-lsp --bin perl-lsp
```

## ⚙️ Configuration

```json
{
  // Auto-download server if not found
  "perl-lsp.autoDownload": true,
  
  // Custom server path (optional)
  "perl-lsp.serverPath": "/usr/local/bin/perl-lsp",
  
  // Enable strict mode warnings
  "perl-lsp.enableStrictMode": false,
  
  // Maximum diagnostics per file
  "perl-lsp.maxNumberOfProblems": 100
}
```

## 🎯 Supported Perl Features

### Modern Perl (5.38+)
- `class/method/field` keywords
- `try/catch/finally` blocks  
- `defer` blocks
- Subroutine signatures
- Type constraints

### Complete Syntax Support
- Regular expressions with any delimiter (`m!pattern!`, `s{}{}`)
- Heredocs (all variants including indented)
- Unicode identifiers (`my $café = 'coffee'`)
- Postfix dereferencing (`$ref->@*`)
- Smart match operator (`~~`)
- Indirect object syntax

### Built-in Functions
Complete signatures for 150+ Perl built-ins with parameter documentation.

## 🤝 Compatibility

Works with any LSP-compatible editor:
- Visual Studio Code
- VSCodium  
- Cursor
- Gitpod
- GitHub Codespaces
- Neovim (via nvim-lspconfig)
- Emacs (via lsp-mode)

## 📚 Resources

- [Documentation](https://github.com/EffortlessMetrics/perl-lsp#readme)
- [Issue Tracker](https://github.com/EffortlessMetrics/perl-lsp/issues)
- [Changelog](https://github.com/EffortlessMetrics/perl-lsp/blob/master/CHANGELOG.md)
- [Migration Guide](https://github.com/EffortlessMetrics/perl-lsp/blob/master/MIGRATION.md)

## 📄 License

MIT © Tree-sitter Perl Team