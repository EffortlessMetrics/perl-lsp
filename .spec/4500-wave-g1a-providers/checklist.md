# Wave G1a Implementation Checklist — Collapse 15 LSP Providers into perl-lsp-rs-core

**Issue:** #4500 | **Meta:** #4496 (v0.13.0 microcrate collapse) | **Branch:** `impl/4500-wave-g1a-providers`

**Scope:** Collapse 15 low-risk LSP provider crates into `perl_lsp_rs_core::providers::*` submodules. Reduces published crate count from 74 → 59. No feature additions, no behavioral changes — pure refactoring.

---

## PART 0 — Pre-Implementation Setup

### Step 0.1: Record Test Baseline
Command:
```bash
cargo test --workspace --lib 2>&1 | grep "test result: ok" | wc -l > /tmp/g1a_baseline.txt
cat /tmp/g1a_baseline.txt  # Record this number — post-G1a test count must be ≥ this
```

Expected: A number like 180–200 (exact count varies by recent changes).

### Step 0.2: Verify All 15 Source Crates Exist
Verify each source crate has `Cargo.toml` and `src/lib.rs`:
```bash
for crate in completion-item file-completion code-lens document-highlight folding selection-range inlay-hints type-hierarchy formatting-types on-type-formatting color-provider symbol-query import-management document-links workspace-symbols; do
  test -f "crates/perl-lsp-$crate/Cargo.toml" && test -f "crates/perl-lsp-$crate/src/lib.rs" || echo "MISSING: perl-lsp-$crate"
done
```

Expected: No output (all 15 exist).

---

## PART 1 — Create Provider Module Structure

### Step 1.1: Create `crates/perl-lsp-rs-core/src/providers/mod.rs`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/providers/mod.rs` (CREATE)

**Content:** (See APPENDIX A for full scaffold)

Declares all 15 submodules in dependency order:
- Group 1 (helpers): `completion_item`, `symbol_query`
- Group 2 (consumers of Group 1): `file_completion`, `workspace_symbols`
- Group 3 (11 independents): `code_lens`, `color`, `document_highlight`, `document_links`, `folding`, `formatting_types`, `import_management`, `inlay_hints`, `on_type_formatting`, `selection_range`, `type_hierarchy`

**Verify:**
```bash
cargo check -p perl-lsp-rs-core --lib
```

Expected: 0 errors (module declaration only, no implementations yet).

### Step 1.2: Add `pub mod providers;` to `crates/perl-lsp-rs-core/src/lib.rs`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/lib.rs`

**Change:** Add line after existing module declarations (around line 100–150):
```rust
pub mod providers;
```

**Verify:**
```bash
cargo check -p perl-lsp-rs-core --lib
```

Expected: 0 errors.

---

## PART 2 — Migrate Group 1 (Helper Providers)

These crates provide utilities consumed by Group 2 consumers. Must be completed before Group 2.

### Step 2.1: Migrate `perl-lsp-completion-item` → `providers::completion_item`

**Step 2.1.1: Copy source tree**
```bash
mkdir -p crates/perl-lsp-rs-core/src/providers/completion_item
cp crates/perl-lsp-completion-item/src/lib.rs crates/perl-lsp-rs-core/src/providers/completion_item/mod.rs
```

**Step 2.1.2: Copy test files to root `crates/perl-lsp-rs-core/tests/`**
```bash
# Check if tests/dedup_sort.rs exists
test -f crates/perl-lsp-completion-item/tests/dedup_sort.rs && \
  cp crates/perl-lsp-completion-item/tests/dedup_sort.rs \
     crates/perl-lsp-rs-core/tests/provider_completion_item_dedup_sort.rs
```

**Step 2.1.3: Update import paths in migrated test file**
**File:** `crates/perl-lsp-rs-core/tests/provider_completion_item_dedup_sort.rs`

Find and replace all:
- `use perl_lsp_completion_item::` → `use perl_lsp_rs_core::providers::completion_item::`

Example:
```rust
// Before
use perl_lsp_completion_item::{CompletionItem, deduplicate_and_sort};
// After
use perl_lsp_rs_core::providers::completion_item::{CompletionItem, deduplicate_and_sort};
```

**Step 2.1.4: Update inline test imports in mod.rs**
**File:** `crates/perl-lsp-rs-core/src/providers/completion_item/mod.rs`

Any `#[cfg(test)]` blocks that import from `crate::` should use `super::` or fully-qualified paths. Typically no changes needed if tests use relative imports like `use crate::completion_item::*`.

**Step 2.1.5: Copy dependencies to `perl-lsp-rs-core/Cargo.toml`**

Read `crates/perl-lsp-completion-item/Cargo.toml`. Extract non-workspace, non-path dependencies. Add to `crates/perl-lsp-rs-core/Cargo.toml` `[dependencies]` section if not already present. Expected additions: none new (completion_item is lightweight).

**Verify:**
```bash
cargo check -p perl-lsp-rs-core --lib
cargo test -p perl-lsp-rs-core --lib  # Should compile inline tests
```

Expected: 0 errors, inline tests from `completion_item/mod.rs` pass.

---

### Step 2.2: Migrate `perl-lsp-symbol-query` → `providers::symbol_query`

Follow same pattern as Step 2.1:

**Step 2.2.1: Copy source**
```bash
mkdir -p crates/perl-lsp-rs-core/src/providers/symbol_query
cp crates/perl-lsp-symbol-query/src/lib.rs crates/perl-lsp-rs-core/src/providers/symbol_query/mod.rs
```

**Step 2.2.2: Copy test files**
```bash
test -f crates/perl-lsp-symbol-query/tests/mutation_killing.rs && \
  cp crates/perl-lsp-symbol-query/tests/mutation_killing.rs \
     crates/perl-lsp-rs-core/tests/provider_symbol_query_mutation.rs
```

**Step 2.2.3: Update imports in test file**
`crates/perl-lsp-rs-core/tests/provider_symbol_query_mutation.rs`:
- `use perl_lsp_symbol_query::` → `use perl_lsp_rs_core::providers::symbol_query::`

**Step 2.2.4: Copy dependencies**
Read `crates/perl-lsp-symbol-query/Cargo.toml`. Add new deps to `perl-lsp-rs-core/Cargo.toml`.

**Verify:**
```bash
cargo check -p perl-lsp-rs-core --lib
cargo test -p perl-lsp-rs-core --lib
```

Expected: 0 errors, both Group 1 helper modules present and testable.

### Step 2.3: Gate after Group 1 completion
```bash
cargo check -p perl-lsp-rs-core --lib
```

Expected: 0 errors. Group 1 helpers are now fully visible as submodules.

---

## PART 3 — Migrate Group 2 (Consumer Providers)

These depend on Group 1. Intra-module imports must be rewritten to use `crate::providers::HELPER`.

### Step 3.1: Migrate `perl-lsp-file-completion` → `providers::file_completion`

**Step 3.1.1: Copy source**
```bash
mkdir -p crates/perl-lsp-rs-core/src/providers/file_completion
cp crates/perl-lsp-file-completion/src/lib.rs crates/perl-lsp-rs-core/src/providers/file_completion/mod.rs
```

**Step 3.1.2: Rewrite intra-provider imports**
**File:** `crates/perl-lsp-rs-core/src/providers/file_completion/mod.rs`

Find and replace:
- `use perl_lsp_completion_item::` → `use crate::providers::completion_item::`
- `use perl_lsp_file_completion::` → `use crate::providers::file_completion::` (for self-tests)

Example:
```rust
// Before
use perl_lsp_completion_item::{CompletionItem, CompletionItemKind};
// After
use crate::providers::completion_item::{CompletionItem, CompletionItemKind};
```

**Step 3.1.3: Copy test files**
```bash
test -f crates/perl-lsp-file-completion/tests/comprehensive_unit_tests.rs && \
  cp crates/perl-lsp-file-completion/tests/comprehensive_unit_tests.rs \
     crates/perl-lsp-rs-core/tests/provider_file_completion_comprehensive.rs
```

**Step 3.1.4: Update imports in test file**
`crates/perl-lsp-rs-core/tests/provider_file_completion_comprehensive.rs`:
- `use perl_lsp_file_completion::` → `use perl_lsp_rs_core::providers::file_completion::`
- `use perl_lsp_completion_item::` → `use perl_lsp_rs_core::providers::completion_item::`

**Step 3.1.5: Copy dependencies**
Read `crates/perl-lsp-file-completion/Cargo.toml`. Add new deps to `perl-lsp-rs-core/Cargo.toml`.

**Verify:**
```bash
cargo check -p perl-lsp-rs-core --lib
cargo test -p perl-lsp-rs-core --lib
```

Expected: 0 errors. `file_completion` module compiles and imports from `completion_item` submodule.

---

### Step 3.2: Migrate `perl-lsp-workspace-symbols` → `providers::workspace_symbols`

Follow same pattern as Step 3.1:

**Step 3.2.1: Copy source**
```bash
mkdir -p crates/perl-lsp-rs-core/src/providers/workspace_symbols
cp crates/perl-lsp-workspace-symbols/src/lib.rs crates/perl-lsp-rs-core/src/providers/workspace_symbols/mod.rs
```

**Step 3.2.2: Rewrite intra-provider imports**
`crates/perl-lsp-rs-core/src/providers/workspace_symbols/mod.rs`:
- `use perl_lsp_symbol_query::` → `use crate::providers::symbol_query::`

**Step 3.2.3: Copy test files**
```bash
test -f crates/perl-lsp-workspace-symbols/tests/edge_cases.rs && \
  cp crates/perl-lsp-workspace-symbols/tests/edge_cases.rs \
     crates/perl-lsp-rs-core/tests/provider_workspace_symbols_edge_cases.rs
```

**Step 3.2.4: Update imports in test file**
`crates/perl-lsp-rs-core/tests/provider_workspace_symbols_edge_cases.rs`:
- `use perl_lsp_workspace_symbols::` → `use perl_lsp_rs_core::providers::workspace_symbols::`
- `use perl_lsp_symbol_query::` → `use perl_lsp_rs_core::providers::symbol_query::`

**Step 3.2.5: Copy dependencies**
Read `crates/perl-lsp-workspace-symbols/Cargo.toml`. Add new deps to `perl-lsp-rs-core/Cargo.toml`.

**Verify:**
```bash
cargo check -p perl-lsp-rs-core --lib
cargo test -p perl-lsp-rs-core --lib
```

Expected: 0 errors. `workspace_symbols` module compiles and imports from `symbol_query` submodule.

---

## PART 4 — Migrate Group 3 (11 Independent Providers)

No inter-provider dependencies. Can be done in any order. Process one at a time to verify each step.

### Step 4.1–4.11: Template for Independent Providers

For each crate in this list (in any order):
1. `perl-lsp-code-lens` → `providers::code_lens`
2. `perl-lsp-document-highlight` → `providers::document_highlight`
3. `perl-lsp-folding` → `providers::folding` (2 test files)
4. `perl-lsp-selection-range` → `providers::selection_range` (no test files)
5. `perl-lsp-inlay-hints` → `providers::inlay_hints` (2 test files)
6. `perl-lsp-type-hierarchy` → `providers::type_hierarchy`
7. `perl-lsp-formatting-types` → `providers::formatting_types`
8. `perl-lsp-on-type-formatting` → `providers::on_type_formatting` (2 test files)
9. `perl-lsp-color-provider` → `providers::color`
10. `perl-lsp-import-management` → `providers::import_management` (2 test files)
11. `perl-lsp-document-links` → `providers::document_links` (2 test files)

**Per-crate steps:**

**4.X.1: Copy source**
```bash
CRATE="code-lens"  # Change for each iteration
MODULE=$(echo $CRATE | sed 's/-/_/g')
mkdir -p crates/perl-lsp-rs-core/src/providers/$MODULE
cp crates/perl-lsp-$CRATE/src/lib.rs crates/perl-lsp-rs-core/src/providers/$MODULE/mod.rs
```

**4.X.2: Copy any test files**
```bash
# For each tests/*.rs in crates/perl-lsp-$CRATE/tests/
for test_file in crates/perl-lsp-$CRATE/tests/*.rs; do
  if [ -f "$test_file" ]; then
    test_name=$(basename "$test_file" .rs)
    cp "$test_file" "crates/perl-lsp-rs-core/tests/provider_${MODULE}_${test_name}.rs"
  fi
done
```

**4.X.3: Update all imports in migrated source and tests**
- Replace `use perl_lsp_$CRATE::` → `use perl_lsp_rs_core::providers::$MODULE::`
- No intra-module deps (Group 3 are independent)

**4.X.4: Copy dependencies to `perl-lsp-rs-core/Cargo.toml`**
Read `crates/perl-lsp-$CRATE/Cargo.toml`. Add new deps if not already present.

**4.X.5: Verify per iteration**
```bash
cargo check -p perl-lsp-rs-core --lib
cargo test -p perl-lsp-rs-core --lib
```

Expected: 0 errors after each migration.

**See APPENDIX B for detailed mapping of all 11 crates and their test files.**

---

## PART 5 — Update Consumer Crates

Six crates import from G1a. Must update `Cargo.toml` and import sites.

### Step 5.1: Update `crates/perl-lsp/Cargo.toml`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp/Cargo.toml`

**Changes:**
- Remove these 12 direct G1a deps (or comment them out if uncertain):
  - `perl-lsp-code-lens`
  - `perl-lsp-document-highlight`
  - `perl-lsp-folding`
  - `perl-lsp-inlay-hints`
  - `perl-lsp-type-hierarchy`
  - `perl-lsp-formatting-types`
  - `perl-lsp-on-type-formatting`
  - `perl-lsp-color-provider`
  - `perl-lsp-symbol-query`
  - `perl-lsp-completion-item`
  - `perl-lsp-import-management` (if present)
  - `perl-lsp-document-links` (if present)

- Ensure `perl-lsp-rs-core` is listed as a dep (added in Wave F; should already be present).

**Before (approximate):**
```toml
perl-lsp-code-lens = { path = "../perl-lsp-code-lens", version = "..." }
perl-lsp-folding = { path = "../perl-lsp-folding", version = "..." }
...
perl-lsp-rs-core = { path = "../perl-lsp-rs-core", version = "..." }
```

**After:**
```toml
perl-lsp-rs-core = { path = "../perl-lsp-rs-core", version = "..." }
# (other deps unchanged)
```

**Verify line count:** `wc -l crates/perl-lsp/Cargo.toml` should decrease by ~12 lines.

### Step 5.2: Update `crates/perl-lsp/src/` import sites
**File(s):** All files under `crates/perl-lsp/src/` that import G1a crates

**Files identified by plan-reviewer:**
- `src/features/code_lens_provider.rs:5`
- `src/features/document_highlight.rs:5`
- `src/features/inlay_hints.rs:2`
- `src/features/lsp_selection_range.rs:2`
- `src/features/type_hierarchy.rs:2`
- `src/runtime/language/colors.rs:5`
- `src/runtime/workspace.rs:466`

**Change pattern:** 
```rust
// Before
use perl_lsp_code_lens::CodeLensProvider;
// After
use perl_lsp_rs_core::providers::code_lens::CodeLensProvider;
```

**Verification:** After updates, each file should compile:
```bash
cargo check --lib -p perl-lsp
```

Expected: 0 errors.

### Step 5.3: Update `crates/perl-lsp-completion/Cargo.toml`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-completion/Cargo.toml`

**Changes:**
- Remove `perl-lsp-completion-item` and `perl-lsp-file-completion` deps
- Ensure `perl-lsp-rs-core` present

**Update import sites in `crates/perl-lsp-completion/src/`:**
- `src/completion/file_path.rs:5`
- `src/completion/items.rs:5`
- `src/completion/sort.rs:5`

Pattern: `use perl_lsp_completion_item::` → `use perl_lsp_rs_core::providers::completion_item::`

### Step 5.4: Update `crates/perl-lsp-code-actions/Cargo.toml`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-code-actions/Cargo.toml`

**Changes:**
- Remove `perl-lsp-import-management` dep
- Ensure `perl-lsp-rs-core` present

**Update import sites in `crates/perl-lsp-code-actions/src/`:**
- `src/enhanced/import_management.rs:4,93`

Pattern: `use perl_lsp_import_management::` → `use perl_lsp_rs_core::providers::import_management::`

### Step 5.5: Update `crates/perl-lsp-formatting/Cargo.toml`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-formatting/Cargo.toml`

**Changes:**
- Remove `perl-lsp-formatting-types` dep
- Ensure `perl-lsp-rs-core` present

**Update import sites in `crates/perl-lsp-formatting/src/`:**
- `src/formatting.rs:3`

Pattern: `use perl_lsp_formatting_types::` → `use perl_lsp_rs_core::providers::formatting_types::`

### Step 5.6: Update `crates/perl-lsp-navigation/Cargo.toml`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-navigation/Cargo.toml`

**Changes:**
- Remove `perl-lsp-document-links`, `perl-lsp-type-hierarchy`, `perl-lsp-workspace-symbols` deps
- Ensure `perl-lsp-rs-core` present

**Update import sites in `crates/perl-lsp-navigation/src/`:**
- `src/lib.rs:36-40`

Pattern:
- `use perl_lsp_document_links::` → `use perl_lsp_rs_core::providers::document_links::`
- `use perl_lsp_type_hierarchy::` → `use perl_lsp_rs_core::providers::type_hierarchy::`
- `use perl_lsp_workspace_symbols::` → `use perl_lsp_rs_core::providers::workspace_symbols::`

### Step 5.7: Update `crates/perl-lsp-providers/Cargo.toml`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-providers/Cargo.toml`

**Changes:**
- Remove `perl-lsp-on-type-formatting`, `perl-lsp-inlay-hints`, `perl-lsp-code-lens`, `perl-lsp-folding` deps
- Ensure `perl-lsp-rs-core` present

**Update import sites in `crates/perl-lsp-providers/src/`:**
- `src/ide/lsp_compat/folding.rs:5`
- `src/ide/lsp_compat/on_type_formatting.rs:6`
- `src/ide/lsp_compat/code_lens_provider.rs:18`
- `src/lib.rs:93,118`

Pattern:
- `use perl_lsp_on_type_formatting::` → `use perl_lsp_rs_core::providers::on_type_formatting::`
- `use perl_lsp_inlay_hints::` → `use perl_lsp_rs_core::providers::inlay_hints::`
- `use perl_lsp_code_lens::` → `use perl_lsp_rs_core::providers::code_lens::`
- `use perl_lsp_folding::` → `use perl_lsp_rs_core::providers::folding::`

**Verify all consumer crates compile:**
```bash
cargo check --workspace --lib
```

Expected: 0 errors across all 6 consumer crates.

---

## PART 6 — Update Test Registry

### Step 6.1: Update `crates/perl-lsp/tests/wired_crates_integration_test.rs`
**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp/tests/wired_crates_integration_test.rs`

This file has 6 G1a imports. Rewrite exactly as follows:

| Current line | Replace with |
|---|---|
| `use perl_lsp_workspace_symbols::WorkspaceSymbolsProvider;` | `use perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolsProvider;` |
| `use perl_lsp_symbol_query::matches_query;` | `use perl_lsp_rs_core::providers::symbol_query::matches_query;` |
| `use perl_lsp_completion_item::{CompletionItem, CompletionItemKind, deduplicate_and_sort};` | `use perl_lsp_rs_core::providers::completion_item::{CompletionItem, CompletionItemKind, deduplicate_and_sort};` |
| `use perl_lsp_formatting_types::FormatRange;` | `use perl_lsp_rs_core::providers::formatting_types::FormatRange;` |
| `use perl_lsp_import_management::collect_imports;` | `use perl_lsp_rs_core::providers::import_management::collect_imports;` |
| `use perl_lsp_document_links::compute_links;` | `use perl_lsp_rs_core::providers::document_links::compute_links;` |

**Verify no stale imports remain:**
```bash
grep "perl_lsp_completion_item\|perl_lsp_symbol_query\|perl_lsp_workspace_symbols\|perl_lsp_formatting_types\|perl_lsp_import_management\|perl_lsp_document_links" crates/perl-lsp/tests/wired_crates_integration_test.rs | wc -l
```

Expected: `0` (all old crate names removed).

**Verify test passes:**
```bash
cargo test -p perl-lsp-rs -- wired_crates
```

Expected: All tests pass.

---

## PART 7 — Update xtask and Configuration Files

### Step 7.1: Update `xtask/src/tasks/build_timing.rs`
**File:** `/h/Code/Rust/perl-lsp/xtask/src/tasks/build_timing.rs`

Check if this file lists individual crate build times. If it includes any of the 15 G1a crates, remove them.

Command to find:
```bash
grep -n "perl-lsp-completion-item\|perl-lsp-file-completion\|perl-lsp-code-lens\|perl-lsp-document-highlight\|perl-lsp-folding\|perl-lsp-selection-range\|perl-lsp-inlay-hints\|perl-lsp-type-hierarchy\|perl-lsp-formatting-types\|perl-lsp-on-type-formatting\|perl-lsp-color-provider\|perl-lsp-symbol-query\|perl-lsp-import-management\|perl-lsp-document-links\|perl-lsp-workspace-symbols" xtask/src/tasks/build_timing.rs
```

If matches found, remove those crate references and add `perl-lsp-rs-core` if not already present.

### Step 7.2: Update `xtask/src/tasks/targeted_checks.rs`
**File:** `/h/Code/Rust/perl-lsp/xtask/src/tasks/targeted_checks.rs`

Same as Step 7.1. Check if this file references any G1a crates and remove them.

### Step 7.3: Update `xtask/published-crate-baseline.txt`
**File:** `/h/Code/Rust/perl-lsp/xtask/published-crate-baseline.txt`

Remove these 15 crate names (one per line):
```
perl-lsp-completion-item
perl-lsp-file-completion
perl-lsp-code-lens
perl-lsp-document-highlight
perl-lsp-folding
perl-lsp-selection-range
perl-lsp-inlay-hints
perl-lsp-type-hierarchy
perl-lsp-formatting-types
perl-lsp-on-type-formatting
perl-lsp-color-provider
perl-lsp-symbol-query
perl-lsp-import-management
perl-lsp-document-links
perl-lsp-workspace-symbols
```

**Verify count:**
```bash
wc -l xtask/published-crate-baseline.txt
```

Expected: 59 (was 74 before G1a).

### Step 7.4: Update `Cargo.toml` root `[workspace.metadata.publish.allow]`
**File:** `/h/Code/Rust/perl-lsp/Cargo.toml`

Find section `[workspace.metadata.publish.allow]`. Remove the same 15 crate names as Step 7.3.

**Verify:**
```bash
cargo metadata --no-deps | jq '.workspace_metadata.publish.allow | length'
```

Expected: 59.

### Step 7.5: Update `scripts/verify-docs-rs.sh`
**File:** `/h/Code/Rust/perl-lsp/scripts/verify-docs-rs.sh`

Check if this script lists crates for docs.rs validation. Remove the 15 G1a crates if present.

Command to find:
```bash
grep -c "perl-lsp-completion-item" scripts/verify-docs-rs.sh
```

If > 0, manually edit and remove those crate references.

---

## PART 8 — Delete Source Crate Directories

Only after all above steps are complete and verified:

```bash
for crate in completion-item file-completion code-lens document-highlight folding selection-range inlay-hints type-hierarchy formatting-types on-type-formatting color-provider symbol-query import-management document-links workspace-symbols; do
  rm -rf crates/perl-lsp-$crate
done
```

**Verify deletion:**
```bash
for crate in completion-item file-completion code-lens document-highlight folding selection-range inlay-hints type-hierarchy formatting-types on-type-formatting color-provider symbol-query import-management document-links workspace-symbols; do
  test -d crates/perl-lsp-$crate && echo "FAILED: perl-lsp-$crate still exists" || echo "✓ perl-lsp-$crate deleted"
done
```

Expected: All 15 deleted successfully.

---

## PART 9 — Final Verification

### Step 9.1: Compilation Gates
```bash
cargo check --workspace --lib
cargo test -p perl-lsp-rs-core --lib
```

Expected: 0 errors.

### Step 9.2: Test Baseline Comparison
```bash
cargo test --workspace --lib 2>&1 | grep "test result: ok" | wc -l
```

Expected: ≥ baseline from Step 0.1 (no test loss).

### Step 9.3: wired_crates_integration_test Verification
```bash
grep -c "perl_lsp_completion_item\|perl_lsp_symbol_query\|perl_lsp_workspace_symbols\|perl_lsp_formatting_types\|perl_lsp_import_management\|perl_lsp_document_links" crates/perl-lsp/tests/wired_crates_integration_test.rs
```

Expected: `0` (all old crate names removed).

**Run the test:**
```bash
cargo test -p perl-lsp-rs -- wired_crates
```

Expected: All pass.

### Step 9.4: Linting and Formatting
```bash
cargo clippy -p perl-lsp-rs-core --all-targets
cargo clippy --workspace --lib
cargo xtask fmt
```

Expected: 0 errors, no format changes needed.

### Step 9.5: Layer Check
```bash
cargo xtask layer-check
```

Expected: 0 errors (perl-lsp-rs-core sits cleanly below consumers).

### Step 9.6: Published Crate Count Check
```bash
cargo xtask published-crate-count-check
```

Expected: 59 (from baseline file).

### Step 9.7: Full CI Gate (local)
```bash
just pr-fast
```

Expected: All checks pass.

---

## APPENDIX A — `providers/mod.rs` Scaffold

**File:** `crates/perl-lsp-rs-core/src/providers/mod.rs`

```rust
//! LSP provider implementations (Wave G1a: 15 low-risk provider crates absorbed).
//!
//! This module contains the implementation of all LSP protocol providers previously
//! distributed across 15 separate crates. Structured in groups by dependency order:
//! - Group 1: Helper utilities (completion_item, symbol_query)
//! - Group 2: Consumers of Group 1 (file_completion, workspace_symbols)
//! - Group 3: Independent providers (11 others)

// Group 1 -- helpers (no inter-provider dependencies)
pub mod completion_item;
pub mod symbol_query;

// Group 2 -- consumers of Group 1 helpers
pub mod file_completion;
pub mod workspace_symbols;

// Group 3 -- independent providers
pub mod code_lens;
pub mod color;
pub mod document_highlight;
pub mod document_links;
pub mod folding;
pub mod formatting_types;
pub mod import_management;
pub mod inlay_hints;
pub mod on_type_formatting;
pub mod selection_range;
pub mod type_hierarchy;
```

---

## APPENDIX B — Group 3 Test File Mapping

| Source crate | Module | Tests to migrate |
|---|---|---|
| `perl-lsp-code-lens` | `code_lens` | `code_lens_tests.rs` |
| `perl-lsp-document-highlight` | `document_highlight` | `document_highlight_tests.rs` |
| `perl-lsp-folding` | `folding` | `ast_folding.rs`, `heredoc_folding.rs` |
| `perl-lsp-selection-range` | `selection_range` | (none — inline tests only) |
| `perl-lsp-inlay-hints` | `inlay_hints` | `comprehensive_unit_tests.rs`, `inlay_hints_extended_tests.rs` |
| `perl-lsp-type-hierarchy` | `type_hierarchy` | `type_hierarchy_coverage.rs` |
| `perl-lsp-formatting-types` | `formatting_types` | `comprehensive_unit_tests.rs` |
| `perl-lsp-on-type-formatting` | `on_type_formatting` | `on_type_formatting_tests.rs`, `tab_size_and_pod_tests.rs` |
| `perl-lsp-color-provider` | `color` | `color_provider_tests.rs` |
| `perl-lsp-import-management` | `import_management` | `import_management_tests.rs`, `mutation_killing.rs` |
| `perl-lsp-document-links` | `document_links` | `pragma_coverage.rs`, `require_mutation_coverage.rs` |

---

## Success Criteria

- [ ] All 15 crate sources migrated to submodules under `perl_lsp_rs_core::providers`
- [ ] All 20 test files copied and import paths updated
- [ ] All 6 consumer crates updated (Cargo.toml + import sites)
- [ ] `wired_crates_integration_test.rs` patched (6 imports rewritten)
- [ ] All 15 source crate directories deleted
- [ ] `cargo check --workspace --lib` → 0 errors
- [ ] `cargo test --workspace --lib` → baseline count maintained
- [ ] `cargo xtask layer-check` → 0 errors
- [ ] `just pr-fast` → green
- [ ] `xtask/published-crate-baseline.txt` contains `59`

