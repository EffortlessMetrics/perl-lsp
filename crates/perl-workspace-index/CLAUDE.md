# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

- **Crate**: `perl-workspace-index`
- **Version**: 0.9.0
- **Tier**: 3 (two-level internal dependencies)
- **Purpose**: Central workspace indexing engine providing cross-file symbol lookup, document management, lifecycle state machine, bounded caching, and SLO monitoring for the Perl LSP server.

## Commands

```bash
cargo build -p perl-workspace-index              # Build
cargo test -p perl-workspace-index               # Run tests
cargo clippy -p perl-workspace-index             # Lint
cargo doc -p perl-workspace-index --open         # View docs
cargo bench -p perl-workspace-index --features workspace  # Benchmarks
```

## Architecture

### Dependencies

- `perl-parser-core` -- core parsing infrastructure (re-exports `Parser`, `Node`, `NodeKind`, `SourceLocation`, `line_index`)
- `perl-position-tracking` -- position/range types with `lsp-compat` feature
- `perl-symbol-types` -- symbol taxonomy
- `perl-uri` -- URI normalization and filesystem path conversion

### Features

| Feature | Effect |
|---------|--------|
| `workspace` | Enables workspace benchmarks |
| `lsp-compat` | Adds optional `lsp-types` dependency for LSP wire types |

### Source Layout

| File | Purpose |
|------|---------|
| `src/lib.rs` | Crate root; re-exports parser core types and workspace modules |
| `src/workspace/mod.rs` | Module root; re-exports key types from submodules |
| `src/workspace/workspace_index.rs` | Core `WorkspaceIndex` with dual indexing, `IndexState`, `IndexPhase`, resource limits, early-exit handling |
| `src/workspace/document_store.rs` | Thread-safe `DocumentStore` and `Document` with URI normalization |
| `src/workspace/state_machine.rs` | Enhanced `IndexStateMachine` with 8 states (Idle, Initializing, Building, Updating, Invalidating, Ready, Degraded, Error) and guarded transitions |
| `src/workspace/cache.rs` | `BoundedLruCache<K,V>` with LRU eviction, TTL, `EstimateSize` trait, and typed cache configs |
| `src/workspace/production_coordinator.rs` | `ProductionIndexCoordinator` integrating state machine, caches, SLO tracker |
| `src/workspace/slo.rs` | `SloTracker` with per-operation latency percentiles and SLO compliance checks |
| `src/workspace/workspace_rename.rs` | Deprecated stub (renamed to `perl-lsp` crate) |

### Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `WorkspaceIndex` | `workspace_index` | Central symbol index with dual qualified/bare name lookup |
| `Location` | `workspace_index` | URI + Range for symbol locations |
| `IndexResourceLimits` | `workspace_index` | Configurable file/symbol/time limits |
| `IndexPhase` / `IndexState` | `workspace_index` | Build lifecycle (Idle, Scanning, Indexing) |
| `DocumentStore` | `document_store` | Thread-safe document cache with version tracking |
| `Document` | `document_store` | Single document with URI, version, text, line index |
| `IndexStateMachine` | `state_machine` | Production state machine with 8 states and transition guards |
| `IndexStateKind` | `state_machine` | Coarse state enum for instrumentation |
| `BoundedLruCache<K,V>` | `cache` | Generic bounded LRU cache |
| `CacheConfig` | `cache` | Max items, max bytes, optional TTL |
| `EstimateSize` | `cache` | Trait for memory size estimation |
| `ProductionIndexCoordinator` | `production_coordinator` | Integrates index + caches + SLOs |
| `WorkspaceCacheManager` | `production_coordinator` | Manages AST, symbol, and workspace caches |
| `SloTracker` | `slo` | Per-operation latency and error-rate tracking |
| `OperationType` | `slo` | Enum of tracked operations (8 variants) |

## Usage

```rust
use perl_workspace_index::workspace::workspace_index::WorkspaceIndex;
use url::Url;

let index = WorkspaceIndex::new();
let uri = Url::parse("file:///lib/MyModule.pm")?;
index.index_file(uri, source_code)?;

// Symbol lookup
let def = index.find_definition("MyModule::helper");
let refs = index.find_references("helper");
let syms = index.find_symbols("helper");
```

### Document Store

```rust
use perl_workspace_index::workspace::document_store::DocumentStore;

let store = DocumentStore::new();
store.open("file:///lib/Foo.pm".into(), 1, source.into());
let doc = store.get("file:///lib/Foo.pm");
store.update("file:///lib/Foo.pm", 2, new_source.into());
store.close("file:///lib/Foo.pm");
```

### Production Coordinator

```rust
use perl_workspace_index::workspace::production_coordinator::ProductionIndexCoordinator;

let coordinator = ProductionIndexCoordinator::new();
coordinator.initialize()?;
coordinator.index_file(uri, text)?;
let def = coordinator.find_definition("my_sub");
let stats = coordinator.statistics();
```

## Important Notes

- **Dual indexing pattern** (PR #122): symbols are indexed under both `Package::name` and `name` for comprehensive cross-file resolution.
- `workspace_index.rs` is the largest source file in the workspace; changes require careful review.
- All public types from submodules are re-exported via `workspace::mod.rs`.
- `workspace_rename.rs` is a deprecated stub; rename logic has moved to `perl-lsp`.
- Thread safety: `WorkspaceIndex` uses `parking_lot::RwLock`/`Mutex`; `DocumentStore` uses `std::sync::RwLock`.
- The `workspace` feature flag gates only the benchmark binary, not runtime functionality.
