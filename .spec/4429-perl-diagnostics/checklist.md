# Implementation Checklist: #4429 — Wave E Microcrate Collapse

**Branch:** `impl/4429-perl-diagnostics`

**Summary:** Create new published crate `perl-diagnostic-catalog` (crate name = `perl_diagnostic_catalog`) by absorbing 3 existing crates:
1. `perl-diagnostics-codes` → `src/codes/mod.rs`
2. `perl-lsp-diagnostic-catalog` → `src/catalog/mod.rs`
3. `perl-lsp-diagnostic-types` → `src/types/mod.rs`

**Scope boundary:**
- **IN SCOPE**: 
  - Create `crates/perl-diagnostic-catalog/` directory tree
  - Migrate source from 3 crates into modules
  - Update `Cargo.toml` (workspace root + 4 consumers)
  - Migrate 6 test files + update import paths
  - Update README in new crate
- **OUT OF SCOPE**: 
  - Ledger amendment (`.spec/microcrate-collapse/ledger.md`) — separate follow-up PR
  - Type unification (`codes::DiagnosticSeverity` vs `types::DiagnosticSeverity`) — deferred to v0.15.0, documented in new crate README

**Workspace member change:** 122 current → 120 post (net −2)
**Publish allowlist change:** 120 → 118 (net −2: remove 3, add 1)

---

## Change Order (compiles at each step)

### Step 1: Create `crates/perl-diagnostic-catalog/` directory structure
- **Files created:**
  - `crates/perl-diagnostic-catalog/Cargo.toml`
  - `crates/perl-diagnostic-catalog/src/lib.rs`
  - `crates/perl-diagnostic-catalog/src/api.rs`
  - `crates/perl-diagnostic-catalog/src/codes/mod.rs`
  - `crates/perl-diagnostic-catalog/src/types/mod.rs`
  - `crates/perl-diagnostic-catalog/src/catalog/mod.rs`
  - `crates/perl-diagnostic-catalog/README.md`
  - `crates/perl-diagnostic-catalog/tests/` (empty directory)
- **Details:**
  - `Cargo.toml`: standard workspace template (see plan-reviewer spec for full template)
  - `src/lib.rs`: module declarations + `pub use api::*;` re-export
  - `src/api.rs`: explicit per-symbol re-exports (no wildcards; compile-error safe)
  - `src/codes/mod.rs`: contains all content from `crates/perl-diagnostics-codes/src/lib.rs` (lines 1–end)
  - `src/types/mod.rs`: contains all content from `crates/perl-lsp-diagnostic-types/src/lib.rs` (lines 1–end)
  - `src/catalog/mod.rs`: contains all content from `crates/perl-lsp-diagnostic-catalog/src/lib.rs` (lines 1–end) but remove the initial module docstring and imports — it's now a module
  - `tests/` directory created but left empty (test files will be added in Step 2)
- **Verify:** `cargo check -p perl-diagnostic-catalog`

### Step 2: Update `Cargo.toml` Cargo.toml for `codes/mod.rs` imports
- **File:** `crates/perl-diagnostic-catalog/src/codes/mod.rs`
- **Change:** Remove `use perl_diagnostics_codes::*;` imports and replace with direct imports from siblings
- **Details:**
  - `codes/mod.rs` must not reference external `perl_diagnostics_codes` crate
  - Keep all `use std::*;` and `use serde::*;` as-is
  - Any cross-module references (e.g., `types::DiagnosticSeverity`) must be changed to absolute paths like `crate::types::DiagnosticSeverity` or kept local if in same module
  - This step just verifies the module compiles internally
- **Verify:** `cargo check -p perl-diagnostic-catalog`

### Step 3: Update `catalog/mod.rs` for internal module references
- **File:** `crates/perl-diagnostic-catalog/src/catalog/mod.rs`
- **Change:** Replace external crate imports with internal module imports
- **Details:**
  - `use perl_diagnostics_codes::DiagnosticCode;` → `use crate::codes::DiagnosticCode;`
  - Any other cross-module references updated to `crate::*` style
  - Keep `serde_json::*` imports as-is
- **Verify:** `cargo check -p perl-diagnostic-catalog`

### Step 4: Define `src/api.rs` re-export surface
- **File:** `crates/perl-diagnostic-catalog/src/api.rs` (create if not already created)
- **Change:** Write explicit per-symbol re-exports
- **Details:**
  - Pattern: `pub use crate::codes::{DiagnosticCode, DiagnosticSeverity, DiagnosticTag};`
  - Pattern: `pub use crate::types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation};`
  - Pattern: `pub use crate::catalog::{diagnostic_meta, parse_error, syntax_error, /* ... all public fns */};`
  - **IMPORTANT**: Do NOT use wildcard re-exports (`pub use crate::codes::*;`) — they cause "ambiguous reexports" compile error due to `DiagnosticSeverity` and `DiagnosticTag` defined in both `codes` and `types`
  - Keep the list explicit; look at current `crates/perl-lsp-diagnostic-catalog/src/lib.rs` to find all public function names
- **Verify:** `cargo check -p perl-diagnostic-catalog`

### Step 5: Update `src/lib.rs` to declare modules and re-export API
- **File:** `crates/perl-diagnostic-catalog/src/lib.rs`
- **Change:** Write module declarations and public re-export
- **Details:**
  - Content:
    ```rust
    //! Unified diagnostic codes, types, and catalog for Perl LSP.
    //!
    //! This crate consolidates three previously separate diagnostic crates:
    //! - `perl-diagnostics-codes` — stable diagnostic codes and severity levels
    //! - `perl-lsp-diagnostic-types` — diagnostic model types (Diagnostic, RelatedInformation)
    //! - `perl-lsp-diagnostic-catalog` — LSP metadata builders for codes
    //!
    //! # Modules
    //!
    //! - [`codes`] — diagnostic codes, severity, and tags
    //! - [`types`] — diagnostic model types and structures
    //! - [`catalog`] — LSP metadata catalog functions
    //!
    //! # Re-exports
    //!
    //! The crate root re-exports all public items from these modules via [`api`].
    
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
  - Verify lib.rs compiles with module structure in place
- **Verify:** `cargo check -p perl-diagnostic-catalog`

### Step 6: Update workspace `Cargo.toml` — members section
- **File:** `/h/Code/Rust/perl-lsp/Cargo.toml`
- **Change:** Remove 3 crate entries; add 1 new entry
- **Details:**
  - Find the `[workspace] members = [` section (around line 1–130)
  - Remove lines:
    - `"crates/perl-diagnostics-codes",`
    - `"crates/perl-lsp-diagnostic-catalog",`
    - `"crates/perl-lsp-diagnostic-types",`
  - Add line:
    - `"crates/perl-diagnostic-catalog",` (insert in alphabetical position, likely after `"crates/perl-dead-code",`)
- **Verify:** `cargo check --all` (workspace parse check)

### Step 7: Update workspace `Cargo.toml` — dependencies section
- **File:** `/h/Code/Rust/perl-lsp/Cargo.toml`
- **Change:** Update `[workspace.dependencies]` block
- **Details:**
  - Find section with `perl-diagnostics-codes = { path = ..., version = ...}`
  - Replace all 3 entries with single:
    ```toml
    perl-diagnostic-catalog = { path = "crates/perl-diagnostic-catalog", version = "0.12.4" }
    ```
  - Remove entries for:
    - `perl-diagnostics-codes`
    - `perl-lsp-diagnostic-catalog`
    - `perl-lsp-diagnostic-types`
- **Verify:** `cargo check --all`

### Step 8: Update workspace `Cargo.toml` — publish allowlist
- **File:** `/h/Code/Rust/perl-lsp/Cargo.toml`
- **Change:** Update `[workspace.metadata.publish] allow = [...]`
- **Details:**
  - Find the `allow = [` section (around line 200–250)
  - Remove 3 entries:
    - `"perl-diagnostics-codes",` (in Tier 6)
    - `"perl-lsp-diagnostic-catalog",` (in Tier 5)
    - `"perl-lsp-diagnostic-types",` (in Tier 3)
  - Add single entry:
    - `"perl-diagnostic-catalog",` (insert in Tier 3 where `perl-lsp-diagnostic-types` was, after `perl-lsp-inlay-hints`, before `perl-module`)
  - **Result:** 120 → 118 allowlist entries
- **Verify:** `cargo check --all`

### Step 9: Update `perl-lsp-code-actions` Cargo.toml
- **File:** `crates/perl-lsp-code-actions/Cargo.toml`
- **Change:** Replace dependency
- **Details:**
  - Find line: `perl-diagnostics-codes = { workspace = true }`
  - Replace with: `perl-diagnostic-catalog = { workspace = true }`
- **Verify:** `cargo check -p perl-lsp-code-actions`

### Step 10: Update `perl-lsp-code-actions` source imports
- **File:** `crates/perl-lsp-code-actions/src/lib.rs` (and any other source files)
- **Change:** Replace diagnostic code imports
- **Details:**
  - Find and replace: `use perl_diagnostics_codes::` → `use perl_diagnostic_catalog::codes::`
  - Example: `use perl_diagnostics_codes::DiagnosticCode;` → `use perl_diagnostic_catalog::codes::DiagnosticCode;`
  - Keep all usage of types the same; only change the import path
- **Verify:** `cargo check -p perl-lsp-code-actions`

### Step 11: Update `perl-lsp-diagnostics` Cargo.toml
- **File:** `crates/perl-lsp-diagnostics/Cargo.toml`
- **Change:** Replace 2 dependencies with 1
- **Details:**
  - Find and remove:
    - `perl-diagnostics-codes = { workspace = true }`
    - `perl-lsp-diagnostic-types = { workspace = true }`
  - Add:
    - `perl-diagnostic-catalog = { workspace = true }`
- **Verify:** `cargo check -p perl-lsp-diagnostics`

### Step 12: Update `perl-lsp-diagnostics` source imports
- **File:** `crates/perl-lsp-diagnostics/src/lib.rs` (and any other source files)
- **Change:** Replace imports from both old crates
- **Details:**
  - `use perl_diagnostics_codes::` → `use perl_diagnostic_catalog::codes::`
  - `use perl_lsp_diagnostic_types::` → `use perl_diagnostic_catalog::types::`
  - Preserve all type usage; only paths change
- **Verify:** `cargo check -p perl-lsp-diagnostics`

### Step 13: Update `perl-lsp` (LSP server) Cargo.toml
- **File:** `crates/perl-lsp/Cargo.toml`
- **Change:** Replace 2 dependencies with 1
- **Details:**
  - Find and remove:
    - `perl-diagnostics-codes = { workspace = true }`
    - `perl-lsp-diagnostic-catalog = { workspace = true }`
  - Add:
    - `perl-diagnostic-catalog = { workspace = true }`
- **Verify:** `cargo check -p perl-lsp`

### Step 14: Update `perl-lsp` source imports
- **File:** `crates/perl-lsp/src/lib.rs` (and any other source files that reference diagnostics)
- **Change:** Replace diagnostic imports
- **Details:**
  - `use perl_diagnostics_codes::` → `use perl_diagnostic_catalog::codes::`
  - `use perl_lsp_diagnostic_catalog::` → `use perl_diagnostic_catalog::catalog::`
  - Check all files in `crates/perl-lsp/src/` for these imports (use grep to find them)
- **Verify:** `cargo check -p perl-lsp`

### Step 15: Migrate test files (part 1)
- **Files created:**
  - `crates/perl-diagnostic-catalog/tests/codes_comprehensive_unit_tests.rs`
  - `crates/perl-diagnostic-catalog/tests/codes_context_hint_tests.rs`
  - `crates/perl-diagnostic-catalog/tests/codes_diagnostic_code_completeness.rs`
- **Change:** Copy test content from old crates and update imports
- **Details:**
  - Copy from: `crates/perl-diagnostics-codes/tests/comprehensive_unit_tests.rs` → new file
  - Update imports: `use perl_diagnostics_codes::` → `use perl_diagnostic_catalog::codes::`
  - Repeat for all 3 test files from `perl-diagnostics-codes/tests/`
  - File naming: prefix with `codes_` to disambiguate in new crate
- **Verify:** `cargo test -p perl-diagnostic-catalog --lib`

### Step 16: Migrate test files (part 2)
- **Files created:**
  - `crates/perl-diagnostic-catalog/tests/catalog_coverage.rs`
  - `crates/perl-diagnostic-catalog/tests/catalog_context_hint_tests.rs`
- **Change:** Copy test content and update imports
- **Details:**
  - Copy from: `crates/perl-lsp-diagnostic-catalog/tests/*.rs` (2 files)
  - Update imports: `use perl_lsp_diagnostic_catalog::` → `use perl_diagnostic_catalog::catalog::`
  - File naming: prefix with `catalog_` to disambiguate
- **Verify:** `cargo test -p perl-diagnostic-catalog --lib`

### Step 17: Migrate test files (part 3)
- **Files created:**
  - `crates/perl-diagnostic-catalog/tests/types_comprehensive_unit_tests.rs`
- **Change:** Copy test content and update imports
- **Details:**
  - Copy from: `crates/perl-lsp-diagnostic-types/tests/comprehensive_unit_tests.rs`
  - Update imports: `use perl_lsp_diagnostic_types::` → `use perl_diagnostic_catalog::types::`
  - File naming: prefix with `types_` to disambiguate
- **Verify:** `cargo test -p perl-diagnostic-catalog --lib`

### Step 18: Full test suite
- **Verify:** `cargo test -p perl-diagnostic-catalog` (all 6 test files run)

### Step 19: Workspace-wide compilation
- **Verify:** `cargo build -p perl-diagnostic-catalog --release`

### Step 20: Lint and format
- **Verify:**
  - `cargo xtask fmt`
  - `cargo clippy -p perl-diagnostic-catalog`
  - `cargo clippy -p perl-lsp-code-actions`
  - `cargo clippy -p perl-lsp-diagnostics`
  - `cargo clippy -p perl-lsp`

### Step 21: Final full workspace check
- **Verify:**
  - `cargo test --lib -p perl-diagnostic-catalog`
  - `cargo test --lib -p perl-lsp-code-actions`
  - `cargo test --lib -p perl-lsp-diagnostics`
  - `cargo check -p perl-lsp`

### Step 22: Delete old crate directories
- **Files deleted:**
  - `crates/perl-diagnostics-codes/` (entire directory)
  - `crates/perl-lsp-diagnostic-catalog/` (entire directory)
  - `crates/perl-lsp-diagnostic-types/` (entire directory)
- **Change:** Remove from filesystem after all imports updated and tests pass
- **Details:**
  - This step is LAST to preserve ability to diff old source if needed during build
  - Once deleted, `cargo check --all` should pass without reference errors
- **Verify:** `cargo check --all` (workspace clean with old dirs gone)

### Step 23: Final verification
- **Verify:**
  - `cargo test --workspace --lib` (all tests pass)
  - `cargo xtask fmt --check` (no formatting issues)
  - `cargo clippy --workspace` (no clippy warnings)
  - Workspace member count: exactly 120 (started with 122, removed 3, added 1)
  - Publish allowlist count: exactly 118 (started with 120, removed 3, added 1)

---

## Callers and Consumers

### `perl-diagnostics-codes` crate consumers:
- `perl-lsp-code-actions` (Cargo.toml dependency)
- `perl-lsp-diagnostics` (Cargo.toml dependency)
- `perl-lsp` (Cargo.toml dependency)
- `perl-lsp-diagnostic-catalog` (Cargo.toml dependency — being collapsed)

### `perl-lsp-diagnostic-types` crate consumers:
- `perl-lsp-diagnostics` (Cargo.toml dependency)
- (any others using `Diagnostic`, `RelatedInformation`, `DiagnosticSeverity`, `DiagnosticTag`)

### `perl-lsp-diagnostic-catalog` crate consumers:
- `perl-lsp` (Cargo.toml dependency)

### Functions from migrated modules:
- `DiagnosticCode` enum (codes module) — used in code-actions, diagnostics, LSP
- `DiagnosticSeverity` enum (codes module) — used widely
- `DiagnosticTag` enum (codes module) — used in diagnostic analysis
- `diagnostic_meta()` function (catalog module) — called from LSP diagnostic reporting
- `parse_error()`, `syntax_error()`, etc. (catalog module) — called from parser diagnostics

---

## Scope Boundary

### Files IN scope:
1. `/h/Code/Rust/perl-lsp/Cargo.toml` (workspace root)
2. `crates/perl-diagnostic-catalog/` (new crate — all files)
3. `crates/perl-lsp-code-actions/Cargo.toml` (dependency + imports)
4. `crates/perl-lsp-code-actions/src/lib.rs` (update imports)
5. `crates/perl-lsp-diagnostics/Cargo.toml` (dependency + imports)
6. `crates/perl-lsp-diagnostics/src/lib.rs` (update imports)
7. `crates/perl-lsp/Cargo.toml` (dependency + imports)
8. `crates/perl-lsp/src/lib.rs` (update imports, possibly other src files)
9. `crates/perl-diagnostics-codes/` (source — deleted at end)
10. `crates/perl-lsp-diagnostic-catalog/` (source — deleted at end)
11. `crates/perl-lsp-diagnostic-types/` (source — deleted at end)

### Files OUT of scope:
- `.spec/microcrate-collapse/ledger.md` (amendment is separate follow-up PR)
- Type unification implementation (deferred to v0.15.0)
- Any refactoring of diagnostic code logic (this is a move-only, no behavior change)
- Documentation updates beyond the new crate README (can be done in follow-up)

---

## Flags for Builder

### Ambiguities and decisions:

1. **`api.rs` re-export pattern is CRITICAL:**
   - Do NOT use wildcard re-exports (`pub use crate::codes::*;`)
   - Reason: `DiagnosticSeverity` and `DiagnosticTag` are defined in BOTH `codes/` and `types/` modules
   - Wildcard would create "ambiguous reexports" compile error
   - Solution: Explicitly list each symbol in `api.rs` (see Step 4 for pattern)
   - If you get ambiguity errors, re-read `api.rs` and expand the explicit list

2. **Inline tests at `perl-lsp-diagnostic-catalog/src/lib.rs:169-205`:**
   - These 4 inline tests must be moved to `crates/perl-diagnostic-catalog/tests/catalog_*.rs` test files
   - Check if there are inline tests with `#[cfg(test)]` in the original source
   - Move them to the new test files (already created in Step 16)

3. **Cross-module references in migrated code:**
   - When you copy source from 3 old crates into 3 modules of new crate, check for inter-module references
   - Example: If `catalog/mod.rs` references `DiagnosticCode` from `codes/`, change `perl_diagnostics_codes::DiagnosticCode` to `crate::codes::DiagnosticCode`
   - Use `grep -n "use perl_"` in each module to find all external imports that need updating

4. **Type duplication documentation:**
   - README.md should include note that `codes::DiagnosticSeverity` and `types::DiagnosticSeverity` are semantically identical
   - Mark in inline docs that types::DiagnosticSeverity is deprecated in favor of codes::DiagnosticSeverity
   - Will be unified in v0.15.0 (document this in README)

5. **Feature flags:**
   - Verify `[features]` section in new Cargo.toml includes `serde` feature (optional, behind feature gate)
   - Old crate has `serde` feature; new crate should too

6. **Test file naming:**
   - Use prefix convention to avoid name collisions: `codes_*.rs`, `catalog_*.rs`, `types_*.rs`
   - Ensures all 6 test files can coexist in `crates/perl-diagnostic-catalog/tests/`

7. **Workspace member order:**
   - When adding `"crates/perl-diagnostic-catalog",` to workspace members, insert in alphabetical order
   - Check current members list to find correct insertion point (likely after `"crates/perl-dead-code",`)

8. **Publish allowlist position:**
   - New crate sits in Tier 3 (analysis and indexing tier, alongside `perl-semantic-analyzer`, `perl-lsp-diagnostics`)
   - Insert after `perl-lsp-inlay-hints` and before `perl-module` to maintain tier coherence

---

## Test Coverage

**Expected test files in new crate:**
1. `tests/codes_comprehensive_unit_tests.rs` (from `perl-diagnostics-codes`)
2. `tests/codes_context_hint_tests.rs` (from `perl-diagnostics-codes`)
3. `tests/codes_diagnostic_code_completeness.rs` (from `perl-diagnostics-codes`)
4. `tests/catalog_coverage.rs` (from `perl-lsp-diagnostic-catalog`)
5. `tests/catalog_context_hint_tests.rs` (from `perl-lsp-diagnostic-catalog`)
6. `tests/types_comprehensive_unit_tests.rs` (from `perl-lsp-diagnostic-types`)

**Expected test count:** 6 test files migrated from 3 crates.

---

## Compilation Gates

- **Each step compiles**: Use `cargo check -p <crate>` or `cargo check --all` as specified
- **No unstaged changes**: Commit only spec files to this branch before handing to red-TDD
- **Final gate**: `cargo test --workspace --lib && cargo clippy --workspace`

