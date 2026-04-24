# Implementation Checklist: #4429 — Wave E Microcrate Collapse

**Branch:** `impl/4429-perl-diagnostics`

**Summary:** Create new published crate `perl-diagnostics` (crate name = `perl_diagnostics`) by absorbing 3 existing crates AND unifying duplicated types in-wave:
1. `perl-diagnostics-codes` → `src/codes/mod.rs` (canonical definitions of `DiagnosticSeverity` and `DiagnosticTag`)
2. `perl-lsp-diagnostic-catalog` → `src/catalog/mod.rs`
3. `perl-lsp-diagnostic-types` → `src/types/mod.rs` (keeps `Diagnostic` + `RelatedInformation` structs; `DiagnosticSeverity` / `DiagnosticTag` become `pub use crate::codes::{...}` re-exports, NOT separate enum definitions)

**Scope boundary:**
- **IN SCOPE**:
  - Create `crates/perl-diagnostics/` directory tree
  - Migrate source from 3 crates into modules
  - **Unify `DiagnosticSeverity` and `DiagnosticTag`** — canonical definitions in `codes/`; `types/` re-exports them (orchestrator-locked decision, see plan-review comment on #4429)
  - Update `Cargo.toml` (workspace root + 3 consumers)
  - Migrate 6 external test files + 4 inline tests + update import paths
  - Add `tests/type_unification.rs` to verify cross-path type identity
  - Add xtask layer-check rule forbidding `perl-diagnostics` → `perl-lsp-*`
  - Update README in new crate
- **OUT OF SCOPE**:
  - Ledger amendment (`.spec/microcrate-collapse/ledger.md:149`) — separate follow-up docs PR
  - Absorbing `perl-lsp-diagnostics` — that is Wave G1 scope, NOT Wave E

**Workspace member change:** 123 current → 121 post (net −2)
**Publish allowlist change:** 120 → 118 (net −2: remove 3, add 1)

---

## Change Order (compiles at each step)

### Step 1: Create `crates/perl-diagnostics/` directory structure
- **Files created:**
  - `crates/perl-diagnostics/Cargo.toml`
  - `crates/perl-diagnostics/src/lib.rs`
  - `crates/perl-diagnostics/src/api.rs`
  - `crates/perl-diagnostics/src/codes/mod.rs`
  - `crates/perl-diagnostics/src/types/mod.rs`
  - `crates/perl-diagnostics/src/catalog/mod.rs`
  - `crates/perl-diagnostics/README.md`
  - `crates/perl-diagnostics/tests/` (empty directory)
- **Details:**
  - `Cargo.toml`: standard workspace template with `name = "perl-diagnostics"`, `edition = "2024"`, `serde_json` as a required dep (NOT behind a feature flag), `serde` as optional feature
  - `src/lib.rs`: module declarations + `pub use api::*;` re-export
  - `src/api.rs`: explicit per-symbol re-exports (no wildcards; compile-error safe)
  - `src/codes/mod.rs`: contains all content from `crates/perl-diagnostics-codes/src/lib.rs` (lines 1–end) — includes canonical `DiagnosticSeverity`, `DiagnosticTag`, `DiagnosticCategory`, `DiagnosticCode` enums
  - `src/types/mod.rs`: contains `Diagnostic` and `RelatedInformation` structs from `crates/perl-lsp-diagnostic-types/src/lib.rs`, but does NOT re-declare `DiagnosticSeverity` or `DiagnosticTag` — those come from `pub use crate::codes::{DiagnosticSeverity, DiagnosticTag};`
  - `src/catalog/mod.rs`: contains all content from `crates/perl-lsp-diagnostic-catalog/src/lib.rs` (lines 1–end) with module docstring adjusted for its new position; ALSO includes the inline `#[cfg(test)] mod tests` block from `perl-lsp-diagnostic-catalog/src/lib.rs:169-205` (4 tests)
  - `tests/` directory created but left empty (test files will be added in Steps 16–19)
- **Verify:** `cargo check -p perl-diagnostics`

### Step 2: Update `codes/mod.rs` for in-module consistency
- **File:** `crates/perl-diagnostics/src/codes/mod.rs`
- **Change:** Ensure the module compiles standalone; remove any `use perl_diagnostics_codes::*` or similar noise from the pre-collapse source.
- **Details:**
  - `codes/mod.rs` must not reference the former `perl_diagnostics_codes` crate.
  - Keep all `use std::*;` and `use serde::*;` as-is.
  - This module owns the canonical `DiagnosticSeverity`, `DiagnosticTag`, `DiagnosticCategory`, `DiagnosticCode` enums — do NOT remove or alter their derives.
- **Verify:** `cargo check -p perl-diagnostics`

### Step 3: Update `types/mod.rs` — unify types via re-export (orchestrator-locked)
- **File:** `crates/perl-diagnostics/src/types/mod.rs`
- **Change:** Remove the redundant `DiagnosticSeverity` and `DiagnosticTag` enum definitions that were in `perl-lsp-diagnostic-types/src/lib.rs`. Replace them with `pub use crate::codes::{DiagnosticSeverity, DiagnosticTag};`. Keep the `Diagnostic` and `RelatedInformation` structs as-is — they reference `DiagnosticSeverity`, which now resolves to the canonical `codes::` type via the re-export.
- **Details:**
  - Delete lines in `types/mod.rs` that declare `pub enum DiagnosticSeverity { ... }` and `pub enum DiagnosticTag { ... }` (roughly `perl-lsp-diagnostic-types/src/lib.rs:12-27` and `:63-68`).
  - Add at the top of `types/mod.rs` (after module docstring, before the `Diagnostic` struct):
    ```rust
    // DiagnosticSeverity and DiagnosticTag are unified with the canonical definitions in `codes::`.
    // This re-export keeps the `perl_diagnostics::types::DiagnosticSeverity` path valid for
    // consumers that previously used `perl_lsp_diagnostic_types::DiagnosticSeverity`.
    pub use crate::codes::{DiagnosticSeverity, DiagnosticTag};
    ```
  - The `Diagnostic` struct's `severity: DiagnosticSeverity` field now binds to the canonical `codes::DiagnosticSeverity` via this re-export — no struct-definition change needed.
  - `RelatedInformation` struct stays unchanged.
- **Verify:** `cargo check -p perl-diagnostics`

### Step 4: Update `catalog/mod.rs` for internal module references
- **File:** `crates/perl-diagnostics/src/catalog/mod.rs`
- **Change:** Replace external crate imports with internal module imports.
- **Details:**
  - `use perl_diagnostics_codes::DiagnosticCode;` → `use crate::codes::DiagnosticCode;`
  - Any other cross-module references updated to `crate::*` style.
  - Keep `serde_json::*` imports as-is (required dep).
  - The inline `#[cfg(test)] mod tests` block (4 tests from `perl-lsp-diagnostic-catalog/src/lib.rs:169-205`) lives inside this file. Inside that block, update any `use super::*;` to still resolve; update `use perl_diagnostics_codes::*` inside the block to `use crate::codes::*;`.
- **Verify:** `cargo check -p perl-diagnostics` and `cargo test -p perl-diagnostics --lib`

### Step 5: Define `src/api.rs` re-export surface (explicit, no wildcards)
- **File:** `crates/perl-diagnostics/src/api.rs`
- **Change:** Write explicit per-symbol re-exports.
- **Details:**
  - Pattern:
    ```rust
    pub use crate::codes::{DiagnosticCode, DiagnosticCategory, DiagnosticSeverity, DiagnosticTag};
    pub use crate::types::{Diagnostic, RelatedInformation};
    // Note: DiagnosticSeverity and DiagnosticTag are canonically defined in codes::
    // and re-exported via types::. api.rs re-exports them via the canonical codes:: path.
    pub use crate::catalog::{
        DiagnosticMeta, diagnostic_meta, parse_error, syntax_error,
        unexpected_eof, missing_strict, missing_warnings, unused_var, undefined_var,
        missing_package_declaration, duplicate_package, duplicate_sub, missing_return,
        bareword_filehandle, two_arg_open, implicit_return, eval_error_flow,
        critic_severity_5, critic_severity_4, critic_severity_3, critic_severity_2,
        critic_severity_1, from_message,
    };
    ```
  - **IMPORTANT**: Do NOT use wildcard re-exports (`pub use crate::codes::*;`) — explicit lists are the locked pattern regardless of whether collisions currently exist.
  - Look at current `crates/perl-lsp-diagnostic-catalog/src/lib.rs` to verify all public function names in the catalog re-export list are present.
- **Verify:** `cargo check -p perl-diagnostics`

### Step 6: Update `src/lib.rs` to declare modules and re-export API
- **File:** `crates/perl-diagnostics/src/lib.rs`
- **Change:** Write module declarations and public re-export.
- **Details:**
  - Content:
    ```rust
    //! Unified diagnostic codes, types, and catalog for Perl LSP.
    //!
    //! This crate consolidates three previously separate diagnostic crates:
    //! - `perl-diagnostics-codes` — stable diagnostic codes, severity, and tags (now `codes` module)
    //! - `perl-lsp-diagnostic-types` — diagnostic model types (Diagnostic, RelatedInformation) (now `types` module)
    //! - `perl-lsp-diagnostic-catalog` — LSP metadata builders for codes (now `catalog` module)
    //!
    //! # Modules
    //!
    //! - [`codes`] — canonical `DiagnosticCode`, `DiagnosticCategory`, `DiagnosticSeverity`, `DiagnosticTag`
    //! - [`types`] — `Diagnostic` and `RelatedInformation` structs; `DiagnosticSeverity` and `DiagnosticTag` are re-exported from [`codes`]
    //! - [`catalog`] — LSP metadata catalog functions
    //!
    //! # Type unification
    //!
    //! `DiagnosticSeverity` and `DiagnosticTag` are single canonical types defined in [`codes`].
    //! The [`types`] module re-exports them so the legacy `types::DiagnosticSeverity` import
    //! path still resolves to the same underlying type.
    //!
    //! # Re-exports
    //!
    //! The crate root re-exports all public items via [`api`].

    #![deny(unsafe_code)]
    #![warn(rust_2018_idioms)]
    #![warn(missing_docs)]
    #![warn(clippy::all)]

    pub mod codes;
    pub mod types;
    pub mod catalog;

    mod api;
    pub use api::*;
    ```
- **Verify:** `cargo check -p perl-diagnostics`

### Step 7: Update workspace `Cargo.toml` — members section
- **File:** `/h/Code/Rust/perl-lsp/Cargo.toml`
- **Change:** Remove 3 crate entries; add 1 new entry.
- **Details:**
  - Remove lines:
    - `"crates/perl-diagnostics-codes",`
    - `"crates/perl-lsp-diagnostic-catalog",`
    - `"crates/perl-lsp-diagnostic-types",`
  - Add line:
    - `"crates/perl-diagnostics",` (insert in alphabetical position)
- **Verify:** `cargo check --all`

### Step 8: Update workspace `Cargo.toml` — `[workspace.dependencies]` section
- **File:** `/h/Code/Rust/perl-lsp/Cargo.toml`
- **Change:** Update `[workspace.dependencies]` block.
- **Details:**
  - Remove entries for:
    - `perl-diagnostics-codes`
    - `perl-lsp-diagnostic-catalog`
    - `perl-lsp-diagnostic-types`
  - Add:
    ```toml
    perl-diagnostics = { path = "crates/perl-diagnostics", version = "0.12.4" }
    ```
- **Verify:** `cargo check --all`

### Step 9: Update workspace `Cargo.toml` — publish allowlist
- **File:** `/h/Code/Rust/perl-lsp/Cargo.toml`
- **Change:** Update `[workspace.metadata.publish] allow = [...]`.
- **Details:**
  - Remove 3 entries:
    - `"perl-diagnostics-codes",`
    - `"perl-lsp-diagnostic-catalog",`
    - `"perl-lsp-diagnostic-types",`
  - Add single entry:
    - `"perl-diagnostics",` (insert in Tier 3 in alphabetical position)
  - **Result:** 120 → 118 allowlist entries.
- **Verify:** `cargo check --all`

### Step 10: Update `perl-lsp-code-actions` Cargo.toml
- **File:** `crates/perl-lsp-code-actions/Cargo.toml`
- **Change:** Replace dependency.
- **Details:**
  - Find line: `perl-diagnostics-codes = { workspace = true }`
  - Replace with: `perl-diagnostics = { workspace = true }`
- **Verify:** `cargo check -p perl-lsp-code-actions`

### Step 11: Update `perl-lsp-code-actions` source imports
- **File:** `crates/perl-lsp-code-actions/src/lib.rs` (and any other source files)
- **Change:** Replace diagnostic code imports.
- **Details:**
  - Find and replace: `use perl_diagnostics_codes::` → `use perl_diagnostics::codes::`
  - Keep all usage of types the same; only change the import path.
- **Verify:** `cargo check -p perl-lsp-code-actions`

### Step 12: Update `perl-lsp-diagnostics` Cargo.toml
- **File:** `crates/perl-lsp-diagnostics/Cargo.toml`
- **Change:** Replace 2 dependencies with 1. Note: `perl-lsp-diagnostics` remains a separate crate (Wave G1 scope, not absorbed in Wave E).
- **Details:**
  - Find and remove:
    - `perl-diagnostics-codes = { workspace = true }`
    - `perl-lsp-diagnostic-types = { workspace = true }`
  - Add:
    - `perl-diagnostics = { workspace = true }`
- **Verify:** `cargo check -p perl-lsp-diagnostics`

### Step 13: Update `perl-lsp-diagnostics` source imports
- **File:** `crates/perl-lsp-diagnostics/src/lib.rs` (and any other source files)
- **Change:** Replace imports from both old crates.
- **Details:**
  - `use perl_diagnostics_codes::` → `use perl_diagnostics::codes::`
  - `use perl_lsp_diagnostic_types::` → `use perl_diagnostics::types::`
  - Preserve all type usage; only paths change. Because `types::DiagnosticSeverity` now re-exports from `codes::`, any code that assigns a severity value across paths continues to work.
- **Verify:** `cargo check -p perl-lsp-diagnostics`

### Step 14: Update `perl-lsp` (LSP server) Cargo.toml
- **File:** `crates/perl-lsp/Cargo.toml`
- **Change:** Replace 2 dependencies with 1.
- **Details:**
  - Find and remove:
    - `perl-diagnostics-codes = { workspace = true }`
    - `perl-lsp-diagnostic-catalog = { workspace = true }`
  - Add:
    - `perl-diagnostics = { workspace = true }`
- **Verify:** `cargo check -p perl-lsp`

### Step 15: Update `perl-lsp` source imports
- **File:** `crates/perl-lsp/src/lib.rs` (and any other source files that reference diagnostics)
- **Change:** Replace diagnostic imports.
- **Details:**
  - `use perl_diagnostics_codes::` → `use perl_diagnostics::codes::`
  - `use perl_lsp_diagnostic_catalog::` → `use perl_diagnostics::catalog::`
  - Check all files in `crates/perl-lsp/src/` for these imports (grep first to find them).
- **Verify:** `cargo check -p perl-lsp`

### Step 16: Migrate test files (codes_*)
- **Files created:**
  - `crates/perl-diagnostics/tests/codes_comprehensive_unit_tests.rs`
  - `crates/perl-diagnostics/tests/codes_context_hint_tests.rs`
  - `crates/perl-diagnostics/tests/codes_diagnostic_code_completeness.rs`
- **Change:** Copy test content from old crates and update imports.
- **Details:**
  - Copy from: `crates/perl-diagnostics-codes/tests/*.rs` (3 files).
  - Update imports: `use perl_diagnostics_codes::` → `use perl_diagnostics::codes::`.
  - File naming: prefix with `codes_` to disambiguate in new crate.
- **Verify:** `cargo test -p perl-diagnostics --test codes_comprehensive_unit_tests`

### Step 17: Migrate test files (catalog_*)
- **Files created:**
  - `crates/perl-diagnostics/tests/catalog_coverage.rs`
  - `crates/perl-diagnostics/tests/catalog_context_hint_tests.rs`
- **Change:** Copy test content and update imports.
- **Details:**
  - Copy from: `crates/perl-lsp-diagnostic-catalog/tests/*.rs` (2 files).
  - Update imports: `use perl_lsp_diagnostic_catalog::` → `use perl_diagnostics::catalog::`.
  - File naming: prefix with `catalog_` to disambiguate.
- **Verify:** `cargo test -p perl-diagnostics --test catalog_coverage`

### Step 18: Migrate test files (types_*)
- **Files created:**
  - `crates/perl-diagnostics/tests/types_comprehensive_unit_tests.rs`
- **Change:** Copy test content and update imports.
- **Details:**
  - Copy from: `crates/perl-lsp-diagnostic-types/tests/comprehensive_unit_tests.rs`.
  - Update imports: `use perl_lsp_diagnostic_types::` → `use perl_diagnostics::types::`.
  - File naming: prefix with `types_` to disambiguate.
- **Verify:** `cargo test -p perl-diagnostics --test types_comprehensive_unit_tests`

### Step 19: Add `tests/type_unification.rs` (new — verifies orchestrator-locked unification)
- **File created:** `crates/perl-diagnostics/tests/type_unification.rs`
- **Change:** New test verifying cross-path type identity.
- **Details:**
  - Content (from plan-reviewer spec on #4429):
    ```rust
    //! Verify DiagnosticSeverity and DiagnosticTag are a single unified type after Wave E.
    //! `types::` re-exports from `codes::` — assigning between them must compile.

    use perl_diagnostics::codes::DiagnosticSeverity as CodesSeverity;
    use perl_diagnostics::types::DiagnosticSeverity as TypesSeverity;
    use perl_diagnostics::codes::DiagnosticTag as CodesTag;
    use perl_diagnostics::types::DiagnosticTag as TypesTag;

    #[test]
    fn severity_is_unified_single_type() -> Result<(), Box<dyn std::error::Error>> {
        let from_codes = CodesSeverity::Error;
        let _as_types: TypesSeverity = from_codes; // same type — must compile
        assert_eq!(from_codes as u8, 1);
        Ok(())
    }

    #[test]
    fn tag_is_unified_single_type() -> Result<(), Box<dyn std::error::Error>> {
        let from_codes = CodesTag::Unnecessary;
        let _as_types: TypesTag = from_codes; // same type — must compile
        assert_eq!(from_codes.to_lsp_value(), 1);
        Ok(())
    }
    ```
- **Verify:** `cargo test -p perl-diagnostics --test type_unification`

### Step 20: Full test suite
- **Verify:** `cargo test -p perl-diagnostics` (all 6 external test files + 4 inline catalog tests + unification tests run)

### Step 21: Add xtask layer-check rule
- **Files modified:** xtask layer-check configuration (see existing layer rules in `xtask/src/`)
- **Change:** Add rule forbidding `perl-diagnostics` from depending on any `perl-lsp-*` crate.
- **Details:**
  - Mirror the pattern used for other leaf-crate layer constraints in the existing layer-check tool.
  - This prevents future drift where someone adds LSP wire types to the diagnostic kernel.
- **Verify:** `cargo xtask layer-check` passes; manually induce a violation (add `perl-lsp-diagnostics` as a dep in `perl-diagnostics/Cargo.toml`) and confirm the layer-check rejects it; revert the induced violation.

### Step 22: Workspace-wide compilation
- **Verify:** `cargo build -p perl-diagnostics --release`

### Step 23: Lint and format
- **Verify:**
  - `cargo xtask fmt`
  - `cargo clippy -p perl-diagnostics`
  - `cargo clippy -p perl-lsp-code-actions`
  - `cargo clippy -p perl-lsp-diagnostics`
  - `cargo clippy -p perl-lsp`

### Step 24: Full workspace check
- **Verify:**
  - `cargo test --lib -p perl-diagnostics`
  - `cargo test --lib -p perl-lsp-code-actions`
  - `cargo test --lib -p perl-lsp-diagnostics`
  - `cargo check -p perl-lsp`

### Step 25: Delete old crate directories
- **Files deleted:**
  - `crates/perl-diagnostics-codes/` (entire directory)
  - `crates/perl-lsp-diagnostic-catalog/` (entire directory)
  - `crates/perl-lsp-diagnostic-types/` (entire directory)
- **Change:** Remove from filesystem after all imports updated and tests pass.
- **Details:**
  - This step is LAST to preserve ability to diff old source if needed during build.
  - Once deleted, `cargo check --all` should pass without reference errors.
- **Verify:** `cargo check --all` (workspace clean with old dirs gone)

### Step 26: Final verification
- **Verify:**
  - `cargo test --workspace --lib` (all tests pass)
  - `cargo xtask fmt --check` (no formatting issues)
  - `cargo clippy --workspace` (no clippy warnings)
  - `cargo xtask publish-closure` passes
  - `cargo xtask layer-check` passes
  - Workspace member count: exactly 121 (started with 123, removed 3, added 1)
  - Publish allowlist count: exactly 118 (started with 120, removed 3, added 1)

---

## Callers and Consumers

### `perl-diagnostics-codes` crate consumers (to update):
- `perl-lsp-code-actions` (Cargo.toml dependency)
- `perl-lsp-diagnostics` (Cargo.toml dependency — **NOT absorbed**, stays as its own crate for Wave G1)
- `perl-lsp-rs` in `crates/perl-lsp/` (Cargo.toml dependency)
- `perl-lsp-diagnostic-catalog` (being collapsed into `perl-diagnostics::catalog`)

### `perl-lsp-diagnostic-types` crate consumers (to update):
- `perl-lsp-diagnostics` (Cargo.toml dependency)

### `perl-lsp-diagnostic-catalog` crate consumers (to update):
- `perl-lsp-rs` in `crates/perl-lsp/` (Cargo.toml dependency)

### Functions from migrated modules:
- `DiagnosticCode` enum (codes module) — used in code-actions, diagnostics, LSP
- `DiagnosticSeverity` enum (codes module, canonical — re-exported by types) — used widely
- `DiagnosticTag` enum (codes module, canonical — re-exported by types) — used in diagnostic analysis
- `DiagnosticCategory` enum (codes module) — **do not forget to re-export in api.rs**
- `diagnostic_meta()` function (catalog module) — called from LSP diagnostic reporting
- `parse_error()`, `syntax_error()`, etc. (catalog module) — called from parser diagnostics
- `context_hint()` method on `DiagnosticCode` — internal call after collapse (same crate)

---

## Scope Boundary

### Files IN scope:
1. `/h/Code/Rust/perl-lsp/Cargo.toml` (workspace root — members, deps, allowlist)
2. `crates/perl-diagnostics/` (new crate — all files)
3. `crates/perl-lsp-code-actions/Cargo.toml` (dependency)
4. `crates/perl-lsp-code-actions/src/` (update imports)
5. `crates/perl-lsp-diagnostics/Cargo.toml` (dependencies)
6. `crates/perl-lsp-diagnostics/src/` (update imports)
7. `crates/perl-lsp/Cargo.toml` (dependencies)
8. `crates/perl-lsp/src/` (update imports)
9. `xtask/` (add layer-check rule for perl-diagnostics → perl-lsp-* forbidden)
10. `crates/perl-diagnostics-codes/` (source — deleted at end)
11. `crates/perl-lsp-diagnostic-catalog/` (source — deleted at end)
12. `crates/perl-lsp-diagnostic-types/` (source — deleted at end)

### Files OUT of scope:
- `.spec/microcrate-collapse/ledger.md:149` (amendment is a separate follow-up docs PR)
- Absorbing `perl-lsp-diagnostics` (that is Wave G1 scope)
- Any refactoring of diagnostic code logic beyond the type-unification re-export
- Documentation updates beyond the new crate README

---

## Flags for Builder

### Ambiguities and decisions:

1. **`api.rs` re-export pattern is CRITICAL:**
   - Do NOT use wildcard re-exports (`pub use crate::codes::*;`).
   - Use explicit per-symbol lists only. This is the locked pattern regardless of whether name collisions currently exist — prevents silent breakage from future module edits.
   - Do not forget to include `DiagnosticCategory` in the `codes::` re-export list.

2. **Type unification is LOCKED — implement in this wave:**
   - `codes::DiagnosticSeverity` and `codes::DiagnosticTag` are the canonical definitions.
   - `types/mod.rs` does `pub use crate::codes::{DiagnosticSeverity, DiagnosticTag};` — do NOT re-declare these enums in `types/mod.rs`.
   - This is a non-breaking widening (the `codes::` version has a strict superset of trait derives).
   - Verify with `tests/type_unification.rs` that cross-path assignment compiles.

3. **Inline tests at `perl-lsp-diagnostic-catalog/src/lib.rs:169-205`:**
   - There are 4 inline tests in a `#[cfg(test)] mod tests` block (not counted by the accuracy-scout's external file count):
     - `parse_error_includes_stable_code_and_docs_url`
     - `critic_codes_have_no_docs_url`
     - `eval_error_flow_has_stable_code_and_docs_url`
     - `message_inference_is_case_insensitive`
   - These must be migrated inline to `src/catalog/mod.rs` as a `#[cfg(test)] mod tests` block.

4. **Cross-module references in migrated code:**
   - When you copy source from 3 old crates into 3 modules of new crate, check for inter-module references.
   - Example: If `catalog/mod.rs` references `DiagnosticCode` from `codes/`, change `perl_diagnostics_codes::DiagnosticCode` to `crate::codes::DiagnosticCode`.
   - Use grep in each module to find all external imports that need updating: `use perl_`.

5. **Feature flags:**
   - `[features]` section in new `Cargo.toml` includes `serde` as optional (matches old crates).
   - `serde_json` is a **required dep** (not optional) — catalog/mod.rs uses it unconditionally. Do not move it behind a feature.

6. **Test file naming:**
   - Use prefix convention to avoid name collisions: `codes_*.rs`, `catalog_*.rs`, `types_*.rs`.
   - Ensures all 6 external test files can coexist in `crates/perl-diagnostics/tests/`.
   - Plus `type_unification.rs` (new).

7. **`perl-lsp-diagnostics` stays as its own crate:**
   - It is a Wave G1 consumer, NOT a Wave E absorbed crate. Do not delete or absorb it.
   - Only update its Cargo.toml dep and source imports.

8. **Publish allowlist position:**
   - New crate sits in Tier 3 (analysis and indexing tier). Insert alphabetically within the tier.

9. **Edition and workspace fields:**
   - Use `edition = "2024"` (matches the collapsed `perl-module` pilot; pre-existing workspace MSRV).
   - Use `rust-version.workspace = true`, `authors.workspace = true`, etc.

---

## Test Coverage

**Expected test files in new crate:**
1. `tests/codes_comprehensive_unit_tests.rs` (from `perl-diagnostics-codes`)
2. `tests/codes_context_hint_tests.rs` (from `perl-diagnostics-codes`)
3. `tests/codes_diagnostic_code_completeness.rs` (from `perl-diagnostics-codes`)
4. `tests/catalog_coverage.rs` (from `perl-lsp-diagnostic-catalog`)
5. `tests/catalog_context_hint_tests.rs` (from `perl-lsp-diagnostic-catalog`)
6. `tests/types_comprehensive_unit_tests.rs` (from `perl-lsp-diagnostic-types`)
7. `tests/type_unification.rs` (**new** — verifies orchestrator-locked type unification)

Plus 4 inline tests migrated into `src/catalog/mod.rs`.

**Expected test count:** 6 external test files migrated + 1 new external test file + 4 inline catalog tests = 10 test units total, from 3 source crates.

---

## Compilation Gates

- **Each step compiles**: Use `cargo check -p <crate>` or `cargo check --all` as specified.
- **No unstaged changes**: Commit only spec files to this branch before handing to red-TDD.
- **Final gate**: `cargo test --workspace --lib && cargo clippy --workspace && cargo xtask layer-check && cargo xtask publish-closure`.
