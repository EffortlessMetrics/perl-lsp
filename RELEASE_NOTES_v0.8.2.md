# Release Notes - v0.8.2

## Overview

Version 0.8.2 brings significant improvements to the LSP server with new editor features, Windows compatibility fixes, and major code quality improvements to the v2 Pest-based parser.

## New Features

### LSP Server Enhancements

#### Document Links (`textDocument/documentLink`)
- **MetaCPAN Integration**: Module names in `use` and `require` statements are now clickable links to MetaCPAN documentation
- **Local File Navigation**: File paths in `require` and `do` statements become clickable links to local files
- **Windows-Safe URI Handling**: Proper cross-platform file URI resolution using the `url` crate

#### Selection Ranges (`textDocument/selectionRange`)
- Smart expanding selections from identifier → expression → statement → block → function
- Enables quick scope selection with keyboard shortcuts in supported editors

#### On-Type Formatting (`textDocument/onTypeFormatting`)
- Automatic indentation and formatting as you type
- Triggers on `{`, `}`, `)`, `;`, and newline characters
- Configurable tab size and spaces vs tabs preferences
- Smart indentation for nested blocks and control structures

#### Workspace File Watching
- Dynamic registration for `workspace/didChangeWatchedFiles`
- Monitors `*.pl`, `*.pm`, and `*.t` files for external changes
- Automatic re-indexing when files change outside the editor (when workspace feature is enabled)

### Incremental Parsing Infrastructure
- Foundation for future incremental parsing support
- Position mapping utilities for efficient text edits
- Prepared infrastructure for v0.8.3 performance improvements

## Code Quality Improvements

### v2 Pest-Based Parser Cleanup
- **90% reduction in clippy warnings** (143 → ~14)
- Fixed regex compilation errors and unsupported backreferences
- Replaced manual string operations with idiomatic Rust patterns
- Added crate-level lint configuration to prevent regressions
- Properly annotated intentionally unused code

### Bug Fixes
- Fixed Windows path handling in document links
- Corrected URI/URL conversion for cross-platform compatibility
- Fixed regex syntax errors in runtime heredoc handler
- Resolved multiple unused variable warnings

## Testing
- Added comprehensive tests for new LSP features
- URL handling tests for Windows and Unix paths
- Document link resolution tests

## Internal Changes
- Migrated from `lsp_types::Uri` string manipulation to proper `url::Url` parsing
- Improved error handling in LSP request processing
- Better separation of concerns for notification vs request handling

## Known Issues
- Workspace features remain behind feature flag (activate with `PERL_LSP_WORKSPACE=1`)
- Some LSP features may need editor-specific configuration

## Compatibility
- Compatible with VSCode, Neovim, Emacs, Sublime, and any LSP-compatible editor
- Full Windows, macOS, and Linux support
- Requires Rust 1.70+ for building from source

## What's Next (v0.8.3)
- Performance optimization for incremental parsing (<10ms for small edits)
- Workspace feature activation by default
- Windows CI testing matrix
- Further v2 parser cleanup to achieve zero warnings

## Installation

### Quick Install (Linux/macOS)
```bash
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash
```

### Homebrew (macOS)
```bash
brew tap tree-sitter-perl/tap
brew install perl-lsp
```

### From Source
```bash
cargo install --path crates/perl-parser --bin perl-lsp
```

## Contributors
Thanks to all contributors who helped make this release possible!

---

For detailed documentation, visit: https://docs.anthropic.com/en/docs/claude-code
Report issues at: https://github.com/EffortlessSteven/tree-sitter-perl/issues