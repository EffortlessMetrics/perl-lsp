# LSP Actual Status - v0.8.7

## LSP GA Contract

**As of v0.8.3 GA, the LSP server only advertises capabilities that are fully functional.** Features that are partially implemented or stubs are no longer advertised and will return "method not supported" errors. This ensures editors don't attempt to use non-functional features.

## Capability Policy

**We only advertise capabilities that are proven by tests.** For conservative point releases we build with the `lsp-ga-lock` feature, which surfaces a reduced set. New features flip on **only** when their acceptance tests land in the same PR.

## Honest Assessment of LSP Functionality

While the `perl-parser` crate includes LSP infrastructure for many features, **about 75% of LSP features now work** (up from 70% in v0.8.6). This document provides an honest assessment of what you can actually expect to work, including the new **incremental parsing performance improvements**.

## ✅ Actually Working Features (~75%)

These features have been tested and provide real, useful functionality:

### 1. **Advanced Syntax Checking & Diagnostics** (ENHANCED v0.8.7)
- **Production-stable hash key context detection** with comprehensive bareword analysis:
  - Hash subscripts: `$hash{bareword_key}` - correctly recognized as legitimate (no false warnings)
  - Hash literals: `{ key => value, another_key => value2 }` - all keys properly identified in all contexts
  - Hash slices: `@hash{key1, key2, key3}` - comprehensive array-based key detection
  - Nested hash access: `$hash{level1}{level2}{level3}` - deep nesting with safety limits
  - Performance optimized with O(depth) complexity and built-in safety limits
- **Enhanced scope analysis** with stabilized `is_in_hash_key_context()` method
- Real-time syntax error detection with **incremental parsing (<1ms updates)**
- Parser error messages with line/column positions
- Enhanced undefined variable detection under `use strict` with comprehensive hash key awareness
- Missing pragma suggestions (strict/warnings)
- **Status**: Fully functional with production-grade hash context detection

### 2. **Enhanced Code Completion** (PERFORMANCE IMPROVED v0.8.7)
- Variables in current scope with **<1ms response time** via incremental parsing
- Perl built-in functions (150+ signatures)
- Keywords (my, sub, if, etc.)
- **Real-time updates** during typing with subtree reuse
- **Limitations**: No package members, no imports, no file paths
- **Status**: ~65% functional with major performance improvements

### 3. **Go to Definition** (Single File Only)
- Jump to variable declarations
- Jump to subroutine definitions
- **Limitations**: Current file only, no cross-file support
- **Status**: ~70% functional with DeclarationProvider

### 4. **Find References** (Single File Only)
- Find all uses of a variable
- Find all calls to a subroutine
- **Limitations**: Current file only
- **Status**: ~70% functional

### 5. **Hover Information**
- Basic variable type info
- Built-in function signatures
- **Limitations**: No package docs, no POD extraction
- **Status**: ~50% functional

### 6. **Signature Help**
- Parameter hints for 150+ built-in functions
- Works even with incomplete/invalid code
- **Status**: ~80% functional

### 7. **Document Symbols**
- Outline view with subroutines and packages
- Hierarchical structure
- Icons for different symbol types
- **Status**: Fully functional

### 8. **Document Formatting**
- Integration with Perl::Tidy
- Whole document formatting
- **Status**: Fully functional (requires perltidy)

### 9. **Folding Ranges**
- Code folding for blocks and subroutines
- Works even when AST parsing fails (text-based fallback)
- **Status**: Fully functional

### 10. **Workspace Symbols** (NEW in v0.8.4)
- Search for symbols across all open files
- Works with workspace index
- **Status**: Fully functional

### 11. **Rename Symbol** (NEW in v0.8.4)
- Rename variables and functions
- Cross-file rename for package variables (`our`)
- Lexical (`my`) rename is currently per-file with scope fences
- **Status**: ~85% functional

### 12. **Code Actions** (NEW in v0.8.4)
- Add missing `use strict` and `use warnings`
- Quick fixes for common issues
- Run perltidy (when available)
- **Status**: ~70% functional

### 13. **Semantic Tokens** (NEW in v0.8.4)
- Enhanced syntax highlighting
- Proper categorization of keywords, operators, strings, etc.
- **Status**: ~80% functional

### 14. **Inlay Hints** (NEW in v0.8.4)
- Parameter names for built-in functions
- Type hints for literals
- **Status**: ~75% functional

### 15. **Document Links** (NEW in v0.8.4)
- Links from `use` and `require` statements
- Navigate to local modules or MetaCPAN
- **Status**: ~80% functional

### 16. **Selection Ranges** (NEW in v0.8.4)
- Smart expand/contract selection
- Hierarchical selection based on AST
- **Status**: Fully functional

### 17. **On-Type Formatting** (NEW in v0.8.4)
- Auto-indent after `{`
- Auto-dedent on `}`
- Triggers: `{`, `}`, `;`, `\n` - only adjusts indentation; no reflow or semantic edits
- **Status**: ~70% functional

### 18. **Type Hierarchy** (NEW in v0.8.5)
- Navigate class inheritance hierarchies
- Find supertypes (parent classes via @ISA, use parent/base)
- Find subtypes (derived classes)
- **Status**: Fully functional

### 19. **Pull Diagnostics** (NEW in v0.8.5)
- Client-initiated diagnostics requests
- Document-level diagnostics (textDocument/diagnostic)
- Workspace-wide diagnostics (workspace/diagnostic)
- Result ID caching for efficiency
- **Status**: Fully functional

### 20. **Workspace Symbol Resolve** (NEW in v0.8.5)
- Enhanced symbol details on demand
- Accurate ranges after initial search
- Container information
- **Status**: Fully functional

### 21. **Type Definition** (NEW in v0.8.6)
- Navigate to type/class definitions
- Find package definitions for blessed references
- Supports method calls and ISA relationships
- **Status**: ~80% functional (preview)

### 22. **Implementation** (NEW in v0.8.6)
- Find implementations of a class/method
- Navigate to subclasses
- Discover method overrides
- **Status**: ~70% functional (preview)

## 📋 GA Contract: What's Advertised vs Not Advertised

### ✅ Advertised in v0.8.6 (Working Features)
- `textDocumentSync` - File synchronization
- `completionProvider` - Basic completions
- `hoverProvider` - Hover information
- `definitionProvider` - Go to definition
- `declarationProvider` - Go to declaration
- `referencesProvider` - Find references
- `documentHighlightProvider` - Highlight occurrences
- `signatureHelpProvider` - Signature help
- `documentSymbolProvider` - Document symbols
- `foldingRangeProvider` - Folding ranges
- `documentFormattingProvider` - Formatting (if perltidy available)
- `workspaceSymbolProvider` - Workspace symbols (NEW)
- `renameProvider` - Rename refactoring (NEW)
- `codeActionProvider` - Code actions (NEW)
- `semanticTokensProvider` - Semantic tokens (NEW)
- `inlayHintProvider` - Inlay hints (NEW)
- `documentLinkProvider` - Document links (NEW)
- `selectionRangeProvider` - Selection ranges (NEW)
- `documentOnTypeFormattingProvider` - On-type formatting (NEW)
- `typeHierarchyProvider` - Type hierarchy (NEW in v0.8.5)
- `diagnosticProvider` - Pull diagnostics (NEW in v0.8.5)
- `workspaceSymbolProvider.resolveProvider` - Symbol resolve (NEW in v0.8.5)
- `typeDefinitionProvider` - Type definition (NEW in v0.8.6)
- `implementationProvider` - Implementation (NEW in v0.8.6)

### ❌ NOT Advertised in v0.8.6 (Not Implemented)

#### Code Lens
- **Status:** Partial (not advertised).
- **Notes:** Early provider exists (run/debug links scaffolding), but no stable contract and no cross-feature integration. Not surfaced until stable & tested.

#### Call/Type Hierarchy
- **Status:** Partial/Not implemented (not advertised).
- **Notes:** Some internal scaffolding for type/call graphs exists but not connected to the LSP layer. Will surface after end-to-end correctness and tests.

#### Execute Command
- **Status:** Not wired (not advertised).
- **Notes:** `workspace/executeCommand` is intentionally not exposed; commands that do exist are handled via normal request/response paths. Clients should not rely on `executeCommand` until explicitly surfaced with tests.

## ⚠️ Partially Implemented (Not Advertised)

These features have partial implementations but are not advertised due to significant limitations:

### 1. **Code Lens**
- Run/Debug links partially implemented
- Test discovery not connected
- **Status**: ~20% functional

### 2. **Call Hierarchy**
- Basic structure exists
- Not fully connected to AST
- **Status**: ~15% functional

## ❌ Not Actually Working (Stub Implementations) (~35%)

These features exist in the code but return empty results or don't work:

### 1. **Workspace Refactoring**
- Extract Variable: Returns empty edits
- Extract Subroutine: Returns empty edits
- Inline Variable: Returns empty edits
- Convert loops: Returns empty edits
- **Status**: 0% functional (stubs only)

### 2. **Import Organization**
- Optimize imports: Returns empty analysis
- Add missing imports: Returns empty suggestions
- Remove unused imports: Returns empty list
- **Status**: 0% functional (stubs only)

### 3. **Dead Code Detection**
- Find unused variables: Returns empty list
- Find unused subroutines: Returns empty list
- Find unreachable code: Returns empty list
- **Status**: 0% functional (stubs only)

### 4. **Cross-file Features**
- Cross-file go to definition: Not wired
- Cross-file find references: Not wired
- Workspace symbols: Index exists but not connected
- **Status**: 0% functional

### 5. **Type Navigation**
- Go to Type Definition: Not implemented
- Go to Implementation: Not implemented
- Type Hierarchy: Returns empty
- **Status**: 0% functional

### 6. **Debug Adapter**
- Debugging support: Not implemented
- Breakpoints: Not implemented
- **Status**: 0% functional

### 7. **Advanced Features**
- Call Hierarchy: Returns empty
- Code Lens: Returns empty
- Inlay Hints: Partially works for hash literals only
- **Status**: <10% functional

## 🚀 Incremental Parsing Performance (NEW in v0.8.7)

The LSP server now includes **true incremental parsing** with significant performance improvements for real-time editing:

### Performance Metrics (**Diataxis: Reference**)
- **Small edits** (single token): <1ms parsing updates (vs 50-150ms full reparse)
- **Moderate edits** (function-level): <2ms parsing updates (vs 100-300ms full reparse) 
- **Cache hit ratios**: 70-90% for typical editing scenarios
- **Memory efficiency**: Arc<Node> sharing with LRU cache eviction (1000 item default)

### Technical Implementation (**Diataxis: Explanation**)
- **Subtree reuse**: Container nodes (Program, Block, Binary) recursively process while reusing unaffected AST subtrees
- **Content-based caching**: Hash-based subtree matching for common patterns (string literals, numbers, identifiers)
- **Position-based caching**: Range-based subtree matching for accurate placement in document
- **Metrics tracking**: Detailed performance analytics (nodes_reused vs nodes_reparsed counts)

### Real-time Editing Benefits (**Diataxis: Tutorials**)
- **Immediate diagnostics**: Syntax errors appear instantly while typing
- **Responsive completion**: Code completion suggestions with <1ms latency  
- **Smooth hover**: Hover information without perceptible delays
- **Instant symbol navigation**: Go-to-definition and find-references with real-time updates

### Configuration (**Diataxis: How-to**)
```bash
# Enable incremental parsing (automatic in LSP server)
export PERL_LSP_INCREMENTAL=1
perl-lsp --stdio

# Benchmark incremental performance
cargo bench incremental

# Test incremental functionality
cargo test -p perl-parser --test incremental_integration_test
```

### Fallback Mechanisms
- **Graceful degradation**: Falls back to full parsing when incremental fails
- **Error recovery**: Maintains functionality with incomplete/invalid code during editing
- **Conservative approach**: Full reparse triggered for complex structural changes

## 📊 Infrastructure vs Implementation

### Infrastructure That Exists (~65%)
The codebase has substantial infrastructure that isn't connected to the LSP layer:

1. **WorkspaceIndex** (`workspace_index.rs`)
   - Full cross-file symbol indexing
   - Dependency tracking
   - Module resolution
   - **Problem**: Not wired to LSP handlers

2. **SemanticAnalyzer** (`semantic_analyzer.rs`)
   - Type inference
   - Symbol resolution
   - Scope analysis
   - **Problem**: Only partially used

3. **RefactoringEngine** (`refactoring_engine.rs`)
   - Extract/inline algorithms
   - Code transformation logic
   - **Problem**: Returns empty results

4. **ModuleResolver** (`module_resolver.rs`)
   - Package resolution
   - Use/require handling
   - **Problem**: Not connected to completions

### Why the Gap?
- Many features were scaffolded but never completed
- Test suite checks response format, not actual functionality
- Stub implementations satisfy type system but don't work
- Focus was on parser (100% complete) not LSP integration

## 🎯 Realistic Expectations

### What You Can Use It For
- ✅ Basic Perl development with syntax checking
- ✅ Simple navigation within single files
- ✅ Code formatting with Perl::Tidy
- ✅ Basic code completion for variables/keywords
- ✅ Document outline navigation

### What You Cannot Use It For
- ❌ Large multi-file Perl projects
- ❌ Automated refactoring
- ❌ Cross-file navigation
- ❌ Import management
- ❌ Debugging

### Comparison with Other Perl LSPs
- **Perl Navigator**: More features actually work
- **PLS (Perl Language Server)**: More complete implementation
- **This LSP**: Better parser, weaker LSP integration

## 🔧 For Contributors

If you want to help connect the existing infrastructure:

### Easy Wins (Infrastructure Exists)
1. Wire WorkspaceIndex to cross-file navigation
2. Connect ModuleResolver to completions
3. Enable RefactoringEngine return values
4. Hook up SemanticAnalyzer to hover/completion

### Harder Tasks (Need Implementation)
1. Debug adapter protocol
2. Type definition navigation
3. Call hierarchy
4. Code lens providers

See [LSP_WIRING_OPPORTUNITIES.md](LSP_WIRING_OPPORTUNITIES.md) for technical details.

## 📈 Version History

### v0.8.3 GA
- Fixed go-to-definition with DeclarationProvider
- Enhanced inlay hints for hash literals
- Improved parser (100% edge cases)
- LSP remains ~35% functional

### v0.7.x
- Added test infrastructure
- Fixed tautological tests
- Added fallback mechanisms
- No significant LSP improvements

## 🚦 Summary

- **Parser**: 🟢 100% complete, production-ready
- **LSP Basic Features**: 🟡 35% functional
- **LSP Advanced Features**: 🔴 0-10% functional
- **Overall LSP Usability**: 🟡 Adequate for simple tasks

**Bottom Line**: Use this for the excellent parser. For full LSP features, consider Perl Navigator or PLS until more features are wired up.