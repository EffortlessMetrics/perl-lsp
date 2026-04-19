# Wave G1b Provider Collapse — Implementation Context

## Critical O5 Correction: perl-lsp-providers is NOT a Pure Aggregator

**Finding (from plan-reviewer direct inspection):**

The issue body incorrectly maps `perl-lsp-providers` → `perl_lsp_rs_core::providers::registry`. This is **wrong**.

**Actual structure of perl-lsp-providers:**

1. **Re-export shims** (`src/lib.rs` public blocks) — forward to now-absorbed sub-crates:
   - `pub use perl_lsp_diagnostics::*;`
   - `pub use perl_lsp_formatting::*;`
   - `pub use perl_lsp_semantic_tokens::*;`
   - etc. for all 9 other G1b crates

2. **Original implementations** (~1,600 LOC) in `src/ide/lsp_compat/`:
   - `signature_help.rs` (550 LOC) — `SignatureHelpProvider`, `SignatureHelp`, `ParameterInfo`, `SignatureInfo`
   - `linked_editing.rs` (407 LOC) — `handle_linked_editing`
   - `selection_range.rs` (232 LOC) — `build_parent_map`, `selection_chain`
   - Plus 24+ additional files (code_lens, folding, document_highlight, lsp_errors, pull_diagnostics, etc.)
   - Some modules feature-gated with `#[cfg(not(target_arch = "wasm32"))]`

**Correct mapping:**

```
perl-lsp-providers/src/
  ├─ lib.rs (re-export shims) → providers module-level re-exports
  └─ ide/lsp_compat/*.rs (1,600 LOC original code) → providers::lsp_compat/ (NEW submodule)
```

**Impact:** Without this correction, the builder would have created `providers::registry` as a pure re-export layer, leaving 1,600 LOC of original code homeless. This would have broken compilation of files importing `perl_lsp_providers::ide::lsp_compat::signature_help::*`, etc.

---

## O4 Verification: Exact Cargo.toml Lines Confirmed

Plan-reviewer enumerated the exact 10 dependency lines to remove from `crates/perl-lsp/Cargo.toml`:

```
Line 36: perl-lsp-providers = { workspace = true, features = ["lsp-compat"] }
Line 37: perl-lsp-formatting = { workspace = true }
Line 48: perl-lsp-code-actions = { workspace = true }
Line 49: perl-lsp-inline-completion = { workspace = true }
Line 50: perl-lsp-ai-provider = { workspace = true }
Line 51: perl-lsp-completion = { workspace = true }
Line 52: perl-lsp-diagnostics = { workspace = true }
Line 54: perl-lsp-navigation = { workspace = true, features = ["lsp-compat"] }
Line 55: perl-lsp-rename = { workspace = true }
Line 56: perl-lsp-semantic-tokens = { workspace = true }
```

Note: Line numbers shift during editing; grep for exact dependency strings and remove them, rather than relying on fixed line numbers.

---

## O3 Protocol: Snapshot Migration Requires Manual Byte Verification

**4 diagnostics snapshot files to migrate:**
```
crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__missing_pragmas_and_unused_variable.snap
crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__package_module_happy_path.snap
crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__script_happy_path.snap
crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__security_string_eval.snap
```

**Migration steps:**

1. Copy each `.snap` file to `crates/perl-lsp-rs-core/tests/snapshots/` (create directory if needed)
2. **DO NOT use `cargo insta review --accept`** — this can mask regressions if snapshots differ
3. Verify byte-identical content using:
   ```bash
   cmp -l source.snap dest.snap  # No output means byte-identical
   # OR
   diff --binary source.snap dest.snap  # No output means byte-identical
   ```
4. Migrate test file: `crates/perl-lsp-diagnostics/tests/diag_snap.rs` → `crates/perl-lsp-rs-core/tests/diag_snap.rs`
5. Update imports in test file: `perl_lsp_diagnostics::` → `perl_lsp_rs_core::providers::diagnostics::`
6. Run `cargo test -p perl-lsp-rs-core diag_snap` — must pass on first run (no pending reviews)
7. If `cargo insta review` shows any differences, each must be manually audited. Any divergence = regression.
8. Document in PR body: "Migrated 4 diagnostics snapshots; content verified byte-identical to pre-G1b content."

**Why this discipline:**
- Insta snapshot files are regression detectors, not auto-accept mechanisms
- A snapshot that differs (even slightly) indicates something changed in the diagnostic engine
- Plan-reviewer's requirement: byte-identical verification is the acceptance standard

---

## O2: Aggregator Public API Surface (Re-exports)

The 9 collapsed providers + new lsp_compat must remain accessible as public API from `perl_lsp_rs_core::providers`.

**Re-export structure in `crates/perl-lsp-rs-core/src/providers/mod.rs`:**

```rust
// Phase 1 & 2 absorbed providers
pub use rename::*;
pub use diagnostics::*;
pub use inline_completion::*;
pub use semantic_tokens::*;
pub use formatting::*;
pub use ai::*;

// Phase 3 absorbed providers
pub use completion::*;
pub use navigation::*;
pub use code_actions::*;

// Original lsp_compat implementations
pub use lsp_compat::*;

// G1a providers already present
pub use completion_item::*;
pub use file_completion::*;
pub use formatting_types::*;
pub use import_management::*;
pub use inlay_hints::*;
pub use folding::*;
pub use on_type_formatting::*;
pub use selection_range::*;
pub use symbol_query::*;
pub use type_hierarchy::*;
pub use workspace_symbols::*;
pub use color::*;
pub use code_lens::*;
pub use document_highlight::*;
pub use document_links::*;

// Deprecated backward-compatibility alias (O2 requirement)
#[deprecated(
    since = "0.9.0",
    note = "Use `perl_lsp_rs_core::providers` directly"
)]
pub use crate as tooling_export;
```

**Why preserve `tooling_export`:**
- External code outside perl-lsp may reference `perl_lsp_rs_core::providers::tooling_export::*`
- Removing it breaks API contract
- The deprecation message guides users to migrate to the direct module path
- Maintaining the alias costs nothing (it's just a module alias)

---

## O1: Module-Level Cycle Audit (Self-Enforcing)

**Intra-G1b provider dependencies:**
- `providers::code_actions` → imports `providers::rename` + `providers::diagnostics`
- `providers::ai` → imports `providers::inline_completion`
- No reverse dependencies (rename/diagnostics do NOT import code_actions; inline_completion does NOT import ai)

**Verification:**
- These become internal crate imports: `use crate::providers::rename::*`
- Rust compiler prevents circular module dependencies — `cargo check -p perl-lsp-rs-core` would fail to compile if any cycle existed
- No separate test needed beyond compilation

---

## Research Corrections Applied

**1. perl-lsp-ai-provider has NO feature gates**
- Issue body claimed "feature-gated"
- Actual: No `[features]` section in Cargo.toml
- Action: Absorb as-is; do not invent feature gates during G1b

**2. perl-lsp-semantic-tokens has ZERO insta snapshots**
- Issue body labeled it "snapshot-heavy"
- Actual: All 4 `.snap` files are exclusively in perl-lsp-diagnostics
- Semantic-tokens has zero snapshots (research verified)
- Action: Remove "snapshot-heavy" rationale; it's a pure-leaf provider

---

## Sequencing Rationale (Why Phases Must Be Ordered)

**Phase 1 (pure leaves):** rename, diagnostics, inline-completion, semantic-tokens
- These have no intra-G1b dependencies
- Can absorb in any order
- Completion before Phase 2 minimizes compile errors

**Phase 2 (near-leaves):** formatting, ai
- `formatting` depends on G1a only (formatting-types)
- `ai` depends on inline-completion (Phase 1) — MUST come after Phase 1
- Both ready to absorb after Phase 1 completes

**Phase 3 (consumers):** completion, navigation, code-actions
- `completion` depends on G1a (completion-item, file-completion)
- `navigation` depends on G1a only
- `code_actions` depends on Phase 1 (rename, diagnostics) — MUST come after Phase 1
- Ready to absorb after Phase 1+2 complete

**Phase 4 (aggregator):** perl-lsp-providers (LAST, ~1,750 LOC total)
- Depends on ALL 9 other G1b providers (Phase 1–3)
- Contains original lsp_compat code + re-export shims
- MUST absorb last when all dependencies are already in place
- Largest item; treat as a mini-collapse itself

**Phase 5–7:** Consumer cleanup + infrastructure + validation
- All absorbed before updating consumers
- Compile errors guide remaining fixes

---

## Known Non-Scope Issues

**perl-parser/src/ide.rs dead code:**
- File contains: `pub use perl_lsp_providers::ide::*`
- But `ide` module is NOT declared in `perl-parser/src/lib.rs`
- This is unreachable/dead code
- Not in scope to fix in G1b (left as-is per plan-reviewer)

---

## Wrapper Pattern (G1a Lesson)

G1a builder added `CodeLensProvider::new()` and `with_source()` because red tests expected constructors.

**For G1b:**
- If red-TDD writes tests expecting `DiagnosticsProvider::new(ast, source)` or similar, verify the signature matches the original crate API
- Do not invent wrappers the red tests do not require
- Document any added constructors in the PR body
- The phase-ordered checklist prevents most of this (no red tests yet), but builder should stay aware

---

## Testing Philosophy

1. **Per-phase verification:** After each phase, run `cargo check -p perl-lsp-rs-core`
2. **Per-module tests:** After each phase, run `cargo test -p perl-lsp-rs-core --lib`
3. **Full suite after completion:** `cargo test -p perl-lsp-rs-core` + `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2`
4. **Final gate:** `just ci-gate` must pass (full CI suite: clippy, fmt, test, snapshots, docs)

---

## Builder Watchpoints

1. **perl-lsp-providers phase (Phase 4):** Heaviest item
   - 1,600 LOC in lsp_compat/
   - 1,727 LOC in comprehensive_unit_tests.rs
   - Moving both + updating 26+ sub-files will be the longest single phase
   - Budget accordingly

2. **Import site sweep:** Broad but mechanical
   - 46+ occurrences of old crate names in perl-lsp/src/
   - Use grep to verify zero misses after all replacements
   - Each replacement is straightforward (s/perl_lsp_X/perl_lsp_rs_core::providers::Y)

3. **Snapshot migration:** Non-standard procedure
   - Use byte-comparison tools, NOT cargo insta
   - Verify all 4 files copy perfectly before deletion
   - If any differ, investigate (regression indicator)

4. **Compilation order matters:** Building out of order will cause cascading compile errors
   - Follow the phase sequence
   - After each phase, run `cargo check -p perl-lsp-rs-core` to verify no breakage
   - If a phase compiles, you're ready for the next

---

## Call-Site Enumeration (Pre-Phase 5)

**Perl-lsp/src import sites to update (15+ files):**

1. features/code_actions.rs — `perl_lsp_code_actions::*`
2. features/code_actions_enhanced.rs — `perl_lsp_code_actions::EnhancedCodeActionsProvider`
3. features/completion.rs — `perl_lsp_completion::*`
4. features/diagnostics/mod.rs — `perl_lsp_diagnostics::*` block
5. features/diagnostics/pull.rs — 5 inline refs to `perl_lsp_diagnostics::`
6. features/document_links.rs — `perl_lsp_navigation::*`
7. features/folding.rs — `perl_lsp_providers::ide::lsp_compat::folding::*`
8. features/formatting.rs — `perl_lsp_formatting::*` block
9. features/inline_completions.rs — `perl_lsp_inline_completion::*` block
10. features/linked_editing.rs — `perl_lsp_providers::ide::lsp_compat::linked_editing::*`
11. features/on_type_formatting.rs — `perl_lsp_providers::ide::lsp_compat::on_type_formatting::*`
12. features/references.rs — `perl_lsp_navigation::*`
13. features/rename.rs — `perl_lsp_rename::*`
14. features/selection_range.rs — `perl_lsp_providers::ide::lsp_compat::selection_range::*`
15. features/semantic_tokens.rs — `perl_lsp_semantic_tokens::*`
16. features/signature_help.rs — `perl_lsp_providers::ide::lsp_compat::signature_help::*`
17. features/type_definition.rs — `perl_lsp_navigation::*`
18. features/workspace_symbols.rs — `perl_lsp_navigation::*`
19. runtime/diagnostics.rs — 3 inline refs to `perl_lsp_diagnostics::`
20. runtime/language/misc.rs — 5 refs to `perl_lsp_inline_completion::`
21. runtime/language/streaming.rs — 5 refs to `perl_lsp_inline_completion::`
22. runtime/mod.rs — 3 refs to `perl_lsp_ai_provider::`

Comprehensive grep to verify all sites before Phase 5:
```bash
grep -rn 'perl_lsp_providers\|perl_lsp_formatting\|perl_lsp_code_actions\|perl_lsp_inline_completion\|perl_lsp_ai_provider\|perl_lsp_completion\|perl_lsp_diagnostics\|perl_lsp_navigation\|perl_lsp_rename\|perl_lsp_semantic_tokens' crates/perl-lsp/src/ --include="*.rs"
```

---

## G1 Combined Impact

**G1a (merged 2026-04-19):** 15 low-risk crates → perl-lsp-rs-core
- Result: 74 → 59 published crates

**G1b (this issue):** 10 medium-risk crates → perl-lsp-rs-core
- Result: 59 → 49 published crates
- Combined G1a+G1b: 74 → 49 (−25, matching parent #4496 target)

**End state:** Microcrate collapse Wave G1 complete; perl-lsp-rs-core is now the comprehensive LSP provider facade with 25 absorbed providers (G1a+G1b).

---

**Context document prepared by plan-reviewer. Builder should refer back to this for rationale on O1–O5 before implementing.**
