# Wave G1b Provider Collapse — Implementation Checklist

**Issue:** #4501  
**Branch:** `impl/4501-wave-g1b-providers`  
**Base:** origin/master (2ef0dad1e, post-G1a)  
**Target:** 10 G1b crates → `perl-lsp-rs-core::providers::*`  
**Crate count:** 59 → 49 published  
**Status:** builder-ready (all 6 pre-plan-review layers + plan-reviewed signed off)

---

## Phase 1: Pure Leaves (no G1b intra-dependencies)

These 4 crates have no inter-dependencies with other G1b crates.

### Step 1.1: Absorb `perl-lsp-rename` → `providers::rename`

**Dependencies:**
- Must complete before Phase 3 (code_actions depends on rename)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/rename/` directory
2. Move all `.rs` files from `crates/perl-lsp-rename/src/` to `crates/perl-lsp-rs-core/src/providers/rename/`
   - Exact: `crates/perl-lsp-rename/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/rename/mod.rs`
   - All other `*.rs` files in `src/` → corresponding files in `providers/rename/`
3. Update internal crate paths in moved files: `use perl_lsp_rename::` → `use crate::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod rename;
   pub use rename::*;
   ```
5. Delete `crates/perl-lsp-rename/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

### Step 1.2: Absorb `perl-lsp-diagnostics` → `providers::diagnostics` (with snapshot migration)

**Dependencies:**
- Must complete before Phase 3 (code_actions depends on diagnostics)
- Snapshot migration required (see O3 protocol below)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/diagnostics/` directory
2. Move all `.rs` files from `crates/perl-lsp-diagnostics/src/` to `crates/perl-lsp-rs-core/src/providers/diagnostics/`
   - Exact: `crates/perl-lsp-diagnostics/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/diagnostics/mod.rs`
3. Update internal crate paths in moved files: `use perl_lsp_diagnostics::` → `use crate::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod diagnostics;
   pub use diagnostics::*;
   ```
5. **Snapshot migration (CRITICAL — see O3 protocol):**
   - Create `/c/wt4501/crates/perl-lsp-rs-core/tests/snapshots/` directory (if not exists)
   - Copy these 4 `.snap` files BYTE-IDENTICAL to new location:
     ```
     crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__missing_pragmas_and_unused_variable.snap
     → crates/perl-lsp-rs-core/tests/snapshots/diag_snap__missing_pragmas_and_unused_variable.snap
     
     crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__package_module_happy_path.snap
     → crates/perl-lsp-rs-core/tests/snapshots/diag_snap__package_module_happy_path.snap
     
     crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__script_happy_path.snap
     → crates/perl-lsp-rs-core/tests/snapshots/diag_snap__script_happy_path.snap
     
     crates/perl-lsp-diagnostics/tests/snapshots/diag_snap__security_string_eval.snap
     → crates/perl-lsp-rs-core/tests/snapshots/diag_snap__security_string_eval.snap
     ```
   - **Verification:** Use `cmp -l` or `diff --binary` to byte-verify copy success before deletion
   - Migrate test file: `crates/perl-lsp-diagnostics/tests/diag_snap.rs` → `crates/perl-lsp-rs-core/tests/diag_snap.rs`
     - Update test module import: `use perl_lsp_diagnostics::` → `use perl_lsp_rs_core::providers::diagnostics::`
     - Run `cargo test -p perl-lsp-rs-core diag_snap` — tests must pass without `cargo insta review` (snapshots match existing content)
6. Delete `crates/perl-lsp-diagnostics/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core diag_snap
```

**O3 Protocol Note:**
- Do NOT run `cargo insta review --accept` on migrated snapshots
- Each snapshot file must match byte-for-byte with original before deletion
- If any snapshot differs, this is a regression — investigate before proceeding
- PR body must include: "Migrated 4 diagnostics snapshots; content verified byte-identical to pre-G1b content."

---

### Step 1.3: Absorb `perl-lsp-inline-completion` → `providers::inline_completion`

**Dependencies:**
- Depends on Phase 2 (ai provider is in Phase 2)
- Must complete before Phase 2 (ai provider depends on inline_completion)
- Actually: MOVE THIS TO PHASE 2 AFTER AI (no, ai depends on inline-completion, so inline-completion FIRST)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/inline_completion/` directory
2. Move all `.rs` files from `crates/perl-lsp-inline-completion/src/` to `crates/perl-lsp-rs-core/src/providers/inline_completion/`
   - Exact: `crates/perl-lsp-inline-completion/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/inline_completion/mod.rs`
3. Update internal crate paths: `use perl_lsp_inline_completion::` → `use crate::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod inline_completion;
   pub use inline_completion::*;
   ```
5. Delete `crates/perl-lsp-inline-completion/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

### Step 1.4: Absorb `perl-lsp-semantic-tokens` → `providers::semantic_tokens`

**Dependencies:**
- None (pure leaf)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/semantic_tokens/` directory
2. Move all `.rs` files from `crates/perl-lsp-semantic-tokens/src/` to `crates/perl-lsp-rs-core/src/providers/semantic_tokens/`
   - Exact: `crates/perl-lsp-semantic-tokens/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/semantic_tokens/mod.rs`
3. Update internal crate paths: `use perl_lsp_semantic_tokens::` → `use crate::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod semantic_tokens;
   pub use semantic_tokens::*;
   ```
5. Delete `crates/perl-lsp-semantic-tokens/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

## Phase 2: Near-Leaves (G1a dependencies only, no G1b cross-deps)

### Step 2.1: Absorb `perl-lsp-formatting` → `providers::formatting`

**Dependencies:**
- Depends on `perl-lsp-formatting-types` (G1a, already in perl-lsp-rs-core)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/formatting/` directory
2. Move all `.rs` files from `crates/perl-lsp-formatting/src/` to `crates/perl-lsp-rs-core/src/providers/formatting/`
   - Exact: `crates/perl-lsp-formatting/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/formatting/mod.rs`
3. Update crate paths in moved files:
   - `use perl_lsp_formatting::` → `use crate::`
   - `perl_lsp_formatting_types::` → already in `crate::providers::formatting_types::`, update refs
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod formatting;
   pub use formatting::*;
   ```
5. Delete `crates/perl-lsp-formatting/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

### Step 2.2: Absorb `perl-lsp-ai-provider` → `providers::ai`

**Dependencies:**
- Depends on `perl-lsp-inline-completion` (completed in Phase 1, Step 1.3)
- Note: NO feature gates in Cargo.toml (research-verifier correction) — absorb as-is

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/ai/` directory
2. Move all `.rs` files from `crates/perl-lsp-ai-provider/src/` to `crates/perl-lsp-rs-core/src/providers/ai/`
   - Exact: `crates/perl-lsp-ai-provider/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/ai/mod.rs`
3. Update crate paths in moved files:
   - `use perl_lsp_ai_provider::` → `use crate::`
   - `perl_lsp_inline_completion::` → `crate::providers::inline_completion::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod ai;
   pub use ai::*;
   ```
5. Delete `crates/perl-lsp-ai-provider/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

## Phase 3: Consumers (depend on Phase 1 + Phase 2)

### Step 3.1: Absorb `perl-lsp-completion` → `providers::completion`

**Dependencies:**
- Depends on `perl-lsp-completion-item` + `perl-lsp-file-completion` (both G1a, in perl-lsp-rs-core)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/completion/` directory
2. Move all `.rs` files from `crates/perl-lsp-completion/src/` to `crates/perl-lsp-rs-core/src/providers/completion/`
   - Exact: `crates/perl-lsp-completion/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/completion/mod.rs`
3. Update crate paths:
   - `use perl_lsp_completion::` → `use crate::`
   - `perl_lsp_completion_item::` → `crate::providers::completion_item::`
   - `perl_lsp_file_completion::` → `crate::providers::file_completion::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod completion;
   pub use completion::*;
   ```
5. Delete `crates/perl-lsp-completion/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

### Step 3.2: Absorb `perl-lsp-navigation` → `providers::navigation`

**Dependencies:**
- Depends on 3 G1a crates (already in perl-lsp-rs-core)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/navigation/` directory
2. Move all `.rs` files from `crates/perl-lsp-navigation/src/` to `crates/perl-lsp-rs-core/src/providers/navigation/`
   - Exact: `crates/perl-lsp-navigation/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/navigation/mod.rs`
3. Update crate paths:
   - `use perl_lsp_navigation::` → `use crate::`
   - G1a crate refs → `crate::providers::<module>::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod navigation;
   pub use navigation::*;
   ```
5. Delete `crates/perl-lsp-navigation/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

### Step 3.3: Absorb `perl-lsp-code-actions` → `providers::code_actions`

**Dependencies:**
- Depends on `perl-lsp-diagnostics` (Phase 1, Step 1.2) ✓
- Depends on `perl-lsp-rename` (Phase 1, Step 1.1) ✓
- Depends on `perl-lsp-import-management` (G1a, in perl-lsp-rs-core)

**Changes:**
1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/code_actions/` directory
2. Move all `.rs` files from `crates/perl-lsp-code-actions/src/` to `crates/perl-lsp-rs-core/src/providers/code_actions/`
   - Exact: `crates/perl-lsp-code-actions/src/lib.rs` → `crates/perl-lsp-rs-core/src/providers/code_actions/mod.rs`
3. Update crate paths:
   - `use perl_lsp_code_actions::` → `use crate::`
   - `perl_lsp_diagnostics::` → `crate::providers::diagnostics::`
   - `perl_lsp_rename::` → `crate::providers::rename::`
   - `perl_lsp_import_management::` → `crate::providers::import_management::`
4. Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod code_actions;
   pub use code_actions::*;
   ```
5. Delete `crates/perl-lsp-code-actions/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

## Phase 4: Aggregator (LAST — largest piece, ~1,750 LOC)

### Step 4.1: Absorb `perl-lsp-providers` (original code) → `providers::lsp_compat` (NEW MODULE)

**Critical Correction (O5):**
`perl-lsp-providers` is NOT a pure aggregator. It contains ~1,600 LOC of original implementations in `src/ide/lsp_compat/`. This code must go to `providers::lsp_compat`, NOT `providers::registry`.

**Dependencies:**
- All 9 other G1b crates must be absorbed first (Phases 1–3) ✓

**Changes:**

**A. Create `providers::lsp_compat` module for original code:**

1. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/lsp_compat/` directory
2. Copy `crates/perl-lsp-providers/src/ide/lsp_compat/*` → `crates/perl-lsp-rs-core/src/providers/lsp_compat/`
   - Exact file copies:
     ```
     signature_help.rs → signature_help.rs
     linked_editing.rs → linked_editing.rs
     selection_range.rs → selection_range.rs
     on_type_formatting.rs → on_type_formatting.rs
     folding.rs → folding.rs
     (and all other .rs files in lsp_compat/)
     ```
3. Update crate paths in `lsp_compat/*.rs`:
   - `use perl_lsp_providers::` → internal references now use `crate::` or relative paths
   - External crate refs (e.g., to providers absorbed above) → `crate::providers::<module>::`
4. Create `/c/wt4501/crates/perl-lsp-rs-core/src/providers/lsp_compat/mod.rs` to export all submodules
5. Update `crates/perl-lsp-rs-core/src/providers/mod.rs`:
   ```rust
   pub mod lsp_compat;
   pub use lsp_compat::*;  // Re-export lsp_compat public surface
   ```

**B. Create module-level re-exports for aggregated providers:**

Update `crates/perl-lsp-rs-core/src/providers/mod.rs` to re-export the 9 absorbed providers plus the new lsp_compat:

```rust
// Re-exports of the 9 collapsed G1b providers
pub use rename::*;
pub use diagnostics::*;
pub use inline_completion::*;
pub use semantic_tokens::*;
pub use formatting::*;
pub use ai::*;
pub use completion::*;
pub use navigation::*;
pub use code_actions::*;

// Original lsp_compat implementations
pub use lsp_compat::*;

// Already present from G1a
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
```

**C. Preserve deprecated `tooling_export` alias (O2 requirement):**

Add to `crates/perl-lsp-rs-core/src/providers/mod.rs`:

```rust
// Deprecated re-export for backward compatibility
#[deprecated(
    since = "0.9.0",
    note = "Use `perl_lsp_rs_core::providers` directly"
)]
pub use crate as tooling_export;
```

**D. Verify `perl-lsp-tooling` dependency is present:**

Check `crates/perl-lsp-rs-core/Cargo.toml` line with:
```bash
grep 'perl-lsp-tooling' crates/perl-lsp-rs-core/Cargo.toml
```

If not present, add:
```toml
perl-lsp-tooling = { workspace = true }
```

**E. Migrate tests from perl-lsp-providers:**

1. Copy `crates/perl-lsp-providers/tests/comprehensive_unit_tests.rs` → `crates/perl-lsp-rs-core/tests/comprehensive_unit_tests.rs`
   - Update all imports: `perl_lsp_providers::ide::lsp_compat::*` → `perl_lsp_rs_core::providers::lsp_compat::*`
   - Update re-export shim refs: `perl_lsp_providers::diagnostics::*` → `perl_lsp_rs_core::providers::diagnostics::*` (for all 9 providers)

2. Copy `crates/perl-lsp-providers/tests/microcrate_reexports_compatibility.rs` → `crates/perl-lsp-rs-core/tests/microcrate_reexports_compatibility.rs`
   - Update imports similarly

3. Delete `crates/perl-lsp-providers/` directory entirely

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

**Test:**
```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
```

---

## Phase 5: Consumer Cleanup (perl-lsp Server Binary)

### Step 5.1: Update `crates/perl-lsp/Cargo.toml` — Remove G1b deps

**File:** `/c/wt4501/crates/perl-lsp/Cargo.toml`

**Lines to remove (exact, per plan-reviewer O4 audit):**
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

**Verification:**
- After removal, `perl-lsp-rs-core` must remain at line ~60
- `cargo check -p perl-lsp-rs` should still compile (all deps now come via perl-lsp-rs-core)

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs
```

---

### Step 5.2: Update all import sites in `crates/perl-lsp/src/` (15+ files)

**File-by-file migration (using grep results + plan-reviewer O4 enumeration):**

**A. `/c/wt4501/crates/perl-lsp/src/features/code_actions.rs`**
- `pub use perl_lsp_code_actions::*;` → `pub use perl_lsp_rs_core::providers::code_actions::*;`

**B. `/c/wt4501/crates/perl-lsp/src/features/code_actions_enhanced.rs`**
- `perl_lsp_code_actions::EnhancedCodeActionsProvider` → `perl_lsp_rs_core::providers::code_actions::EnhancedCodeActionsProvider`

**C. `/c/wt4501/crates/perl-lsp/src/features/completion.rs`**
- `pub use perl_lsp_completion::*;` → `pub use perl_lsp_rs_core::providers::completion::*;`

**D. `/c/wt4501/crates/perl-lsp/src/features/diagnostics/mod.rs`**
- `pub use perl_lsp_diagnostics::` → `pub use perl_lsp_rs_core::providers::diagnostics::`

**E. `/c/wt4501/crates/perl-lsp/src/features/diagnostics/pull.rs`**
- `use perl_lsp_diagnostics::` → `use perl_lsp_rs_core::providers::diagnostics::`
- `perl_lsp_diagnostics::detect_dead_code` → `perl_lsp_rs_core::providers::diagnostics::detect_dead_code`
- `perl_lsp_diagnostics::Diagnostic` → `perl_lsp_rs_core::providers::diagnostics::Diagnostic`
- `perl_lsp_diagnostics::DiagnosticTag::*` → `perl_lsp_rs_core::providers::diagnostics::DiagnosticTag::*`
- `perl_lsp_diagnostics::build_parse_error_hint` → `perl_lsp_rs_core::providers::diagnostics::build_parse_error_hint`

**F. `/c/wt4501/crates/perl-lsp/src/features/document_links.rs`**
- `pub use perl_lsp_navigation::*;` → `pub use perl_lsp_rs_core::providers::navigation::*;`

**G. `/c/wt4501/crates/perl-lsp/src/features/folding.rs`**
- `pub use perl_lsp_providers::ide::lsp_compat::folding::*;` → `pub use perl_lsp_rs_core::providers::lsp_compat::folding::*;`

**H. `/c/wt4501/crates/perl-lsp/src/features/formatting.rs`**
- `pub use perl_lsp_formatting::` → `pub use perl_lsp_rs_core::providers::formatting::`

**I. `/c/wt4501/crates/perl-lsp/src/features/inline_completions.rs`**
- `pub use perl_lsp_inline_completion::` → `pub use perl_lsp_rs_core::providers::inline_completion::`

**J. `/c/wt4501/crates/perl-lsp/src/features/linked_editing.rs`**
- `pub use perl_lsp_providers::ide::lsp_compat::linked_editing::*;` → `pub use perl_lsp_rs_core::providers::lsp_compat::linked_editing::*;`

**K. `/c/wt4501/crates/perl-lsp/src/features/on_type_formatting.rs`**
- `pub use perl_lsp_providers::ide::lsp_compat::on_type_formatting::*;` → `pub use perl_lsp_rs_core::providers::lsp_compat::on_type_formatting::*;`

**L. `/c/wt4501/crates/perl-lsp/src/features/references.rs`**
- `pub use perl_lsp_navigation::*;` → `pub use perl_lsp_rs_core::providers::navigation::*;`

**M. `/c/wt4501/crates/perl-lsp/src/features/rename.rs`**
- `pub use perl_lsp_rename::*;` → `pub use perl_lsp_rs_core::providers::rename::*;`

**N. `/c/wt4501/crates/perl-lsp/src/features/selection_range.rs`**
- `pub use perl_lsp_providers::ide::lsp_compat::selection_range::*;` → `pub use perl_lsp_rs_core::providers::lsp_compat::selection_range::*;`

**O. `/c/wt4501/crates/perl-lsp/src/features/semantic_tokens.rs`**
- `pub use perl_lsp_semantic_tokens::*;` → `pub use perl_lsp_rs_core::providers::semantic_tokens::*;`

**P. `/c/wt4501/crates/perl-lsp/src/features/signature_help.rs`**
- `pub use perl_lsp_providers::ide::lsp_compat::signature_help::*;` → `pub use perl_lsp_rs_core::providers::lsp_compat::signature_help::*;`

**Q. `/c/wt4501/crates/perl-lsp/src/features/type_definition.rs`**
- `pub use perl_lsp_navigation::*;` → `pub use perl_lsp_rs_core::providers::navigation::*;`

**R. `/c/wt4501/crates/perl-lsp/src/features/workspace_symbols.rs`**
- `pub use perl_lsp_navigation::*;` → `pub use perl_lsp_rs_core::providers::navigation::*;`

**S. `/c/wt4501/crates/perl-lsp/src/runtime/diagnostics.rs`**
- `perl_lsp_diagnostics::detect_dead_code` → `perl_lsp_rs_core::providers::diagnostics::detect_dead_code`
- `perl_lsp_diagnostics::build_parse_error_hint` → `perl_lsp_rs_core::providers::diagnostics::build_parse_error_hint`
- Update all 3 occurrences (confirmed via grep above)

**T. `/c/wt4501/crates/perl-lsp/src/runtime/language/misc.rs`**
- `perl_lsp_inline_completion::InlineCompletionList` → `perl_lsp_rs_core::providers::inline_completion::InlineCompletionList`
- `perl_lsp_inline_completion::PreparedInlineCompletionContext` → `perl_lsp_rs_core::providers::inline_completion::PreparedInlineCompletionContext`
- `perl_lsp_inline_completion::BackendError` → `perl_lsp_rs_core::providers::inline_completion::BackendError`
- `perl_lsp_inline_completion::BackendRequest` → `perl_lsp_rs_core::providers::inline_completion::BackendRequest`
- `perl_lsp_inline_completion::InlineCompletionItem` → `perl_lsp_rs_core::providers::inline_completion::InlineCompletionItem`

**U. `/c/wt4501/crates/perl-lsp/src/runtime/language/streaming.rs`**
- `perl_lsp_inline_completion::InlineCompletionProvider::new()` → `perl_lsp_rs_core::providers::inline_completion::InlineCompletionProvider::new()`
- `perl_lsp_inline_completion::BackendRequest` → `perl_lsp_rs_core::providers::inline_completion::BackendRequest`
- `perl_lsp_inline_completion::StreamChunk` → `perl_lsp_rs_core::providers::inline_completion::StreamChunk`
- `perl_lsp_inline_completion::StreamControl::Stop` → `perl_lsp_rs_core::providers::inline_completion::StreamControl::Stop`

**V. `/c/wt4501/crates/perl-lsp/src/runtime/mod.rs`** (3 ai-provider refs, lines ~402-414 per plan-reviewer)
- `perl_lsp_ai_provider::OpenAiConfig` → `perl_lsp_rs_core::providers::ai::OpenAiConfig`
- `perl_lsp_ai_provider::RateLimiter::new` → `perl_lsp_rs_core::providers::ai::RateLimiter::new`
- `perl_lsp_ai_provider::OpenAiProvider::new` → `perl_lsp_rs_core::providers::ai::OpenAiProvider::new`

**Comprehensive grep to ensure no misses:**
```bash
cd /c/wt4501 && grep -rn 'perl_lsp_providers\|perl_lsp_formatting\|perl_lsp_code_actions\|perl_lsp_inline_completion\|perl_lsp_ai_provider\|perl_lsp_completion\|perl_lsp_diagnostics\|perl_lsp_navigation\|perl_lsp_rename\|perl_lsp_semantic_tokens' crates/perl-lsp/src/ --include="*.rs"
```

After changes, this should return ONLY `perl_lsp_rs_core::providers::*` refs and any comments referencing old names.

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs
```

**Must pass with ZERO unresolved imports.**

---

## Phase 6: Infrastructure Updates

### Step 6.1: Update `crates/perl-lsp-rs-core/Cargo.toml` — Add Missing Dependencies

**File:** `/c/wt4501/crates/perl-lsp-rs-core/Cargo.toml`

**Add (if not already present):**

1. `perl-lsp-text-utils`:
   ```toml
   perl-lsp-text-utils = { workspace = true }
   ```
   (Required by code_actions, absorbed from perl-lsp-code-actions)

2. `ureq`:
   - Check if workspace defines `ureq` key in root `Cargo.toml`:
     ```bash
     grep -A 2 '^\[workspace.dependencies\]' Cargo.toml | grep ureq
     ```
   - If workspace defines it, use:
     ```toml
     ureq = { workspace = true }
     ```
   - If NOT defined, use:
     ```toml
     ureq = { version = "3", features = ["json"] }
     ```
   (Required by ai provider, absorbed from perl-lsp-ai-provider)

3. Verify `perl-lsp-tooling` is already present (required for `tooling_export` re-export):
   ```bash
   grep 'perl-lsp-tooling' crates/perl-lsp-rs-core/Cargo.toml
   ```

**Verify command:**
```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
```

---

### Step 6.2: Update `xtask/published-crate-baseline.txt`

**File:** `/c/wt4501/xtask/published-crate-baseline.txt`

**Change:**
```
59
```

**To:**
```
49
```

**Verification:**
```bash
cat /c/wt4501/xtask/published-crate-baseline.txt
# Should output: 49
```

---

### Step 6.3: Update root `Cargo.toml` — Publish Allow List

**File:** `/c/wt4501/Cargo.toml`

**Remove from `[workspace.metadata.workspace-publish.allow-list]` (if present):**
- perl-lsp-providers
- perl-lsp-rename
- perl-lsp-diagnostics
- perl-lsp-formatting
- perl-lsp-inline-completion
- perl-lsp-ai-provider
- perl-lsp-semantic-tokens
- perl-lsp-completion
- perl-lsp-navigation
- perl-lsp-code-actions

(These are now internal modules of perl-lsp-rs-core and should not appear in the publish allow list.)

**Verify command:**
```bash
cd /c/wt4501 && grep -A 100 'allow-list' Cargo.toml | grep -E 'perl-lsp-(providers|rename|diagnostics|formatting|inline-completion|ai-provider|semantic-tokens|completion|navigation|code-actions)'
```

Should return NO matches after cleanup.

---

## Phase 7: Validation & Testing

### Step 7.1: Comprehensive Compile Check

```bash
cd /c/wt4501 && cargo check -p perl-lsp-rs-core
cd /c/wt4501 && cargo check -p perl-lsp-rs
```

Both must pass with zero errors. Zero unresolved imports.

---

### Step 7.2: Unit Test Suite

```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core --lib
cd /c/wt4501 && cargo test -p perl-lsp-rs-core -- --test-threads=1
```

All tests must pass (including migrated diagnostics snapshots).

---

### Step 7.3: Integration & LSP Threading Tests

```bash
cd /c/wt4501 && cargo test -p perl-lsp-rs-core
cd /c/wt4501 && RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

Must pass with LSP threading model.

---

### Step 7.4: Full CI Gate

```bash
cd /c/wt4501 && just ci-gate
```

All checks pass. Specific attention:
- `cargo clippy --workspace` — zero warnings
- `cargo test --workspace --lib` — all unit tests pass
- Snapshot tests match byte-for-byte

---

## Post-Implementation Notes

### Dead Code Found (Not in Scope)

`crates/perl-parser/src/ide.rs` contains:
```rust
pub use perl_lsp_providers::ide::*
```

However, `ide` module is NOT declared in `perl-parser/src/lib.rs`, so this re-export is dead code. Leave as-is (not in G1b scope to fix).

---

### Wrapper-Type Constructors (Wrapper Pattern)

If red-TDD tests require stub constructors like `DiagnosticsProvider::new(ast, source)`, verify the signature matches the original crate's API. Do not invent constructors the red tests do not require. Document any added wrappers in the PR body.

---

## Acceptance Criteria Checklist

- [ ] All 10 crate directories deleted from `crates/`:
  - [ ] `crates/perl-lsp-rename/`
  - [ ] `crates/perl-lsp-diagnostics/`
  - [ ] `crates/perl-lsp-inline-completion/`
  - [ ] `crates/perl-lsp-semantic-tokens/`
  - [ ] `crates/perl-lsp-formatting/`
  - [ ] `crates/perl-lsp-ai-provider/`
  - [ ] `crates/perl-lsp-completion/`
  - [ ] `crates/perl-lsp-navigation/`
  - [ ] `crates/perl-lsp-code-actions/`
  - [ ] `crates/perl-lsp-providers/`

- [ ] All 10 modules accessible via `perl_lsp_rs_core::providers::*`:
  - [ ] `perl_lsp_rs_core::providers::rename::`
  - [ ] `perl_lsp_rs_core::providers::diagnostics::`
  - [ ] `perl_lsp_rs_core::providers::inline_completion::`
  - [ ] `perl_lsp_rs_core::providers::semantic_tokens::`
  - [ ] `perl_lsp_rs_core::providers::formatting::`
  - [ ] `perl_lsp_rs_core::providers::ai::`
  - [ ] `perl_lsp_rs_core::providers::completion::`
  - [ ] `perl_lsp_rs_core::providers::navigation::`
  - [ ] `perl_lsp_rs_core::providers::code_actions::`
  - [ ] `perl_lsp_rs_core::providers::lsp_compat::` (new submodule with ~1,600 LOC from perl-lsp-providers/ide/lsp_compat/)

- [ ] `providers::lsp_compat` contains all original `ide/lsp_compat/*.rs` code from perl-lsp-providers

- [ ] `crates/perl-lsp/Cargo.toml` — all 10 G1b deps removed (lines 36,37,48-56):
  - [ ] perl-lsp-providers
  - [ ] perl-lsp-formatting
  - [ ] perl-lsp-code-actions
  - [ ] perl-lsp-inline-completion
  - [ ] perl-lsp-ai-provider
  - [ ] perl-lsp-completion
  - [ ] perl-lsp-diagnostics
  - [ ] perl-lsp-navigation
  - [ ] perl-lsp-rename
  - [ ] perl-lsp-semantic-tokens
  - [ ] perl-lsp-rs-core remains (no changes needed)

- [ ] All `crates/perl-lsp/src/` import sites updated (15+ files):
  - [ ] All `perl_lsp_*` crate refs → `perl_lsp_rs_core::providers::*`
  - [ ] All `perl_lsp_providers::ide::lsp_compat::*` → `perl_lsp_rs_core::providers::lsp_compat::*`
  - [ ] `cargo check -p perl-lsp-rs` passes with zero unresolved imports

- [ ] `xtask/published-crate-baseline.txt` updated from 59 to 49

- [ ] 4 diagnostics snapshots migrated to `crates/perl-lsp-rs-core/tests/snapshots/`:
  - [ ] diag_snap__missing_pragmas_and_unused_variable.snap
  - [ ] diag_snap__package_module_happy_path.snap
  - [ ] diag_snap__script_happy_path.snap
  - [ ] diag_snap__security_string_eval.snap
  - [ ] Content verified byte-identical to pre-G1b files
  - [ ] PR body includes: "Migrated 4 diagnostics snapshots; content verified byte-identical to pre-G1b content."

- [ ] perl-lsp-providers tests migrated or replaced in `perl-lsp-rs-core/tests/`:
  - [ ] comprehensive_unit_tests.rs (~1,652 LOC) migrated
  - [ ] microcrate_reexports_compatibility.rs migrated
  - [ ] All imports updated to reference `perl_lsp_rs_core::providers::*`

- [ ] perl-lsp-rs-core Cargo.toml updated with new deps:
  - [ ] perl-lsp-text-utils added (if not present)
  - [ ] ureq added (if not present)
  - [ ] perl-lsp-tooling verified present

- [ ] Module-level cycle audit (O1):
  - [ ] `cargo check -p perl-lsp-rs-core` compiles (cycles would fail)
  - [ ] No files in `providers::code_actions` import `providers::rename` or `providers::diagnostics` in reverse
  - [ ] No files in `providers::rename` or `providers::diagnostics` import `providers::code_actions` (reverse not present)

- [ ] Aggregator public API preserved (O2):
  - [ ] All 9 provider modules re-exported from `providers` mod
  - [ ] `tooling_export` deprecated alias preserved with `#[deprecated(since = "0.9.0")]`
  - [ ] perl-lsp-tooling dep retained

- [ ] Compile & Test Results:
  - [ ] `cargo check -p perl-lsp-rs-core` passes
  - [ ] `cargo check -p perl-lsp-rs` passes
  - [ ] `cargo test -p perl-lsp-rs-core` green
  - [ ] `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2` green
  - [ ] `just ci-gate` green

- [ ] Zero behavior change observable through LSP protocol

---

## Quick Reference: File Paths

**Crates to absorb (deletion order doesn't matter, but semantically follows the phases):**
- `/c/wt4501/crates/perl-lsp-rename/`
- `/c/wt4501/crates/perl-lsp-diagnostics/`
- `/c/wt4501/crates/perl-lsp-inline-completion/`
- `/c/wt4501/crates/perl-lsp-semantic-tokens/`
- `/c/wt4501/crates/perl-lsp-formatting/`
- `/c/wt4501/crates/perl-lsp-ai-provider/`
- `/c/wt4501/crates/perl-lsp-completion/`
- `/c/wt4501/crates/perl-lsp-navigation/`
- `/c/wt4501/crates/perl-lsp-code-actions/`
- `/c/wt4501/crates/perl-lsp-providers/`

**Target module:**
- `/c/wt4501/crates/perl-lsp-rs-core/src/providers/`

**Consumer updates:**
- `/c/wt4501/crates/perl-lsp/Cargo.toml`
- `/c/wt4501/crates/perl-lsp/src/features/*.rs` (8 files)
- `/c/wt4501/crates/perl-lsp/src/runtime/mod.rs`
- `/c/wt4501/crates/perl-lsp/src/runtime/diagnostics.rs`
- `/c/wt4501/crates/perl-lsp/src/runtime/language/misc.rs`
- `/c/wt4501/crates/perl-lsp/src/runtime/language/streaming.rs`

**Infrastructure:**
- `/c/wt4501/crates/perl-lsp-rs-core/Cargo.toml`
- `/c/wt4501/crates/perl-lsp-rs-core/src/providers/mod.rs`
- `/c/wt4501/xtask/published-crate-baseline.txt`
- `/c/wt4501/Cargo.toml` (root, allow-list cleanup)

---

**End of Checklist**
