# Release Notes - v0.8.6

## 🎉 Highlights

This release brings **production-clean implementations** of Type Definition and Implementation providers, advancing our LSP compliance to **~70%** (up from 65% in v0.8.5).

## ✨ New Features

### Type Definition Provider (Preview)
- Navigate to type/class definitions for blessed references
- Find package definitions from method calls
- Support for ISA relationships
- **Real UTF-16 positions** with CRLF/emoji support
- **Typed Location/LocationLink objects** (not JSON)

### Implementation Provider (Preview)
- Find implementations of a class/method
- Navigate to subclasses that inherit from a package
- Discover method overrides
- Single-file support (multi-file coming in future release)

## 🛠️ Improvements

### Position Handling
- Added comprehensive UTF-16 position conversion infrastructure
- Proper handling of Windows line endings (\r\n)
- Correct position calculation for multi-byte characters and emojis
- No more dummy (0,0) positions

### Architecture
- **Single Source of Truth** for LSP capabilities
- Features controlled by catalog-driven system
- De-duplicated URI parsing helper function
- Clean separation of concerns

### Code Quality
- Fixed all clippy warnings
- Removed unused imports and dead code
- Consistent code formatting
- Added comprehensive test coverage

## 📊 Metrics

- **LSP Compliance**: ~70% (up from 65%)
- **Test Coverage**: 82% feature compliance
- **Performance**: <50ms for all LSP operations
- **Code Quality**: Zero warnings, all tests passing

## 🧪 Testing

- Added CRLF/emoji regression tests
- Position verification assertions  
- Comprehensive E2E test coverage
- All features backed by acceptance tests

## 📦 Installation

```bash
# Install from crates.io
cargo install perl-parser --bin perl-lsp

# Or build from source
cargo build -p perl-parser --bin perl-lsp --release
```

## 🔄 Breaking Changes

None. This release maintains backward compatibility with v0.8.5.

## 🐛 Bug Fixes

- Fixed position calculations for CRLF line endings
- Corrected UTF-16 code unit counting for emojis
- Resolved all clippy style issues

## 📚 Documentation

- Updated ROADMAP.md with v0.8.6 status
- Updated LSP_ACTUAL_STATUS.md with new features
- Features catalog reflects preview status

## Contributors

Thanks to everyone who contributed to this release through testing, bug reports, and code contributions.

---

**Full Changelog**: https://github.com/tree-sitter-perl/tree-sitter-perl/compare/v0.8.5...v0.8.6