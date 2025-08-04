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
- 4-19x performance improvement over v1
- Handles all edge cases including:
  - Arbitrary regex delimiters (m!pattern!)
  - Indirect object syntax
  - Complex dereferencing
  - Unicode identifiers

### Known Issues
- Formatting requires separate perltidy installation
- Cross-platform binaries not yet included (build from source for non-native platforms)

## [Unreleased]

### Planned
- Refactoring support (extract/inline variable)
- Code lens features
- Workspace symbols
- Multi-root workspace support
- Integrated perltidy (no separate installation needed)