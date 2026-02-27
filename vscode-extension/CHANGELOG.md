# Change Log

All notable changes to the Perl Language Server extension will be documented in this file.

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

### Parser Features
- Uses tree-sitter-perl v3 parser
- 100% Perl 5 syntax coverage
- 1-150μs typical parse times
- Handles all edge cases including:
  - Arbitrary regex delimiters (m!pattern!)
  - Indirect object syntax
  - Complex dereferencing
  - Unicode identifiers

### Known Issues
- Formatting requires separate perltidy installation
- Cross-platform binaries not yet included (build from source for non-native platforms)

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
- ⚙️ **New Configuration Options**
  - `perl.inlayHints.*` - Control inlay hint behavior
  - `perl.testRunner.*` - Configure test execution
- **New Commands**
  - `perl.runTest` - Run a specific test
  - `perl.runTestFile` - Run all tests in a file
  - `perl.debugTest` - Debug a test

### Enhanced
- Added "Testing" category to extension capabilities
- Improved activation events for test files
- Better TypeScript types and error handling

### Fixed
- Improved handling of anonymous subroutines in navigation features
- Better error recovery for malformed syntax
- Fixed race conditions in document synchronization

## [Unreleased]

### Added
- **TCP Socket Mode**: LSP server can now listen on TCP sockets in addition to stdio
- **Uninitialized Variable Detection**: Semantic analyzer detects use of uninitialized variables

### Changed
- **Performance**: O(1) symbol lookups, optimized scope analysis
- **Code Quality**: Unified position/range types, improved code formatting

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

### Planned
- Multi-root workspace support
- Integrated perltidy (no separate installation needed)
- Performance profiling tools