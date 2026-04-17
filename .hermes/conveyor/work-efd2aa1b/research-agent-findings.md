# Research Findings — work-efd2aa1b

## Issue Summary
Collapse 11 `perl-dap-*` satellite crates (6.6K LOC source, 14.5K LOC tests, 28 test files) into `perl-dap::components` module hierarchy as part of Wave H of the microcrate-collapse initiative (#4410). External consumers (`perl-lsp`, `perl-lsp-config`) only depend on `perl-dap-platform`, which will be re-exported as `perl_dap::components::launch::platform` after migration.

## Relevant Codebase Areas
- `/crates/perl-dap/` — Main DAP server (owner crate, stays intact)
- `/crates/perl-dap-*/` — 11 satellite crates to absorb
- `perl-lsp` and `perl-lsp-config` — External consumers of `perl-dap-platform`
- `.spec/microcrate-collapse/ledger.md` lines 219-233 — Wave H tracking

## Key Findings
1. **Clean DAG**: Inter-satellite dependencies form a cycle-free DAG (platform → command-args; shell → platform, command-args; variables → value). No build.rs files in any satellite.
2. **Proposed layout** follows dependency order: `types` (foundation) → `variables` → `launch` → `security`
3. **Platform code**: `perl-dap-platform` has `cfg(unix)`/`cfg(windows)` conditionals that must be preserved (Med risk)
4. **Wave 1 pattern**: `perl-module-*` → `perl-module` collapse established the template with submodules under `src/`
5. **Test consolidation**: 28 test files distributed unevenly — `perl-dap-variables` has 5 files (4.2K LOC), `perl-dap-stack` has 3 files (2.5K LOC)

## Proposed Approach
Follow the Wave 1 (perl-module) pattern: create `src/components/{types,variables,launch,security}/` directories, move satellite source files into appropriate component modules, update `lib.rs` module declarations, fix internal imports from `perl_dap_*` to `super::components::*`, consolidate tests into `perl-dap/tests/components_*.rs`, and remove satellite entries from workspace Cargo.toml.

## Top Risks
1. **Test consolidation** (Med): 28 test files across 11 crates must be merged — import paths and module references must be updated consistently
2. **Platform conditional code** (Med): `perl-dap-platform` has OS-specific conditionals that must be preserved exactly
3. **External consumers** (Low): `perl-lsp` and `perl-lsp-config` need import path updates from `perl-dap-platform` → `perl-dap`

## Scope
Covers: All 11 satellite crates absorbed into `perl-dap/src/components/`; 28 test files consolidated; workspace metadata updated.
Does NOT cover: Changes to `perl-dap` core (bridge_adapter, debug_adapter, dispatcher, etc.); parser train (Waves 3-4); other microcrate collapse waves.
