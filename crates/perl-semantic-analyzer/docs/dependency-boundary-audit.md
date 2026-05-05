# Dependency boundary audit: `perl-semantic-analyzer` → `perl-workspace`

Date: 2026-05-05

## Goal

Document concrete coupling points so follow-up PRs can remove analyzer → workspace dependencies while preserving behavior.

## Current dependency shape

`crates/perl-semantic-analyzer/Cargo.toml` currently depends on `perl-workspace`.

## Direct usages found

### 1) Public re-export (API coupling)

- `src/lib.rs` re-exports `perl_workspace::workspace_index`.
- Impact: downstream crates can import workspace index types through the analyzer crate, creating a reverse-layer convenience dependency.
- Classification: **accidental convenience**.

### 2) Declaration-provider key types (shared vocabulary)

- `src/analysis/declaration.rs` imports `SymKind` and `SymbolKey` from `workspace_index`.
- Impact: analyzer-side declaration lookup returns workspace-defined symbol identifiers.
- Classification: **shared vocabulary (currently hosted in workspace)**.

### 3) Semantic query facade (`WorkspaceIndex`) references (query/store)

- `src/analysis/semantic/query_facade.rs` accepts `workspace_index::WorkspaceIndex`.
- Impact: semantic facade APIs in analyzer crate require workspace storage/query types.
- Classification: **query/store coupling**.

## Recommended migration slices

1. **Vocabulary extraction (first):** move `SymKind` and `SymbolKey` into a neutral crate (`perl-symbol-types` or `perl-semantic-facts`) and update both crates to consume it.
2. **API decoupling (second):** remove `workspace_index` re-export from analyzer public API after downstream call sites switch to explicit workspace imports.
3. **Facade relocation (third):** move `analysis/semantic/query_facade.rs` workspace-facing query APIs into `perl-workspace` (or keep analyzer-only facts facade and add workspace adapters there).

## Safety checks for follow-up PRs

- `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`
- `./scripts/cargo-safe check --all-targets -p perl-semantic-analyzer --profile agent --locked`
- `./scripts/cargo-safe clippy -p perl-semantic-analyzer --profile agent --locked -- -D warnings -A missing_docs`
- `./scripts/cargo-safe xtask fmt`

## Non-goals in this audit PR

- No behavior changes.
- No schema or scorecard changes.
- No provider behavior changes.
