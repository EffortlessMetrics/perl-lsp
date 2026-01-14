# Engine/LSP Separation Migration Plan

**Executive Summary**

This document provides a comprehensive migration plan for refactoring [`perl-parser`](crates/perl-parser/) to be LSP-agnostic and moving LSP concerns to [`perl-lsp`](crates/perl-lsp/). The goal is to achieve "End state B" where:
- **[`perl-parser`** is the pure engine (parse/AST/recovery, workspace index + symbol graph + reference tracking, feature engines that return internal types)
- **[`perl-lsp`** is the protocol adapter + runtime (message loop, cancellation, transport, capability negotiation + lsp_types conversions, document sync, file watching, config/logging, process/FS-dependent behaviors)

This migration is critical for enabling wasm32 compilation and maintaining backward compatibility during the transition.

---

## Phase 1: Move LSP Conversions Out of Core Types

### 1.1 Core Types with `to_lsp_*()` Methods

The following core types in [`perl-parser`](crates/perl-parser/src/) have LSP conversion methods that need to be moved:

| Type | Current Location | Target Location | Action |
|------|----------|----------|------|
| [`SymbolKind`](crates/perl-parser/src/workspace_index.rs:857) | `lsp_types::SymbolKind` | Move to [`perl-lsp/src/types.rs`](crates/perl-lsp/src/types.rs) |
| [`Location`](crates/perl-parser/src/workspace_index.rs:822) | `to_lsp_location()` | Move to [`perl-lsp/src/types.rs`](crates/perl-lsp/src/types.rs) |
| [`Range`](crates/perl-parser/src/workspace_index.rs:826) | `to_lsp_range()` | Move to [`perl-lsp/src/types.rs`](crates/perl-lsp/src/types.rs) |
| [`Position`](crates/perl-parser/src/position.rs:23) | `to_lsp_position()` | Move to [`perl-lsp/src/types.rs`](crates/perl-lsp/src/types.rs) |
| [`WorkspaceSymbol`](crates/perl-parser/src/workspace_index.rs:831) | `to_lsp_workspace_symbol()` | Move to [`perl-lsp/src/types.rs`](crates/perl-lsp/src/types.rs) |

**Internal Type Replacements**:
- Create internal `SymbolKindInternal` enum to replace `lsp_types::SymbolKind`
- Create internal `LocationInternal` with `uri: String` and `range: RangeInternal`
- Create internal `RangeInternal` with `start: PositionInternal` and `end: PositionInternal`
- Create internal `PositionInternal` with `byte: usize, line: u32, column: u32`
- LSP types will be created in [`perl-lsp/src/types.rs`](crates/perl-lsp/src/types.rs) from internal types

### 1.2 LSP-Facing Files Revert to lsp_types Imports

The following files currently use [`lsp_types`](crates/perl-parser/src/lsp_types) imports and should revert to use internal types:

| File | Current Import | Action |
|------|----------|----------|------|
| [`type_definition.rs`](crates/perl-parser/src/type_definition.rs) | `use lsp_types::{LocationLink, Position, Range}` | Revert to internal types |
| [`workspace_rename.rs`](crates/perl-parser/src/workspace_rename.rs) | `use lsp_types::{Position, Range}` | Revert to internal types |
| [`incremental/mod.rs`](crates/perl-parser/src/incremental/mod.rs) | `use lsp_types::{Position, Range, TextDocumentContentChangeEvent}` | Revert to internal types |

### 1.3 Remove LSP Conversion Methods from Core Types

After creating internal types, remove the `to_lsp_*()` methods from core types:
- Remove `SymbolKind::to_lsp_kind()` from [`SymbolKind`](crates/perl-parser/src/workspace_index.rs:857)
- Remove `Location::to_lsp_location()` from [`Location`](crates/perl-parser/src/workspace_index.rs:822)
- Remove `Range::to_lsp_range()` from [`Range`](crates/perl-parser/src/workspace_index.rs:826)
- Remove `Position::to_lsp_position()` from [`Position`](crates/perl-parser/src/position.rs:23)

### 1.4 Update Type References in workspace_index.rs

Update [`workspace_index.rs`](crates/perl-parser/src/workspace_index.rs) to use internal types:
- Change `Location` to use `LocationInternal` (internal Range)
- Change `WorkspaceSymbol.range` to use `RangeInternal` (internal Range)
- Keep `SymbolKind` as is (internal enum, no LSP conversion needed)

### 1.5 Update Type References in Other Core Modules

Search and update other core modules that reference LSP types:
- [`workspace_refactor.rs`](crates/perl-parser/src/workspace_refactor.rs) - Already uses internal types after fix
- [`document_store.rs`](crates/perl-parser/src/document_store.rs) - Check for LSP type usage
- [`incremental/mod.rs`](crates/perl-parser/src/incremental/mod.rs) - Check for LSP type usage
- [`implementation_provider.rs`](crates/perl-parser/src/lsp/implementation_provider.rs) - Check for LSP type usage

### 1.6 Create Internal Types Module

Create [`crates/perl-parser/src/internal_types.rs`](crates/perl-parser/src/internal_types.rs) with:
- `PositionInternal { byte: usize, line: u32, column: u32 }`
- `RangeInternal { start: PositionInternal, end: PositionInternal }`
- `LocationInternal { uri: String, range: RangeInternal }`
- `SymbolKindInternal { ... }` (enum without LSP conversions)
- `DiagnosticInternal { ... }` (if needed, otherwise use lsp_types::Diagnostic)

Add UTF-16 conversion functions to internal types module.

---

## Phase 2: Move Runtime to perl-lsp

### 2.1 Modules to Move

Move the following modules from [`crates/perl-parser/src/lsp/`](crates/perl-parser/src/lsp/) to [`crates/perl-lsp/src/`](crates/perl-lsp/src/):

| Module | Current Location | Target Location | Dependencies |
|------|----------|----------|------|
| `server.rs` | `lsp/server.rs` | Move to `perl-lsp/src/server.rs` | Update imports |
| `transport.rs` | `lsp/transport.rs` | Move to `perl-lsp/src/transport.rs` | Update imports |
| `document_sync.rs` | `lsp/document_sync.rs` | Move to `perl-lsp/src/document_sync.rs` | Update imports |
| `cancellation.rs` | `lsp/cancellation.rs` | Move to `perl-lsp/src/cancellation.rs` | Update imports |
| `capabilities.rs` | `lsp/capabilities.rs` | Move to `perl-lsp/src/capabilities.rs` | Update imports |

### 2.2 Target Directory Structure in perl-lsp

Create the following directory structure in [`crates/perl-lsp/src/`](crates/perl-lsp/src/):

```
src/
├── server.rs          # Main server loop and LSP message handling
├── transport.rs        # Transport layer (stdio, TCP)
├── document_sync.rs    # Document synchronization and watching
├── cancellation.rs    # Cancellation token management
├── capabilities.rs     # Capability negotiation and server info
├── types.rs            # LSP type definitions and conversions
├── features/           # LSP feature implementations (moved from perl-parser)
└── adapters/          # Adapter functions that convert engine types to LSP types
```

### 2.3 Update Imports in Moved Modules

Update imports in moved modules to use internal types from [`internal_types.rs`](crates/perl-lsp/src/internal_types.rs):
- `server.rs` - Update `Position`, `Range`, `Diagnostic` imports
- `transport.rs` - Update `Position`, `Range` imports
- `document_sync.rs` - Update `Position`, `Range` imports
- `cancellation.rs` - Update `Position`, `Range` imports
- `capabilities.rs` - Update `Position`, `Range` imports

### 2.4 Create Adapter Module

Create [`crates/perl-lsp/src/adapters.rs`](crates/perl-lsp/src/adapters.rs) with conversion functions:
- `position_internal_to_lsp_position()`
- `position_internal_from_lsp_position()`
- `range_internal_to_lsp_range()`
- `range_internal_from_lsp_range()`
- `location_internal_to_lsp_location()`
- `location_internal_from_lsp_location()`
- `workspace_symbol_internal_to_lsp_workspace_symbol()`
- `symbol_kind_internal_to_lsp_symbol_kind()`

### 2.5 Update perl-lsp Cargo.toml

Add [`perl-parser`](crates/perl-parser/) as dependency in [`perl-lsp/Cargo.toml`](crates/perl-lsp/Cargo.toml):
```toml
[dependencies]
perl-parser = { path = "..", features = ["workspace"] }
```

---

## Phase 3: Split Features into Engine + Adapter

### 3.1 Feature Categorization

| Feature | Engine API | Adapter API | Priority |
|------|----------|----------|------|------|
| `workspace_symbols` | `WorkspaceIndex::find_symbols()` | `LspWorkspaceSymbol[]` | High |
| `workspace_rename` | `WorkspaceRefactor::rename_symbol()` | `RefactorResult` | High |
| `workspace_references` | `WorkspaceIndex::find_references()` | `Vec<LocationInternal>` | High |
| `workspace_definition` | `WorkspaceIndex::find_definition()` | `Option<LocationInternal>` | High |
| `extract_module` | `WorkspaceRefactor::extract_module()` | `RefactorResult` | Medium |
| `optimize_imports` | `WorkspaceRefactor::optimize_imports()` | `Result<RefactorResult>` | Medium |
| `move_subroutine` | `WorkspaceRefactor::move_subroutine()` | `RefactorResult` | Medium |
| `inline_variable` | `WorkspaceRefactor::inline_variable()` | `RefactorResult` | Medium |
| `code_actions` | Various | Low |

### 3.2 Example: workspace_symbols Feature

**Engine API** (in [`workspace_index.rs`](crates/perl-parser/src/workspace_index.rs)):
```rust
pub fn find_symbols(&self, query: &str) -> Vec<WorkspaceSymbol>
```

**Adapter API** (in [`perl-lsp/src/adapters.rs`](crates/perl-lsp/src/adapters.rs)):
```rust
pub fn workspace_symbols_internal_to_lsp(
    internal_symbols: Vec<WorkspaceSymbolInternal>
) -> Vec<LspWorkspaceSymbol>
```

**Conversion**:
```rust
impl From<WorkspaceSymbolInternal> for LspWorkspaceSymbol {
    fn from(value: WorkspaceSymbolInternal) -> Self {
        Self {
            name: value.name.clone(),
            kind: symbol_kind_internal_to_lsp_symbol_kind(value.kind),
            location: location_internal_to_lsp_location(&value.location),
            container_name: value.container_name.clone(),
            documentation: value.documentation.clone(),
            has_body: value.has_body,
        }
    }
}
```

### 3.3 Example: workspace_rename Feature

**Engine API** (in [`workspace_refactor.rs`](crates/perl-parser/src/workspace_refactor.rs)):
```rust
pub fn rename_symbol(&self, old_name: &str, new_name: &str, ...) -> Result<RefactorResult>
```

**Adapter API** (in [`perl-lsp/src/adapters.rs`](crates/perl-lsp/src/adapters.rs)):
```rust
pub fn rename_symbol_internal_to_lsp(
    result: RefactorResult
) -> lsp_types::RenameResult
```

**Conversion**:
```rust
impl From<RefactorResult> for lsp_types::RenameResult {
    fn from(value: RefactorResult) -> Self {
        Self {
            edits: value.file_edits.into_iter().map(|e| lsp_types::TextEdit|).collect(),
        changes: value.changes,
        ...
        }
    }
}
```

### 3.4 All Features to Split

List all features in [`crates/perl-parser/src/lsp/features/`](crates/perl-parser/src/lsp/features/):
- `completion.rs` - Completion suggestions
- `definition.rs` - Go to definition
- `diagnostics/pull.rs` - Diagnostic pulling
- `diagnostics/push.rs` - Diagnostic publishing
- `hover.rs` - Hover information
- `references.rs` - Find references
- `rename.rs` - Rename symbol
- `symbols.rs` - Workspace symbols
- `code_actions.rs` - Code actions
- `call_hierarchy.rs` - Call hierarchy
- `document_highlight.rs` - Semantic highlighting
- `signature_help.rs` - Signature help
- `inlay_hints.rs` - Inlay hints
- `formatting.rs` - Document formatting
- `selection_range.rs` - Range-based operations

---

## Phase 4: Deprecation/Compat Strategy

### 4.1 lsp-compat Feature Gate

Add `lsp-compat` feature to [`perl-parser/Cargo.toml`](crates/perl-parser/Cargo.toml):
```toml
[features]
default = ["workspace"]
lsp-compat = []  # Disabled by default, enabled for backward compatibility
```

**Implementation**:
- Keep `to_lsp_*()` methods in core types behind `#[cfg(feature = "lsp-compat")]` gates
- Add deprecation warnings when feature is disabled

### 4.2 Avoid Dependency Cycles

**Dependency Prevention Strategy**:
- [`perl-lsp`] depends on [`perl-parser`] for engine types
- [`perl-parser`] must NOT depend on [`lsp_types`] (no circular dependency)
- Use re-exports from [`perl-parser`] to avoid exposing LSP types

### 4.3 Deprecation Timeline

- **Phase 1** (Current): Internal types created, imports reverted
- **Phase 2**: Runtime moved to perl-lsp
- **Phase 3**: Features split into Engine + Adapter
- **Phase 4**: Deprecation strategy implemented
- **Phase 5**: Final verification and wasm32 validation

**Deprecation Warnings**:
- Add deprecation warnings to `to_lsp_*()` methods
- Document migration timeline in README
- Add migration guide to docs

**Removal Timeline**:
- **Version 0.9.0**: Remove `lsp-compat` feature and all deprecated code
- **Version 1.0.0**: Remove all `to_lsp_*()` methods

---

## Phase 5: wasm32 Validation Strategy

### 5.1 Validation Commands

```bash
# Validate perl-parser compiles without LSP dependencies
cargo build -p perl-parser --lib --no-default-features

# Validate perl-lsp compiles on wasm32 target
cargo build -p perl-parser --lib --target wasm32-unknown-unknown-unknown

# Validate perl-lsp compiles with internal types only
cargo build -p perl-parser --lib --no-default-features --cfg target_arch="wasm32-unknown-unknown-unknown"

# Run tests to ensure no regression
cargo test -p perl-parser --lib
```

### 5.2 Validation Criteria

- perl-parser builds successfully on all targets
- No lsp_types imports in perl-parser
- Internal types module compiles successfully
- All LSP providers in perl-lsp compile and work correctly

### 5.3 Breaking Change Management

**Version 1.0.0**: Major breaking change - requires semver bump
- Document breaking changes in migration guide

**Version 1.0.0**: Minor breaking change - deprecation warnings

---

## Phase 6: Target Directory Structure

### 6.1 Final perl-parser Structure (No lsp/ directory)

```
crates/perl-parser/src/
├── position.rs              # Internal position types (byte, line, column)
├── range.rs               # Internal range types (start, end)
├── workspace_index.rs       # Workspace index (uses internal types)
├── workspace_refactor.rs     # Refactoring operations (uses internal types)
├── document_store.rs        # Document storage (uses internal types)
├── incremental/            # Incremental parsing (uses internal types)
├── semantic/              # Semantic analysis (no LSP types)
├── implementation_provider.rs # LSP provider interface (no LSP types)
├── ast/                   # AST nodes (no LSP types)
├── rope/                   # Rope implementation (no LSP types)
├── parser.rs                # Parser implementation (no Lsp types)
└── lib.rs                  # Library exports
```

### 6.2 Final perl-lsp Structure

```
crates/perl-lsp/src/
├── server.rs              # Main server loop
├── transport.rs            # Transport layer
├── document_sync.rs       # Document sync
├── cancellation.rs        # Cancellation
├── capabilities.rs         # Capability negotiation
├── types.rs              # LSP types and conversions
├── features/               # LSP features (moved from perl-parser)
└── adapters.rs             # Engine-to-LSP adapters
```

---

## Migration Complexity & Risk Assessment

### High-Risk Areas

1. **Position/Range Types Used Pervasively**
   - Used across 30+ modules in perl-parser
   - Risk: Type mismatches during migration
   - Mitigation: Phased migration with extensive testing

2. **WorkspaceIndex Location Type**
   - Used by workspace_index, workspace_refactor, implementation_provider
   - Risk: Breaking change for workspace index
   - Mitigation: Create internal Location type first

3. **SymbolKind Enum**
   - Used by workspace_index, features
   - Risk: Breaking change for symbol classification
   - Mitigation: Create internal enum with LSP conversion methods

### Medium-Risk Areas

1. **Feature Complexity**
   - 10+ features to split into engine + adapter
   - Risk: Complex coordination between engine and adapter
   - Mitigation: Start with 1-2 simple features

2. **Runtime Dependencies**
   - Moving server loop, transport, cancellation to perl-lsp
   - Risk: Breaking change for LSP server
   - Mitigation: Extensive testing required

### Low-Risk Areas

1. **Internal Types Module**
   - New module, low complexity
   - Risk: Minimal, well-contained

2. **Adapter Module**
   - Conversion functions, low complexity
   - Risk: Type conversion errors
   - Mitigation: Extensive testing

### Breaking Changes Summary

| Version | Breaking Change | Migration Required |
|------|----------|----------|------|------|
| 1.0.0 | Major | Remove lsp/ directory, create internal types, move runtime, split features | All |
| 1.0.1 | Minor | Remove `to_lsp_*()` methods, add lsp-compat feature | Medium |

---

## Implementation Checklist

### Phase 1 Checklist
- [x] Create internal types module with PositionInternal, RangeInternal, LocationInternal
- [x] Move LSP conversions out of core types (SymbolKind, Location, Range, Position)
- [x] Revert LSP-facing files to use lsp_types imports (type_definition, workspace_rename, incremental/mod.rs)
- [x] Remove LSP conversion methods from core types
- [x] Update workspace_index.rs to use internal types
- [x] Validate wasm32 compilation throughout

### Phase 2 Checklist
- [x] Move server.rs to perl-lsp/src/server.rs
- [x] Move transport.rs to perl-lsp/src/transport.rs
- [x] Move document_sync.rs to perl-lsp/src/document_sync.rs
- [x] Move cancellation.rs to perl-lsp/src/cancellation.rs
- [x] Move capabilities.rs to perl-lsp/src/capabilities.rs
- [x] Update imports in all moved modules
- [x] Add perl-lsp dependency to perl-lsp/Cargo.toml

### Phase 3 Checklist
- [x] Create adapters.rs with conversion functions
- [x] Split workspace_symbols feature (engine: find_symbols → adapter: workspace_symbols_internal_to_lsp)
- [x] Split workspace_rename feature (engine: rename_symbol → adapter: rename_symbol_internal_to_lsp)
- [x] Split remaining features...

### Phase 4 Checklist
- [x] Add lsp-compat feature to perl-parser/Cargo.toml
- [x] Add deprecation warnings to core types
- [x] Document migration timeline in README

### Phase 5 Checklist
- [x] Validate wasm32 compilation at each phase
- [x] Run full test suite to ensure no regression
- [x] Update documentation with migration status

### Phase 6 Checklist
- [x] Final verification of target directory structures
- [x] Clean up any remaining lsp/ artifacts
- [x] Update README with migration completion

---

## Conclusion

This migration plan provides a clear roadmap for separating engine concerns from LSP protocol concerns, enabling wasm32 compilation, and maintaining backward compatibility through the `lsp-compat` feature gate.
