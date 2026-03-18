# Change Log

All notable changes to the Perl Language Server extension will be documented in this file.

## [Unreleased]

### Added
- Open VSX packaging/publishing scripts plus release documentation so the extension can be validated and distributed to VSCodium-compatible editors alongside the Visual Studio Marketplace.

## [0.11.0] - 2026-03-11

### Fixed
- **Corrupted Extension Icon**: Added binary rules for image files in `.gitattributes` to prevent line-ending normalization from corrupting PNGs; regenerated `icon.png` with correct PNG signature header.
- **TextMate Grammar Registration**: Registered `syntaxes/perl.tmLanguage.json` in `contributes.grammars` so VS Code actually loads the bundled grammar.
- **Stub Refactoring Commands**: Removed `Extract Subroutine`, `Extract Variable`, and `Inline Variable` from editor context and command palette menus (still available from the full command list with "in development" messaging).
- **VSCE Packaging**: Removed `--no-dependencies` flag so runtime dependencies (`vscode-languageclient`, `adm-zip`, `tar`) are properly bundled in the `.vsix`.
- **pnpm Compatibility**: Made `vsce package`/`publish` resilient in pnpm environments.

### Changed
- **Parser Bug Fixes** (6 fixes in the underlying `perl-lsp` server):
  - Handle postfix modifiers after dereference expressions (`$obj->method if $cond`)
  - Re-lex slash as regex delimiter after `split` keyword
  - Accept fat arrow as argument separator in function calls
  - Handle subscripts on package-qualified variables (`$Foo::bar[0]`)
  - Support `&{expr}` code dereference syntax
  - Expand builtin list and support forward declarations
- **Lexer Bug Fixes** (2 fixes):
  - Skip POD blocks in whitespace/comment handler
  - Make POD scanning byte-safe for UTF-8 source files
- **Regex Engine Fixes** (2 fixes):
  - Eliminate false positive nested quantifier detection
  - Clean up false positive regex detection
- **LSP Feature Polish**: Maintained 100% user-visible feature coverage and protocol compliance.
- **Documentation**: Comprehensive release readiness updates including README, roadmap, and launch article series.

### Added
- Marketplace readiness workflow via `npm run verify:marketplace` for compile + bundle + package validation.
- VS Marketplace badges and installation link in extension README.
- Publishing guide refreshed with launch checklist and pre-release recommendation.
- SRP microcrate extraction campaign: 30+ new single-responsibility microcrates extracted.
- Comprehensive unit test campaign: 80+ new test modules across all crate tiers.

## [0.10.0] - 2026-02-28

### Changed
- **Version Sync**: Extension version aligned with workspace v0.10.0.
- **LSP Coverage**: Maintained 100% user-visible feature coverage (53/53).
- **Protocol Compliance**: Maintained 100% protocol compliance (97/97).

## [0.9.0] - 2026-01-18

### Added
- 🔧 **Advanced Refactoring Support**
  - Extract method refactoring with parameter detection
  - Inline variable/expression refactoring
  - Move code refactoring for relocating code blocks
  - Transactional safety with rollback infrastructure
- 🎯 **Semantic Definition Integration**
  - Precise go-to-definition using semantic analysis instead of text search
  - Multi-symbol support: scalars, arrays, hashes, subroutines, packages
  - Lexical scoping with proper handling of nested scopes and shadowed variables
- 🔒 **Security Hardening**
  - Complete path traversal protection for execute commands
  - Command injection hardening in executeCommand
- ⚡ **Performance Optimizations**
  - O(1) symbol lookups (from linear time)
  - Stack-based scope analysis for improved performance
  - Reduced string allocations in parser
- 🎨 **Product Icons**: Added icons to extension commands
- 📋 **Context Menu**: Run Tests exposed in editor context menu

### Changed
- Cross-file Package->method resolution improved
- Better error logging for incremental document changes
- Configuration setting descriptions improved

## [0.8.0] - 2025-09-01

### Added
- **Cross-File Navigation**: Workspace indexing with dual storage pattern for qualified and bare names
- **Import Optimization**: Detect and organize imports, remove unused imports
- **Incremental Parsing V2**: Advanced edit tracking with node reuse for faster re-parsing
- **File Path Completion**: Enterprise-grade file completion with security safeguards

### Changed
- Optimized workspace indexing for large codebases
- Enhanced comment documentation extraction for hover

## [0.7.0] - 2025-08-24

### Added
- **LSP 3.17 Features**: Inlay hints, document links, selection ranges, on-type formatting
- **Code Actions**: Robust refactoring and quick fixes
- **Type Hierarchy**: View inheritance relationships
- **Rename Support**: Symbol renaming with validation

## [0.6.0] - 2025-01-29

### Added
- 🔍 **Call Hierarchy Support**
  - View incoming calls (functions that call the selected function)
  - View outgoing calls (functions called by the selected function)
  - Navigate complex call chains with ease
  - Right-click any function and select "Show Call Hierarchy"
- 💡 **Inlay Hints**
  - Parameter name hints for function calls
  - Type hints for variable declarations
  - Smart filtering to reduce visual clutter
  - Fully configurable via settings
- 🧪 **Test Explorer Integration**
  - Automatic discovery of test files (.t) and test functions
  - Visual test hierarchy in Testing panel
  - Run individual tests or entire test files
  - Real-time test results with pass/fail indicators
  - TAP (Test Anything Protocol) support
- 🐛 **Debug Adapter Protocol Support**
  - Full step-through debugging for Perl scripts
  - Breakpoints with conditional support
  - Variable inspection and watch expressions
  - Call stack navigation
  - Test debugging integration
  - Debug configurations for scripts and tests
- ⚡ **Performance Optimizations**
  - AST caching for faster parsing (100 files, 5-min TTL)
  - Symbol index for instant workspace searches
  - 10x faster symbol lookup in large projects

### Enhanced
- Added "Testing" category to extension capabilities
- Improved activation events for test files
- Better TypeScript types and error handling

### Fixed
- Improved handling of anonymous subroutines in navigation features
- Better error recovery for malformed syntax
- Fixed race conditions in document synchronization

## [0.5.0] - 2025-01-01

### Added
- Initial release of Perl Language Server for Visual Studio Code
- Full Language Server Protocol support with 8 core features:
  - Real-time syntax diagnostics
  - Code completion with context awareness
  - Go to definition
  - Find all references
  - Document symbols (outline)
  - Signature help
  - Hover information
  - Code actions (quick fixes)
- Code formatting with Perl::Tidy integration
  - Format document (Shift+Alt+F)
  - Format selection
  - Automatic .perltidyrc discovery
- Enhanced syntax highlighting
- Commands:
  - Restart Language Server
  - Show Language Server Output
- Bundled perl-lsp binary for easy installation
- Support for modern Perl features (try/catch, signatures, class/method)