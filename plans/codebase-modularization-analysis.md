# Codebase Modularization Analysis and Recommendations

**Date:** 2026-01-18  
**Repository:** perl-lsp / tree-sitter-perl-rs  
**Workspace:** `/home/steven/code/Rust/perl-lsp/review`

---

## Executive Summary

This analysis examines the current crate structure, dependencies, and code organization of the Perl development ecosystem. The workspace contains **17 crates** (15 in workspace + 2 excluded), with significant architectural complexity and opportunities for modularization improvements.

---

## 1. Current Crate Structure Analysis

### 1.1 Workspace Members (15 crates)

| Crate | Purpose | Status | Notes |
|-------|---------|--------|-------|
| `perl-lexer` | Core | Tokenization, Unicode support |
| `perl-parser-core` | Core | Parser engine foundation |
| `perl-semantic-analyzer` | Core | Symbol extraction, type inference |
| `perl-workspace-index` | Core | Workspace indexing, refactoring orchestration |
| `perl-refactoring` | Core | Refactoring and modernization utilities |
| `perl-incremental-parsing` | Core | Incremental parsing support |
| `perl-tdd-support` | Core | Test-driven development helpers |
| `perl-lsp-providers` | Core | LSP provider glue and tooling |
| `perl-lsp-protocol` | Core | JSON-RPC/LSP protocol types |
| `perl-lsp-transport` | Core | LSP transport layer |
| `perl-position-tracking` | Core | UTF-8/UTF-16 position tracking |
| `perl-parser` | Main | Parser + LSP providers + TDD + workspace + refactoring + incremental |
| `perl-lsp` | Binary | LSP server binary |
| `perl-dap` | Binary | Debug Adapter Protocol server |
| `perl-corpus` | Support | Test corpus management |
| `xtask` | Tools | Advanced testing tools (excluded from workspace) |
| `tree-sitter-perl` | Legacy | Tree-sitter integration (excluded) |
| `tree-sitter-perl-c` | Legacy | C bindings (excluded) |
| `tree-sitter-perl-rs` | Legacy | Rust wrapper (excluded) |

### 1.2 Dependency Graph Overview

```
┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                          perl-lexer                                 │
│                            │         │
│                            │         │
│                            ▼         │
│                            │         │
│                            │         │
│               ┌────────────────────────────────────────┐         │
│               │                                │         │
│               │   perl-parser-core             │         │
│               │   (AST, Parser, Position)        │         │
│               │                                │         │
│               └────────────────────────────────────────┘         │
│                            │         │
│                            │         │
│   ┌──────────────────────────────────────────────┐         │
│   │                                │         │
│   │   perl-semantic-analyzer           │         │
│   │   perl-workspace-index             │         │
│   │   perl-refactoring                 │         │
│   │   perl-incremental-parsing         │         │
│   │   perl-tdd-support                 │         │
│   │   perl-lsp-providers               │         │
│   │   perl-lsp-protocol               │         │
│   │   perl-lsp-transport               │         │
│   │   perl-position-tracking             │         │
│   └──────────────────────────────────────────────┘         │
│                            │         │
│                            │         │
│                            ▼         │
│                            │         │
│                            │         │
│               ┌────────────────────────────────────────┐         │
│               │   perl-parser (MAIN CRATE)    │         │
│               │   (aggregates all above)     │         │
│               └────────────────────────────────────────┘         │
│                            │         │
│                            │         │
│                            ▼         │
│                            │         │
│                            │         │
│               ┌────────────────────────────────────────┐         │
│               │   perl-lsp (BINARY)          │         │
│               │   perl-dap (BINARY)          │         │
│               └────────────────────────────────────────┘         │
└────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Code Organization Analysis

### 2.1 perl-parser (Main Crate) - Module Structure

**File:** [`crates/perl-parser/src/lib.rs`](crates/perl-parser/src/lib.rs:1)

**Modules:**
- `engine` - Parser engine components
- `ide` - IDE integration helpers (LSP/DAP runtime support)
- `analysis` - Analysis modules (dead code detection, declaration, index, scope_analyzer, semantic, symbol, type_inference)
- `builtins` - Builtin function parsing
- `compat` - Compatibility module for tests
- `incremental` - Incremental parsing (feature-gated)
- `refactor` - Refactoring and modernization
- `tdd` - Test-driven development support
- `tokens` - Token stream and trivia
- `tooling` - Tooling integrations
- `workspace` - Workspace indexing and refactoring

**Observations:**
- **637 lines** in lib.rs - Very large module file
- **Extensive re-exports** - Lines 106-289 re-export types from submodules
- **LSP feature exports commented out** - Lines 299-322 show deprecated LSP exports (migrated to perl-lsp)
- **Mixed responsibilities** - Parser engine, LSP providers, TDD, refactoring, workspace all in one crate

### 2.2 perl-lsp (Binary Crate) - Module Structure

**File:** [`crates/perl-lsp/src/lib.rs`](crates/perl-lsp/src/lib.rs:1)

**Modules:**
- `protocol` - JSON-RPC message types
- `transport` - Message framing and transport layer
- `state` - Document and server state management
- `runtime` - Core server implementation and lifecycle management
- `features` - LSP feature providers
- `convert` - Conversions between engine types and lsp_types
- `util` - URI handling, UTF-16 conversion
- `fallback` - Text-based fallback implementations
- `handlers` - LSP request/notification handlers
- `dispatch` - Request routing and dispatch logic
- `server` - Public server interface

**Observations:**
- **258 lines** - Large but more focused than perl-parser
- **Internal re-exports** - Lines 77-215 re-export parser types for compatibility
- **Clean separation** - Better organized than perl-parser

### 2.3 perl-lsp-providers - Module Structure

**File:** [`crates/perl-lsp-providers/src/lib.rs`](crates/perl-lsp-providers/src/lib.rs:1)

**Modules:**
- `ide` - IDE integration helpers
- `tooling` - Tooling integrations and performance helpers

**Observations:**
- **49 lines** - Very small, focused crate
- **Shim layer** - Provides compatibility shims between perl-parser and perl-lsp

### 2.4 perl-semantic-analyzer - Module Structure

**File:** [`crates/perl-semantic-analyzer/src/lib.rs`](crates/perl-semantic-analyzer/src/lib.rs:1)

**Modules:**
- `analysis` - Semantic analysis, symbol extraction, type inference

**Observations:**
- **60 lines** - Focused, single-responsibility crate
- **Clean design** - Well-organized semantic analysis

---

## 3. Identified Issues and Concerns

### 3.1 Structural Issues

#### Issue 1: perl-parser is a Monolithic "God Crate"

**Description:** The [`perl-parser`](crates/perl-parser/Cargo.toml:1) crate aggregates too many responsibilities:

- Parser engine (engine, parser, position, ast, edit, parser_context, pragma_tracker, quote_parser, util)
- LSP providers (diagnostics, code_actions, completion, document_highlight, document_links, folding, formatting, implementation_provider, inlay_hints, inline_completions, on_type_formatting, references, rename, selection_range, semantic_tokens, signature_help, type_definition, type_hierarchy, workspace_symbols)
- IDE runtime (cancellation, call_hierarchy_provider, execute_command, diagnostics_catalog)
- Analysis modules (dead_code_detector, declaration, index, scope_analyzer, semantic, symbol, type_inference)
- Refactoring (import_optimizer, modernize, modernize_refactored, refactoring)
- TDD (tdd_basic, tdd_workflow, test_generator, test_runner)
- Workspace (document_store, workspace_index, workspace_refactor, workspace_rename)
- Tooling (performance, perl_critic, perltidy)
- Tokens (token_stream, token_wrapper, trivia, trivia_parser)

**Impact:**
- **Circular dependencies:** Multiple crates depend on perl-parser, which in turn depends on them
- **Testing complexity:** Changes to perl-parser require re-testing all dependent crates
- **Maintenance burden:** Any change affects the entire ecosystem
- **Compilation time:** Large crate increases build times

#### Issue 2: LSP Feature Migration Incomplete

**Description:** Lines 299-322 in [`perl-parser/src/lib.rs`](crates/perl-parser/src/lib.rs:299) show LSP feature exports that are commented out with deprecation notice:

```rust
// pub use code_actions::{CodeAction, CodeActionEdit, CodeActionKind, CodeActionsProvider};
// pub use code_actions_enhanced::{EnhancedCodeActionsProvider};
// ... (many more commented LSP exports)
```

**Impact:**
- **Unclear migration status:** Code is commented but not yet removed
- **API surface confusion:** Users may reference types that appear to exist but are deprecated
- **Maintenance burden:** Dead code increases cognitive load

#### Issue 3: perl-lsp Contains Internal Re-exports

**Description:** Lines 77-215 in [`crates/perl-lsp/src/lib.rs`](crates/perl-lsp/src/lib.rs:77) contain internal compatibility re-exports:

```rust
/// Parser re-export for migrated code
pub(crate) use perl_parser::Parser;

/// Position utilities re-export
pub(crate) mod position {
    pub use perl_parser::position::*;
}
```

**Impact:**
- **Namespace pollution:** Internal modules re-exported at crate root
- **API surface confusion:** Mix of public and private APIs
- **Documentation burden:** Internal types appear in public documentation

#### Issue 4: Excluded Crates Create Confusion

**Description:** The workspace excludes 4 crates:
- `tree-sitter-perl` - Tree-sitter grammar
- `tree-sitter-perl-c` - C bindings
- `tree-sitter-perl-rs` - Rust wrapper
- `archive` - Archived legacy components

**Impact:**
- **Discovery difficulty:** New developers may not know these exist
- **Documentation gaps:** Excluded crates may not be documented
- **Inconsistent practices:** Mixed workspace policies

#### Issue 5: perl-position-tracking Edition Mismatch

**Description:** [`perl-position-tracking`](crates/perl-position-tracking/Cargo.toml:4) uses Rust 2021 edition while workspace uses Rust 2024.

**Impact:**
- **Edition inconsistency:** May cause compilation issues
- **Feature compatibility:** Different editions have different feature support

#### Issue 6: Feature Proliferation

**Description:** Many crates have numerous feature flags:
- `perl-parser`: ~20 features (cli, incremental, lsp-compat, lsp-ga-lock, test-compat, test-performance, lsp-advanced, expose_lsp_test_api, workspace_refactor, modernize, substitution-advanced, utf16-complete, semantic-phase2, parser-extras, crash-repros)
- `perl-lsp`: ~15 features (workspace, incremental, test-compat, test-performance, lsp-advanced, stress-tests, lsp-extras, strict-jsonrpc, dap-phase1, dap-phase3, utf16-complete)
- `perl-dap`: ~3 features (dap-phase1, dap-phase2, dap-phase3)

**Impact:**
- **Configuration complexity:** Hard to understand which features enable what
- **Testing burden:** Many feature combinations need testing
- **Documentation gaps:** Feature interactions may not be documented

#### Issue 7: Inconsistent Dependency Patterns

**Description:** Different crates use different patterns for accessing shared functionality:

**Example:** perl-lsp uses `pub(crate) use perl_parser::*` while perl-semantic-analyzer uses `pub use perl_parser_core::*`

**Impact:**
- **Cognitive load:** Developers must remember which pattern to use
- **Refactoring difficulty:** Changes require updating multiple files

---

## 4. Dependency Analysis

### 4.1 perl-parser Dependencies

**From Cargo.toml:**
```toml
[dependencies]
perl-lexer = { path = "../perl-lexer" }
perl-parser-core = { path = "../perl-parser-core" }
perl-semantic-analyzer = { path = "../perl-semantic-analyzer" }
perl-workspace-index = { path = "../perl-workspace-index" }
perl-refactoring = { path = "../perl-refactoring" }
perl-incremental-parsing = { path = "../perl-incremental-parsing", optional = true }
perl-tdd-support = { path = "../perl-tdd-support" }
perl-lsp-providers = { path = "../perl-lsp-providers", default-features = false }
perl-dap = { path = "../perl-dap" }
thiserror = "2.0.17"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
regex = "1.12.2"
lazy_static = "1.5.0"
lsp-types = { version = "0.97.0", optional = true }
tracing = "0.1.44"
url = "2.5.8"
rustc-hash = "2.1.1"
md5 = "0.8.0"
phf = { version = "0.13.1", features = ["macros"] }
parking_lot = "0.12.5"
ropey = "1.6.1"
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
walkdir = "2.5.0"
[target.'cfg(unix)'.dependencies]
nix = { version = "0.30.1", features = ["signal"] }
[dev-dependencies]
# ... (many more)
```

**Observations:**
- **Direct dependencies:** 8 workspace crates + 8 external crates
- **Optional dependencies:** 2 (lsp-types, incremental-parsing)
- **Dev dependencies:** ~15 crates for testing
- **High coupling:** Every crate in workspace depends on perl-parser

### 4.2 Circular Dependency Risk

**Potential circular dependencies identified:**

1. **perl-parser** → **perl-lsp** → **perl-parser** (via re-exports)
2. **perl-parser** → **perl-lsp-providers** → **perl-parser** (via re-exports)
3. **perl-parser** → **perl-semantic-analyzer** → **perl-parser-core** → **perl-lexer**
4. **perl-lsp** → **perl-lsp-providers** → **perl-semantic-analyzer** → **perl-workspace-index** → **perl-position-tracking**

**Analysis:**
- **No actual circular dependencies:** Cargo resolves dependencies correctly
- **Architectural concern:** Tight coupling through re-exports creates implicit dependencies
- **Risk:** Changes to core types require updating multiple crates

---

## 5. Modularization Recommendations

### 5.1 Recommendation: Split perl-parser into Smaller, Focused Crates

**Rationale:** The [`perl-parser`](crates/perl-parser/Cargo.toml:1) crate is too monolithic. It combines parser engine, LSP providers, TDD support, workspace indexing, refactoring, and tooling.

**Proposed Structure:**

```
perl-parser-core
├── parser/           - Core parsing logic
├── ast/               - AST definitions
├── position/           - Position tracking
└── engine/            - Parser orchestration

perl-parser-ast
├── ast/               - AST node definitions
└── ast_v2/           - Experimental AST (if needed)

perl-parser-parser
├── parser.rs          - Main Parser struct and implementation
├── parser_context.rs  - Error recovery and context
├── pragma_tracker.rs   - Pragma handling
└── quote_parser.rs     - Quote-like operators

perl-parser-workspace
├── workspace_index.rs  - Core workspace indexing
├── document_store.rs   - Document storage
├── workspace_refactor.rs - Workspace refactoring
└── workspace_rename.rs    - Workspace-aware rename

perl-parser-refactoring
├── import_optimizer.rs   - Import analysis
├── modernize.rs         - Code modernization
└── refactoring.rs       - Refactoring engine

perl-parser-tdd
├── test_generator.rs    - Test generation
├── test_runner.rs       - Test execution
└── tdd_workflow.rs       - TDD workflow

perl-parser-ide
├── diagnostics_catalog.rs  - Error catalog
├── cancellation.rs        - Cancellation infrastructure
└── execute_command.rs    - External tool integration

perl-parser-tooling
├── performance.rs       - Performance helpers
├── perl_critic.rs       - Perl::Critic integration
└── perltidy.rs          - Perltidy integration
```

**Benefits:**
- **Reduced compilation time:** Smaller crates compile faster
- **Clearer boundaries:** Each crate has a single, well-defined responsibility
- **Easier testing:** Changes to one crate don't require re-testing everything
- **Better documentation:** Smaller crates are easier to document comprehensively
- **Independent evolution:** Different crates can evolve at different rates
- **Selective dependencies:** Crates only depend on what they need

**Migration Strategy:**
1. Create new crates with clear responsibilities
2. Move code to appropriate crates
3. Update dependencies
4. Update re-exports in perl-lsp and perl-lsp-providers
5. Update documentation

### 5.2 Recommendation: Create a Shared Types Crate

**Rationale:** Multiple crates need access to common types (AST nodes, positions, errors) but use different import paths.

**Proposed Crate:** `perl-common-types`

**Contents:**
```rust
pub use crate::ast::{Node, NodeKind, SourceLocation};
pub use crate::position::{LineColumn, LineIndex, Position};
pub use crate::error::{ParseError, ParseResult};
pub use crate::parser::{Parser, ParserContext};
pub use crate::symbol::{Symbol, SymbolKind, SymbolReference, SymbolTable};
pub use crate::pragma_tracker::{PragmaState, PragmaTracker};
pub use crate::token_stream::{Token, TokenKind, TokenStream};
pub use crate::trivia::{NodeWithTrivia, Trivia, TriviaToken};
```

**Benefits:**
- **Single source of truth:** Common types defined once
- **Consistent imports:** All crates use `use perl_common_types::*`
- **Reduced compilation:** No need to re-export through multiple layers
- **Better documentation:** Types documented in one place

### 5.3 Recommendation: Consolidate LSP Infrastructure

**Rationale:** LSP functionality is split across multiple crates with inconsistent patterns.

**Proposed Structure:**

```
perl-lsp-protocol
├── protocol.rs        - JSON-RPC types
├── lsp_types.rs       - LSP types re-exports
└── capabilities.rs     - LSP capability configuration

perl-lsp-transport
├── framing.rs          - Message framing
└── stdio.rs            - stdio transport

perl-lsp-server
├── server.rs           - Core server implementation
├── handlers.rs         - Request handlers
├── dispatch.rs          - Request routing
└── state.rs            - State management

perl-lsp-features
├── completion.rs       - Completion provider
├── diagnostics.rs      - Diagnostics provider
├── document_links.rs   - Document links
├── folding.rs          - Text folding
├── formatting.rs       - Code formatting
├── hover.rs           - Hover provider
├── inlay_hints.rs     - Inlay hints
├── references.rs       - Find references
├── rename.rs          - Rename provider
├── semantic_tokens.rs  - Semantic tokens
├── signature_help.rs   - Signature help
├── type_definition.rs  - Go to definition
├── type_hierarchy.rs   - Type hierarchy
└── workspace_symbols.rs - Workspace symbols
```

**Benefits:**
- **Clear ownership:** Each crate owns its LSP feature
- **Consistent patterns:** Similar structure across all features
- **Easier testing:** Features can be tested in isolation
- **Better documentation:** Each crate has focused scope

### 5.4 Recommendation: Extract Incremental Parsing to Separate Crate

**Rationale:** Incremental parsing is currently feature-gated within perl-parser but has significant code volume.

**Proposed Structure:**

```
perl-incremental-parsing
├── incremental_state.rs     - Incremental state management
├── incremental_document.rs   - Document with incremental support
├── incremental_edit.rs      - Edit tracking
├── incremental_handler.rs  - Incremental handlers
├── incremental_simple.rs   - Simple incremental parsing
└── incremental_v2.rs         - Experimental incremental parsing
```

**Benefits:**
- **Optional by default:** Users can choose whether to use it
- **Clearer API:** Dedicated crate for incremental parsing
- **Reduced perl-parser size:** Removes ~5,000 lines from main crate

### 5.5 Recommendation: Remove Dead Code and Complete LSP Migration

**Rationale:** The commented-out LSP exports in [`perl-parser/src/lib.rs`](crates/perl-parser/src/lib.rs:299) create confusion and maintenance burden.

**Action Items:**
1. Remove commented-out LSP exports (lines 299-322)
2. Remove internal re-exports from [`perl-lsp/src/lib.rs`](crates/perl-lsp/src/lib.rs:77)
3. Update [`perl-lsp-providers`](crates/perl-lsp-providers/src/lib.rs:1) to provide complete LSP implementations
4. Update all documentation to reflect new architecture
5. Run full test suite to ensure no regressions

### 5.6 Recommendation: Standardize Feature Flags

**Rationale:** Feature flags are inconsistent across crates and poorly documented.

**Proposed Standard:**

1. **Group features by domain:**
   - `parser-core`: Core parsing features
   - `lsp`: LSP server features
   - `dap`: Debug adapter features
   - `incremental`: Incremental parsing
   - `tdd`: Test-driven development

2. **Document feature interactions:** Create a features matrix showing which features enable what

3. **Reduce feature count:** Consolidate related features

4. **Use feature combinations:** Replace individual boolean features with feature groups

**Example:**
```toml
[features]
default = ["parser-core", "lsp-basic"]
lsp-basic = ["completion", "diagnostics", "hover"]
lsp-advanced = ["lsp-basic", "references", "rename"]
```

### 5.7 Recommendation: Fix Edition Inconsistency

**Rationale:** [`perl-position-tracking`](crates/perl-position-tracking/Cargo.toml:4) uses Rust 2021 while workspace uses Rust 2024.

**Action:** Update [`perl-position-tracking/Cargo.toml`](crates/perl-position-tracking/Cargo.toml:4) to use `edition.workspace = true`

### 5.8 Recommendation: Establish Clear Dependency Direction

**Rationale:** The dependency graph shows many crates depend on perl-parser, creating a hub-and-spoke pattern.

**Proposed Principle:**
- **Core crates should not depend on binary crates** (perl-lsp, perl-dap)
- **Binary crates should depend on core crates**
- **Provider crates should be thin layers** (perl-lsp-providers, perl-lsp-protocol, perl-lsp-transport)
- **No circular dependencies** through re-exports

**Dependency Rules:**
1. `perl-lexer` - Zero dependencies (leaf crate)
2. `perl-parser-core` - Only depends on `perl-lexer`
3. `perl-semantic-analyzer` - Only depends on `perl-parser-core` and `perl-workspace-index`
4. `perl-workspace-index` - Only depends on `perl-parser-core` and `perl-position-tracking`
5. `perl-refactoring` - Only depends on `perl-parser-core` and `perl-workspace-index`
6. `perl-lsp-providers` - Only depends on core crates
7. `perl-lsp-protocol` - Zero dependencies
8. `perl-lsp-transport` - Only depends on `perl-lsp-protocol`
9. `perl-lsp` - Only depends on provider crates
10. `perl-dap` - Only depends on `lsp-types` (external)

---

## 6. Proposed New Crate Structure

### 6.1 Core Layer (Foundation Crates)

```
perl-lexer
├── Zero dependencies
└── Single responsibility: Tokenization

perl-parser-core
├── Depends on: perl-lexer
├── Contains: AST, Position, Parser
└── Single responsibility: Parser engine foundation

perl-common-types (NEW)
├── Zero dependencies
├── Contains: Shared type definitions
└── Single responsibility: Type consistency

perl-position-tracking
├── Depends on: Zero dependencies (should be independent)
├── Contains: UTF-8/UTF-16 conversion
└── Single responsibility: Position tracking

perl-workspace-index
├── Depends on: perl-parser-core, perl-position-tracking
├── Contains: Workspace indexing logic
└── Single responsibility: Cross-file analysis

perl-semantic-analyzer
├── Depends on: perl-parser-core, perl-workspace-index
├── Contains: Symbol extraction, type inference
└── Single responsibility: Semantic analysis

perl-refactoring
├── Depends on: perl-parser-core, perl-workspace-index
├── Contains: Refactoring utilities
└── Single responsibility: Code transformation

perl-incremental-parsing
├── Depends on: perl-parser-core
├── Optional by default
└── Single responsibility: Incremental parsing

perl-tdd-support
├── Depends on: perl-parser-core
├── Contains: Test generation utilities
└── Single responsibility: TDD workflow

```

### 6.2 Provider Layer (LSP Infrastructure)

```
perl-lsp-protocol
├── Zero dependencies
├── Contains: JSON-RPC types
└── Single responsibility: Protocol definitions

perl-lsp-transport
├── Depends on: perl-lsp-protocol
├── Contains: Message framing, stdio transport
└── Single responsibility: Transport layer

perl-lsp-providers
├── Depends on: core crates
├── Contains: LSP provider implementations
└── Single responsibility: Feature glue layer

```

### 6.3 Application Layer (Binary Crates)

```
perl-lsp
├── Depends on: provider layer
├── Contains: Server implementation
└── Single responsibility: LSP server binary

perl-dap
├── Depends on: lsp-types (external)
├── Contains: DAP server implementation
└── Single responsibility: Debug adapter binary
```

### 6.4 Support Layer

```
perl-corpus
├── Test utilities and fixtures
└── Single responsibility: Test infrastructure

xtask
├── Development tools (excluded from workspace)
└── Single responsibility: Build/test automation
```

---

## 7. Migration Strategy

### Phase 1: Foundation (High Priority)
1. Create `perl-common-types` crate
2. Update all crates to use common types
3. Fix `perl-position-tracking` edition

### Phase 2: LSP Infrastructure (High Priority)
1. Remove dead LSP exports from `perl-parser`
2. Remove internal re-exports from `perl-lsp`
3. Consolidate LSP providers into dedicated crates
4. Update `perl-lsp-providers` to provide complete implementations

### Phase 3: Parser Modularization (Medium Priority)
1. Create `perl-parser-core` crate with parser engine
2. Create `perl-parser-ast` crate with AST definitions
3. Create `perl-parser-parser` crate with parsing logic
4. Create `perl-parser-workspace` crate for workspace features
5. Create `perl-parser-refactoring` crate for refactoring
6. Create `perl-parser-tdd` crate for TDD support
7. Create `perl-parser-ide` crate for IDE helpers
8. Create `perl-parser-tooling` crate for tooling
9. Update `perl-parser` to re-export from new crates
10. Migrate code to new crates

### Phase 4: Incremental Parsing (Low Priority)
1. Extract incremental parsing to `perl-incremental-parsing` crate
2. Update `perl-parser` to use new crate
3. Update feature flags

### Phase 5: Cleanup (Low Priority)
1. Remove `perl-parser-pest` (legacy)
2. Remove `perl-incremental-parsing` from `perl-parser` (after extraction)
3. Clean up excluded crates
4. Update documentation

---

## 8. Risk Assessment

### 8.1 High-Risk Items

| Risk | Impact | Mitigation |
|------|---------|------------|
| Breaking changes to perl-parser | Affects entire ecosystem | Incremental migration with feature flags |
| Circular dependencies through re-exports | Difficult to reason about | Clear dependency direction policy |
| Feature flag complexity | Hard to test and understand | Standardize feature groups |
| Dead code in perl-parser | Maintenance burden | Remove before modularization |

### 8.2 Medium-Risk Items

| Risk | Impact | Mitigation |
|------|---------|------------|
| Large-scale refactoring | Many files to move | Create migration scripts |
| Documentation updates | Significant changes required | Update all affected crates |
| Test suite updates | Need comprehensive re-testing | Prioritize test stability |

### 8.3 Low-Risk Items

| Risk | Impact | Mitigation |
|------|---------|-----------|
| Edition mismatch | Potential compilation issues | Fix perl-position-tracking |
| Excluded crate confusion | Document exclusion policy clearly | Update README |

---

## 9. Success Criteria

A successful modularization would achieve:

1. **Clear separation of concerns:** Each crate has a single, well-defined responsibility
2. **Minimal dependencies:** Crates only depend on what they need
3. **No circular dependencies:** Clear dependency graph without cycles
4. **Consistent patterns:** Similar structure across related crates
5. **Reduced compilation time:** Smaller crates compile faster
6. **Better testability:** Changes are isolated to specific crates
7. **Clear documentation:** Each crate can be documented independently
8. **Maintain backward compatibility:** Existing APIs continue to work during migration
9. **Feature flag clarity:** Easy to understand which features enable what

---

## 10. Conclusion

The current codebase shows signs of organic growth with good architectural patterns, but has accumulated some technical debt through rapid feature development. The proposed modularization would:

1. **Reduce cognitive load:** Smaller, focused crates are easier to understand
2. **Improve maintainability:** Clear boundaries reduce coupling
3. **Enable faster development:** Smaller crates compile faster and can be worked on independently
4. **Enhance testability:** Isolated changes require less re-testing
5. **Support better documentation:** Focused crates are easier to document comprehensively

**Recommendation:** Proceed with a phased approach, starting with low-risk items (creating `perl-common-types`, fixing edition mismatch) and working toward the high-priority modularization of the LSP infrastructure.
