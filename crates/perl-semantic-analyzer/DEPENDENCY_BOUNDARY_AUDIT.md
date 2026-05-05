# Dependency Boundary Audit (PR1)

This document records the current `perl-semantic-analyzer` coupling to
`perl-workspace` and classifies each touchpoint to guide follow-up
decoupling work.

## Goal

Keep `perl-semantic-analyzer` as a producer of semantic facts and keep
workspace query/storage policy in `perl-workspace` / `perl-workspace-index`.

## Current direct workspace dependencies

### 1) Public re-export of workspace index module

- Location: `src/lib.rs`
- Usage: `pub use perl_workspace::workspace_index;`
- Classification: **query/store coupling**
- Why: re-export exposes workspace index surface as part of analyzer API,
  blurring crate responsibilities.
- Follow-up direction: remove this re-export and provide a compatibility
  migration note in `CHANGELOG` / release notes.

### 2) Declaration provider symbol key types

- Location: `src/analysis/declaration.rs`
- Usage: `use crate::workspace_index::{SymKind, SymbolKey};`
- Classification: **shared vocabulary (currently misplaced)**
- Why: analyzer emits declaration targets keyed by symbol identity,
  but the key type currently lives in workspace index implementation.
- Follow-up direction: move `SymKind` + `SymbolKey` to a neutral crate
  (`perl-symbol-types` or `perl-semantic-facts`) and consume from both
  analyzer and workspace layers.

### 3) Semantic query facade tests use workspace index container

- Location: `src/analysis/semantic/query_facade.rs`
- Usage: creates `workspace_index::WorkspaceIndex` in tests.
- Classification: **accidental convenience**
- Why: tests validate semantic query behavior by wiring concrete workspace
  storage directly.
- Follow-up direction: replace with fixture-backed fact/query harness
  that asserts analyzer outputs without requiring workspace container types.

## Proposed migration sequence

1. Introduce neutral `SymbolKey` / `SymKind` definitions outside
   `perl-workspace` and dual-implement conversions.
2. Update analyzer declaration APIs to use neutral types.
3. Remove `workspace_index` re-export from analyzer crate root.
4. Port query facade tests to analyzer-owned harness.
5. Drop `perl-workspace` from analyzer `Cargo.toml` dependencies.

## Non-goals in this PR

- No behavioral change in semantic outputs.
- No provider-facing query policy changes.
- No workspace index invalidation/storage changes.
