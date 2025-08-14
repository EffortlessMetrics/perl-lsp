# Release Notes - v0.8.2

## What's New

This release brings **significant LSP enhancements** and **code quality improvements** to the Perl Language Server.

## Added

### New LSP Features
- **`textDocument/documentLink`** - Navigate to modules and files directly from your code
  - MetaCPAN links for `use Module::Name` statements
  - Local file links for `require` and `do` statements  
  - Windows-safe path handling with proper URI resolution
- **`textDocument/selectionRange`** - Smart selection expansion
  - Progressively expand from identifier → expression → statement → block → function
- **`textDocument/onTypeFormatting`** - Auto-formatting as you type
  - Triggers on `{`, `}`, `)`, `;`, and newline
  - Smart indentation handling
- **`workspace/didChangeWatchedFiles`** - Live file synchronization
  - Dynamic registration for `*.pl`, `*.pm`, `*.t` files
  - Automatic re-indexing on external changes

### Testing Infrastructure
- Added comprehensive test suite for new LSP features
- Enhanced test helpers with robust assertion methods
- Added `completion_items()` helper for future-proof testing

## Changed

### Internal Improvements
- Enhanced response reading to handle multiple messages correctly
- Improved URI handling for cross-platform compatibility
- Updated CI configuration with split clippy handling:
  - **v3 (perl-parser)**: Strict warnings-as-errors
  - **v2 (tree-sitter-perl-rs)**: Warnings allowed for now

### Code Quality
- Fixed all tautological test assertions
- All LSP completion tests now passing (17/17)
- Zero clippy warnings in perl-parser and perl-lexer

## Technical Details

- **Performance**: All new features maintain sub-50ms response times
- **Compatibility**: Full support for Windows, macOS, and Linux
- **Standards**: Compliant with LSP 3.17 specification

## Installation

```bash
# Quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

# Homebrew (macOS)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# Build from source
cargo install --git https://github.com/EffortlessSteven/tree-sitter-perl --tag v0.8.2 perl-parser --bin perl-lsp
```

## Coming Next (v0.8.3)

- Workspace scanning enabled by default
- Enhanced method completion with type inference
- Zero warnings in v2 parser crate
- Windows CI matrix expansion

---

**Full Changelog**: https://github.com/EffortlessSteven/tree-sitter-perl/compare/v0.8.1...v0.8.2