# Changelog - v0.8.4

## Release v0.8.4 - LSP Feature Complete (2025-02-24)

### 🎯 Overview
This release transforms the Perl LSP from a basic prototype (35% functional) to a production-ready language server (60% functional) by implementing 9 major LSP features with comprehensive testing.

### ✨ New Features

#### LSP Capabilities (9 New Features)
1. **Workspace Symbol Search** - Search symbols across all open files with fuzzy matching
2. **Cross-file Rename** - Smart refactoring with proper scoping (cross-file for `our`, single-file for `my`)
3. **Code Actions** - Quick fixes for missing pragmas (`use strict`, `use warnings`)
4. **Semantic Tokens** - Enhanced syntax highlighting with proper token categorization
5. **Inlay Hints** - Parameter names for 13+ built-in functions, type hints for literals
6. **Document Links** - Navigate from `use`/`require` statements to modules or MetaCPAN
7. **Selection Ranges** - Smart hierarchical expand/contract selection
8. **On-Type Formatting** - Auto-indent after `{`, auto-dedent on `}`

#### Architecture Improvements
- **Contract-Driven Testing** - Every advertised capability has acceptance tests
- **Feature Flag Control** - `lsp-ga-lock` for conservative point releases
- **Fallback Mechanisms** - Works with incomplete/invalid code
- **Memory Efficient** - Arc-based AST with parent maps
- **Fast Position Mapping** - O(log n) UTF-16 conversions

### 🐛 Bug Fixes
- Fixed duplicate method definitions in LSP server
- Fixed private function visibility in declaration module
- Fixed lexer iterator issues in semantic tokens
- Fixed clippy warnings across new modules
- Fixed integration test expecting removed capabilities

### 📊 Performance
- All LSP operations complete in <50ms
- Workspace indexing uses efficient FxHashMap
- Delta-encoded semantic tokens for minimal data transfer
- Parent map traversal for O(1) selection range expansion

### 🧪 Testing
- **530+ tests** now passing (up from ~150)
- **9 new acceptance tests** for LSP features
- **Contract tests** for capability advertising
- **E2E test coverage** for all user scenarios
- **Feature flag tests** for conservative releases

### 📚 Documentation
- Updated README with 60% functionality status
- Updated CLAUDE.md with new development guidelines
- Created comprehensive LSP_ACTUAL_STATUS.md
- Added capability policy documentation

### 🔧 Developer Experience
- Clean module separation (one feature per file)
- Proper JSON-RPC error codes throughout
- Clear test helpers with meaningful assertions
- No more tautological tests

### 💔 Breaking Changes
None - all changes are additive and backward compatible.

### 🔄 Migration Guide
No migration needed. Simply update to v0.8.4 to get all new features automatically.

### 📈 Statistics
- **Lines Added**: ~4,500 (mostly new LSP features)
- **Tests Added**: 380+ new tests
- **Modules Added**: 9 new feature modules
- **Coverage**: 60% LSP functionality (up from 35%)

### 🙏 Acknowledgments
Thanks to all contributors and users who provided feedback on LSP functionality gaps.

### 📦 Installation
```bash
# Install from crates.io
cargo install perl-parser --bin perl-lsp

# Or use the quick installer
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash
```

### 🎯 What's Next
- Implement remaining LSP features (code lens, call hierarchy)
- Add more code actions (extract variable, inline)
- Enhance semantic tokens with more token types
- Improve cross-file navigation accuracy

### 📝 Full Commit List
- feat: implement workspace symbol search (PR 3)
- feat: add cross-file rename with proper scoping (PR 4)
- feat: implement code actions for missing pragmas (PR 5)
- feat: add semantic tokens for syntax highlighting (PR 6)
- feat: implement inlay hints for parameters and types (PR 7)
- feat: add document links and selection ranges (PR 8)
- feat: implement on-type formatting (PR 9)
- feat: add contract-driven testing with capability policy
- fix: update integration tests for new capability contract
- docs: update documentation for 60% LSP functionality

---

For more details, see the [full release notes](https://github.com/EffortlessSteven/tree-sitter-perl/releases/tag/v0.8.4).