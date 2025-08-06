# Release Notes - v0.7.1

## Overview
This patch release fixes critical parsing issues with built-in functions and empty blocks/hashes, ensuring correct AST generation for common Perl patterns.

## Bug Fixes

### Parser
- **Fixed `bless {}` parsing** - Previously incorrectly parsed as hash element access, now correctly recognized as function call with empty hash argument
- **Fixed empty block parsing for `sort`, `map`, `grep`** - These functions now properly handle empty blocks (`sort {} @array`)
- **Enhanced builtin function argument handling** - Better recognition of hash vs block contexts

### Code Quality
- Cleaned up unused imports and compiler warnings
- Added comprehensive test coverage (25+ new test cases)

## Testing
- Added dedicated test suite for `bless` variations (10 tests)
- Added test suite for builtin functions with empty blocks (15 tests)
- All 141 edge case tests continue to pass
- All 63+ LSP user story tests passing

## Performance
- No performance regression - parser maintains sub-millisecond response times
- Typical parsing: ~1-150µs for most files

## Compatibility
- Fully backward compatible with v0.6.0
- Tree-sitter S-expression format maintained
- LSP protocol compatibility preserved

## Installation

### From Source
```bash
cargo install --path crates/perl-parser --bin perl-lsp
```

### Update Existing Installation
```bash
cargo install --path crates/perl-parser --bin perl-lsp --force
```

## Next Steps
See [ROADMAP.md](ROADMAP.md) for upcoming features in the next release.

## Contributors
Thanks to all contributors who reported issues and helped test these fixes!