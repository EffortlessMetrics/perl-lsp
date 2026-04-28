# Wave 4-Completion — Spec Context

## Issue Scope
Absorb 3 parser satellite crates into `perl-parser` as internal modules (D1-D3 locked from parent #4541 plan-review).

## Locked Decisions (D1-D3, from #4541 plan-review)

### D1: perl-dead-code → perl-parser::dead_code
- Currently: `pub use perl_dead_code as dead_code_detector;` in lib.rs line 459
- Target: Replace with real module, preserve re-export alias for compatibility

### D2: perl-refactoring → perl-parser::refactor
- Currently: `crates/perl-parser/src/refactor.rs` is a re-export shim
- Re-exports from `perl_refactoring::refactor::*`
- Target: Replace shim with real module content from `crates/perl-refactoring/src/refactor/`

### D3: perl-incremental-parsing → perl-parser::incremental
- Currently: `crates/perl-parser/src/incremental.rs` is a re-export shim
- Re-exports from `perl_incremental_parsing::incremental::*`
- Critical consumer: `crates/perl-lsp/src/runtime/text_sync.rs` imports `perl_incremental_parsing::` in 6+ places
- Target: Replace shim with real module content from `crates/perl-incremental-parsing/src/incremental/`, rewire text_sync.rs

## Prior Work Status
- All 3 crates exist as published standalone crates
- All 3 are in `[workspace.metadata.publish.allow]` (baseline: 37 published)
- All 3 have dependencies listed in perl-parser Cargo.toml
- Ledger (`.spec/microcrate-collapse/ledger.md`) planned Wave 4 absorption but work never executed

## Published Count Trajectory
- Current: 37 published (verified master 837030fe7)
- After PR A: 34 published (3 removed from allowlist)
- Final target (with PR B): 31 published

## Why Split from #4541 (PR B)
- PR A (parser satellites) is independent of PR B (LSP deferrals) at dependency level
- PR B requires deleting 2 G3 negative-assertion tests that live in `perl-lsp-rs-core/tests/`
- Coupling parser work to LSP test deletion creates scope-drift risk + builder context exhaustion
- Clean structural seam allows independent spec → red-TDD → builder pipelining

## Acceptance Criteria (from #4542 body)
1. 3 absorbed crate dirs marked `publish = false`
2. 3 allowlist entries removed from root `Cargo.toml`
3. `xtask/published-crate-baseline.txt` updated 37 → 34
4. Existing G3 baseline assertions updated to expect 34 (not 37)
5. `perl-parser/src/dead_code/`, `.../refactor/`, `.../incremental/` modules created with absorbed content
6. `perl-parser/src/lib.rs` exposes all three via `pub mod` + re-export aliases for compat
7. `crates/perl-lsp/src/runtime/text_sync.rs` rewired to `perl_parser::incremental::*`
8. `cargo check --workspace` passes
9. `cargo xtask layer-check` passes
10. `cargo xtask published-crate-count` returns 34
11. `cargo xtask publish-closure` zero violations
12. New test file `crates/perl-parser/tests/wave4_completion_absorption_tests.rs` with per-crate accessibility, publish-false, allowlist-absent, count assertions
13. `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs` passes

## Master Context
- HEAD: 837030fe756a9bff66f48360545e03d9e1ec732f
- G3 baseline post-merge status: baseline.txt = 37 (verified correct via cargo xtask published-crate-count)
- 3 G3 assertion tests exist in `perl-lsp-rs-core/tests/` asserting published count == 37
- No G3 negative tests in parser crates (those are in PR B scope)

## Design Rationale
- **Module absorption, not relocation**: Content moves to perl-parser source tree, not symlinked. This maintains clean dependencies and build semantics.
- **Re-export aliases preserved**: Consumer code outside perl-parser can continue using `perl_parser::dead_code_detector` (for dead_code) and `perl_parser::refactor::*`, `perl_parser::incremental::*` for refactoring/incremental. No consumer breakage.
- **Incremental rewire in text_sync.rs**: Only consumer outside perl-parser (in different crate), so single targeted import update.
- **Push after each absorption**: To avoid builder context exhaustion (memory feedback).

## Related Issues & PRs
- Parent tracker: #4410 (v0.13.0 microcrate collapse)
- Parent spec issue: #4541 (Wave Final — retitled to defer LSP work to PR B)
- Sister issue (Wave Final, PR B): TBD (not filed yet)
- Prior related: #4422 (Wave 1 — module-resolution satellites), #4426 (Wave A — workspace satellites)
