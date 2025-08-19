# LSP Wiring Opportunities - What's Already Built

## Executive Summary

**The v3 parser has ~70% of the infrastructure already built** - it just needs proper wiring to the LSP server. Most "stub" features could be made functional by connecting existing parser capabilities to the LSP layer.

## Already Implemented in Parser (Just Needs Wiring)

### 1. Workspace Indexing (`workspace_index.rs`) ✅ FULLY IMPLEMENTED
The parser **already has** a complete workspace indexing system:
- `WorkspaceIndex::index_file()` - Parses and indexes entire files
- `WorkspaceIndex::find_symbols()` - Searches across all files
- `WorkspaceIndex::find_definition()` - Cross-file go-to-definition
- `WorkspaceIndex::find_references()` - Cross-file find references
- `WorkspaceIndex::find_dependents()` - Tracks module dependencies

**Status**: Feature-gated behind `workspace` feature flag (not enabled by default)
**To Enable**: Add to Cargo.toml default features or compile with `--features workspace`

### 2. Semantic Analysis (`semantic.rs`) ✅ FULLY IMPLEMENTED
Complete semantic analyzer exists:
- `SemanticAnalyzer::find_definition()` - Resolves symbol definitions
- `SemanticAnalyzer::symbol_at()` - Gets symbol info at position
- Symbol table with scopes and references
- Type inference for variables

**Status**: Working but not fully connected to LSP handlers

### 3. Code Actions & Refactoring (`code_actions_enhanced.rs`) ✅ PARTIALLY IMPLEMENTED
Real refactoring logic exists:
- `create_extract_variable_action()` - Extract variable refactoring
- `create_extract_subroutine_action()` - Extract function
- `convert_loop_style()` - Convert between loop styles
- `add_error_checking()` - Add error handling

**Status**: Implemented but returns actual edits only for some operations

### 4. Module Resolution (`lsp_server.rs::resolve_module_to_path()`) ✅ IMPLEMENTED
Module resolution logic exists:
- Converts `Module::Name` to file paths
- Searches @INC paths
- Caches results for performance

**Status**: Working but limited to basic cases

### 5. Symbol Management ✅ FULLY IMPLEMENTED
Complete symbol infrastructure:
- `SymbolTable` with definitions and references
- `SymbolIndex` for fast lookups
- `DocumentSymbolProvider` for outline
- Full AST traversal for symbol extraction

## What's Actually Missing vs Stubbed

### Actually Stubbed (No Implementation)
1. **workspace_refactor.rs** - All methods return empty edits
2. **import_optimizer.rs** - Returns empty analysis
3. **dead_code_detector.rs** - Returns empty results
4. **debug_adapter.rs** - All TODOs

### Has Implementation But Not Wired
1. **Cross-file navigation** - WorkspaceIndex exists but feature-gated
2. **Workspace symbols** - Index exists but not enabled by default
3. **Advanced refactoring** - Logic exists in code_actions_enhanced.rs
4. **Import tracking** - Parser tracks `use` statements but not analyzed

## Quick Wins - Enable Existing Features

### 1. Enable Workspace Features (1 line change)
```toml
# In Cargo.toml
[features]
default = ["workspace"]  # Add workspace to default features
```

This immediately enables:
- Cross-file go-to-definition
- Cross-file find references
- Workspace symbol search
- Module dependency tracking

### 2. Wire Existing Refactoring (Small changes)
The `code_actions_enhanced.rs` has the logic but needs to return actual edits:
```rust
// Currently returns placeholder
fn create_extract_variable_action(&self, node: &Node) -> CodeAction {
    // Logic exists to calculate edits
    // Just needs to return them instead of empty vec
}
```

### 3. Connect Semantic Analyzer (Already used partially)
The semantic analyzer is already used for some features but could power more:
- Hover information (partially connected)
- Type information (has inference, not fully exposed)
- Unused variable detection (implemented, not wired to diagnostics)

## Effort Estimate to Wire Existing Features

### Minimal Effort (Hours)
1. **Enable workspace feature by default** - 1 line change
2. **Wire semantic analyzer to hover** - ~50 lines
3. **Connect existing module resolution** - ~100 lines

### Small Effort (Days)
1. **Complete extract variable/function** - ~200 lines
2. **Wire symbol index to completion** - ~150 lines
3. **Connect type inference to hover** - ~100 lines

### Medium Effort (Weeks)
1. **Complete import analysis** - Build on existing `use` tracking
2. **Basic dead code detection** - Use existing symbol references
3. **Finish workspace refactoring** - Extend existing code actions

## The Reality

**~70% of the "missing" LSP features have working implementations in the parser** that just need to be:
1. Enabled (workspace feature flag)
2. Wired to LSP handlers (connection code)
3. Completed (some have partial implementations)

The parser team built a solid foundation. The LSP layer just hasn't fully utilized it yet.

## Recommended Immediate Actions

### 1. Enable Workspace Feature
```bash
# Build with workspace features
cargo build --features workspace

# Or add to default features in Cargo.toml
```

### 2. Complete Existing Refactorings
Focus on `code_actions_enhanced.rs` - the logic is there, just needs to return real edits instead of placeholders.

### 3. Wire Semantic Analyzer
The semantic analyzer has rich information that's not fully exposed through LSP.

### 4. Document What's Actually Available
Update docs to show which parser features exist but need LSP wiring vs truly missing.

## Code Examples of Existing Infrastructure

### WorkspaceIndex Already Works
```rust
// This already exists and works!
let workspace_index = WorkspaceIndex::new();
workspace_index.index_file(url, content)?;
let definition = workspace_index.find_definition("some_function");
let references = workspace_index.find_references("$variable");
```

### Semantic Analysis Already Works
```rust
// This already provides rich information!
let analyzer = SemanticAnalyzer::analyze(ast);
let symbol = analyzer.find_definition(position);
let type_info = analyzer.infer_type(node);
```

### Refactoring Logic Exists
```rust
// The logic is implemented, just needs completion!
let provider = EnhancedCodeActionsProvider::new(source);
let actions = provider.get_enhanced_refactoring_actions(ast, range);
// Actions are calculated but edits might be empty - needs finishing
```

## Conclusion

The claim that the LSP is "35% functional" is somewhat misleading. The **parser** has ~70% of the required infrastructure built. The issue is:

1. **Not enabled by default** (workspace feature flag)
2. **Not fully wired** (connection code missing)
3. **Some incomplete implementations** (return empty instead of real results)

With focused effort on wiring and completion rather than reimplementation, the LSP could reach 70-80% functionality relatively quickly.