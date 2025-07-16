# Tree-sitter Perl - Legacy C Implementation

This directory contains the legacy C implementation of the tree-sitter Perl grammar.

## Structure

- `parser.c` - Generated parser from grammar.js
- `scanner.c` - C scanner implementation
- `tsp_unicode.h` - Unicode support headers
- `bsearch.h` - Binary search utilities
- `grammar.js` - Tree-sitter grammar definition
- `grammar.json` - Generated grammar JSON
- `node-types.json` - Generated node types
- `bindings/` - Language bindings (Rust, Go, Node.js, Python, Swift)
- `test/` - Test corpus and highlight tests
- `lib/` - Grammar utilities and primitives
- `queries/` - Tree-sitter query files

## Build Files

- `CMakeLists.txt` - CMake build configuration
- `Makefile` - Make build configuration
- `binding.gyp` - Node.js binding configuration
- `package.json` - Node.js package configuration
- `setup.py` - Python setup configuration
- `pyproject.toml` - Python project configuration
- `go.mod` - Go module configuration
- `Package.swift` - Swift package configuration
- `cpanfile` - Perl dependencies

## Usage

This directory serves as the legacy C implementation root. The Rust implementation is now in `/crates/tree-sitter-perl`.

For development, use the Rust implementation in the parent directory. 