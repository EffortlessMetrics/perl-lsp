# Release Notes - v0.8.1

## 🚀 VSCode Extension Launch

This release marks the official launch of the Perl LSP VSCode extension on the marketplace!

### ✨ Extension Highlights

- **Auto-download**: Extension automatically downloads the correct LSP binary for your platform
- **SHA256 Verification**: All downloads are cryptographically verified
- **Cross-platform**: Windows, macOS, Linux (x64 and ARM64)
- **Zero Configuration**: Works out of the box with smart defaults
- **40+ Snippets**: Productivity-boosting Perl code snippets included

### 📦 New Distribution Channels

- **VSCode Marketplace**: Install directly from VSCode
- **Open VSX**: Available for VSCodium and other editors
- **Homebrew**: `brew install perl-lsp` (auto-updates)
- **Linux Packages**: .deb and .rpm packages for all architectures

### 🛠️ Features

All 26+ LSP features are now easily accessible through VSCode:

- **IntelliSense**: Smart completions with documentation
- **Diagnostics**: Real-time error detection
- **Go to Definition**: Navigate to symbols instantly
- **Find References**: See all usages across your project
- **Rename**: Safe refactoring across files
- **Hover**: Rich documentation on hover
- **Signature Help**: Parameter hints while typing
- **Code Actions**: Quick fixes and refactorings
- **Document Symbols**: Outline view support
- **Semantic Highlighting**: Context-aware syntax colors
- **And 16+ more features...**

### 🔧 Improvements

- Enhanced downloader with .tar.xz support
- Multi-pattern artifact name resolution
- Improved error messages with asset listing
- Added LSP trace setting for debugging
- Virtual workspace support
- Untrusted workspace support

### 📚 Documentation

- Comprehensive EXTENSION.md guide
- MIGRATION.md for v0.8.0 breaking changes
- Updated CONTRIBUTING.md with VSCode dev guide
- PUBLISHER_SETUP.md for maintainers

### 🙏 Thank You

Thanks to all contributors and early testers who helped make this release possible!

## Installation

### VSCode
```
ext install tree-sitter-perl.perl-lsp
```

### Command Line
```bash
# macOS
brew install perl-lsp

# Linux/macOS (shell installer)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

# From source
cargo install --git https://github.com/EffortlessSteven/tree-sitter-perl --bin perl-lsp
```

## What's Next

- Video tutorials and demos
- Integration with more editors
- Performance optimizations
- Additional refactoring tools

Report issues: https://github.com/EffortlessSteven/tree-sitter-perl/issues