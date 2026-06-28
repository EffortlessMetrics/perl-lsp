# Implementation Checklist: #1432 — Migrate 6 DAP variablesReference consumers to VariableReference codec

## Overview

The `VariableReference` codec (merged in PR #1430) defines type-safe encoding/decoding for DAP's single `i32` variablesReference field across three disjoint wire bands:
- **Scope:** [1, 999_999] — `frame_id * 10 + kind` (kind ∈ [1,3])
- **EvalResult:** [1_000_000, 1_999_999_999] — `1_000_000 + counter`
- **Child:** [2_000_000_000, i32::MAX] — `2_000_000_000 + (parent << 16 | index)`

This checklist migrates 6 consumer files from raw arithmetic to `VariableReference::encode()` and `VariableReference::decode()`.

## Call sites identified

### `frames.rs` (lines 148-150)
```rust
let locals_ref = frame_id * 10 + 1;
let package_ref = frame_id * 10 + 2;
let globals_ref = frame_id * 10 + 3;
```
**Type:** Scope encoding (3 sites)
**Migrate to:** `VariableReference::Scope { frame_id, kind: ScopeKind::Locals/Package/Globals }.encode()`

### `evaluation.rs` (line 569)
```rust
let eval_ref = 1_000_000_i32.saturating_add(Self::i64_to_i32_saturating(raw_counter as i64));
```
**Type:** EvalResult encoding
**Migrate to:** `VariableReference::EvalResult { counter }.encode()`
**Note:** Must handle `Option` return and propagate `None` → return `0` (invalid varref)

### `variables.rs` (lines 120-121)
```rust
let frame_id = variables_ref / 10;
match variables_ref % 10 {
    1 => { /* Locals */ }
    2 => { /* Package */ }
    3 => { /* Globals */ }
    _ => { /* invalid */ }
}
```
**Type:** Scope decoding
**Migrate to:** `VariableReference::decode(variables_ref)` with match on variants
**Note:** Must handle `None` → return empty variables list (DAP-correct response, never crash)

### `parsing.rs` (line 124)
```rust
let scope_type = variables_ref % 10;
```
**Type:** Scope kind extraction (discriminant only)
**Migrate to:** Decode and extract kind from Scope variant

### `parsing.rs` (line 228)
```rust
let variables = match variables_ref % 10 {
    1 => { /* Locals */ }
    2 => { /* Package */ }
    3 => { /* Globals */ }
    _ => vec![] // fallback
}
```
**Type:** Scope kind discriminant (fallback path)
**Migrate to:** `VariableReference::decode()` with Scope pattern match

### `parsing/scope_variables.rs` (lines 60-65)
```rust
pub(super) fn compute_child_reference(variables_ref: i32, start: usize, idx: usize) -> i32 {
    let absolute_index = start.saturating_add(idx).saturating_add(1);
    variables_ref.saturating_mul(1000).saturating_add(...)
}
```
**Type:** Child reference encoding (ad-hoc arithmetic: parent * 1000 + index)
**Current usage:** Children of Scope refs use this, NOT the VariableReference::Child codec
**Decision point:** This is NOT part of the VariableReference codec (which uses parent << 16 | index).
This arithmetic is Scope-child-specific and must NOT be migrated to the codec.
**Action:** Leave unchanged; verify it does not use raw % 10 / * 10 / / 10 arithmetic (it doesn't).

## Change order (compiles at each step)

### Step 1: Add `var_ref` module import and `use` declarations to `mod.rs`
- **File:** `crates/perl-dap/src/debug_adapter/mod.rs`
- **Change:** Add module declaration and import `VariableReference`, `ScopeKind` into scope
- **Details:**
  - Ensure `mod var_ref;` is declared (likely already present from #1430 merge)
  - Add `use self::var_ref::{VariableReference, ScopeKind};` at the top of `mod.rs` or re-export in `lib.rs`
- **Verify:** `cargo check -p perl-dap`

### Step 2: Migrate `frames.rs` handle_scopes (lines 148-150)
- **File:** `crates/perl-dap/src/debug_adapter/frames.rs`
- **Change:** Replace 3 raw `frame_id * 10 + kind` with `VariableReference::Scope { frame_id, kind }.encode()`
- **Details:**
  ```rust
  let locals_ref = VariableReference::Scope { frame_id, kind: ScopeKind::Locals }.encode();
  let package_ref = VariableReference::Scope { frame_id, kind: ScopeKind::Package }.encode();
  let globals_ref = VariableReference::Scope { frame_id, kind: ScopeKind::Globals }.encode();
  ```
  - frame_id is already available as `i32` (line 144)
  - All three are guaranteed in-band (frame_id comes from DAP, typically small)
  - No `None` handling needed here (encode always succeeds for small frame_ids)
- **Verify:** `cargo check -p perl-dap`

### Step 3: Migrate `evaluation.rs` allocate_evaluate_result_ref (line 569)
- **File:** `crates/perl-dap/src/debug_adapter/evaluation.rs`
- **Change:** Replace manual `1_000_000 + counter` with `VariableReference::EvalResult { counter }.encode()`
- **Details:**
  ```rust
  let raw_counter = self.debugger_output_marker.fetch_add(1, Ordering::Relaxed);
  let counter = Self::i64_to_i32_saturating(raw_counter as i64);
  let eval_ref = VariableReference::EvalResult { counter }.encode();
  // eval_ref is i32, use directly in upsert and return i64::from(eval_ref)
  ```
  - **CLARIFICATION:** Looking at the actual codec, `encode()` returns `i32` (NOT `Option`). It saturates on overflow.
  - For EvalResult { counter }, the wire value is `1_000_000 + counter` saturated
  - No `None` handling needed (saturates at i32::MAX)
  - Verify wire value matches old arithmetic exactly for counter ∈ [0, 999_999]
- **Verify:** `cargo check -p perl-dap && cargo test -p perl-dap evaluate_allocation_tests`

### Step 4: Migrate `variables.rs` handle_variables decode site (lines 120-121)
- **File:** `crates/perl-dap/src/debug_adapter/variables.rs`
- **Change:** Replace decode arithmetic with `VariableReference::decode(variables_ref)` pattern match
- **Details:**
  ```rust
  match VariableReference::decode(variables_ref) {
      Some(VariableReference::Scope { frame_id, kind }) => {
          match kind {
              ScopeKind::Locals => {
                  // (lines 122-145: lexical vars via B-module)
              }
              ScopeKind::Package => {
                  // (existing Package code)
              }
              ScopeKind::Globals => {
                  // (existing Globals code)
              }
          }
      }
      Some(VariableReference::EvalResult { counter }) => {
          // Handle EvalResult references → look up in variable cache
      }
      Some(VariableReference::Child { parent, index }) => {
          // Handle Child references (if applicable)
      }
      None => {
          // Invalid varref → return empty variables list (DAP-correct, never crash)
          return DapMessage::Response {
              success: false,
              message: Some("Invalid variablesReference".to_string()),
              body: None,
              ...
          };
      }
  }
  ```
  - **CRITICAL:** `decode()` returns `Option` — must handle `None` gracefully with empty/error response
  - Do NOT panic or crash on invalid varref
  - Test: verify `variables_ref = 0, -1, 999_999_998` all return error, never crash
- **Verify:** `cargo check -p perl-dap && cargo test -p perl-dap variables_tests`

### Step 5: Migrate `parsing.rs` parse_scope_variables_from_lines (line 124)
- **File:** `crates/perl-dap/src/debug_adapter/parsing.rs`
- **Change:** Replace `scope_type = variables_ref % 10` with decoded ScopeKind
- **Details:**
  ```rust
  let scope_type = match VariableReference::decode(variables_ref) {
      Some(VariableReference::Scope { kind, .. }) => match kind {
          ScopeKind::Locals => 1,
          ScopeKind::Package => 2,
          ScopeKind::Globals => 3,
      },
      _ => return (Vec::new(), HashMap::new()), // Invalid varref
  };
  let parsed = scope_variables::parse_assignments(lines, scope_type);
  ```
  - scope_variables::parse_assignments still expects i32 kind discriminant (1/2/3)
  - Extract discriminant from the decoded variant
  - Handle `None` by returning empty results (no crash)
- **Verify:** `cargo check -p perl-dap`

### Step 6: Migrate `parsing.rs` fallback_scope_variables (line 228)
- **File:** `crates/perl-dap/src/debug_adapter/parsing.rs`
- **Change:** Replace `match variables_ref % 10` with `VariableReference::decode()` pattern
- **Details:**
  ```rust
  let variables = match VariableReference::decode(variables_ref) {
      Some(VariableReference::Scope { kind, .. }) => match kind {
          ScopeKind::Locals => vec![
              Variable { name: "$self".to_string(), ... },
              Variable { name: "@_".to_string(), ... },
          ],
          ScopeKind::Package => vec![
              Variable { name: "$VERSION".to_string(), ... },
          ],
          ScopeKind::Globals => vec![
              Variable { name: "$_".to_string(), ... },
          ],
      },
      _ => vec![], // Invalid varref → empty fallback
  };
  ```
  - Fallback path for when debugger output is unavailable
  - Handle `None` by returning empty vec (DAP-correct)
  - No crash on invalid varref
- **Verify:** `cargo check -p perl-dap`

### Step 7: SKIP `parsing/scope_variables.rs` compute_child_reference
- **File:** `crates/perl-dap/src/debug_adapter/parsing/scope_variables.rs`
- **Decision:** This function implements Scope-child encoding (parent * 1000 + index), which is distinct from the VariableReference codec's Child variant (parent << 16 | index). It is NOT a consumer of the codec and must NOT be migrated.
- **Verify:** Confirm no % 10 / * 10 / / 10 raw arithmetic remains (it doesn't). Leave as-is.

### Step 8: Verify no raw arithmetic remains in variable_cache.rs
- **File:** `crates/perl-dap/src/debug_adapter/variable_cache.rs`
- **Change:** None required — this file only stores/retrieves by reference, no encoding/decoding
- **Verify:** Confirm no % 10 / * 10 / / 10 arithmetic (verified: none present)

### Step 9: Full integration test + wire-level validation
- **Test:** Add integration test proving H4 (wire output unchanged for small frame_ids)
  - Encode a Scope { frame_id: 0, kind: Locals } → wire 1 (matches old `0 * 10 + 1`)
  - Encode a Scope { frame_id: 99_999, kind: Globals } → wire 999_993 (matches old `99_999 * 10 + 3`)
  - Prove EvalResult { counter: 0 } → 1_000_000, EvalResult { counter: 1 } → 1_000_001
  - Decode each wire value back → original variant (round-trip)
- **File:** `crates/perl-dap/tests/var_ref.rs` (already exists with unit tests; add integration)
- **Verify:** `cargo test -p perl-dap var_ref_tests`

### Step 10: Final verification
- **Verify:**
  ```bash
  cargo test -p perl-dap
  cargo xtask fmt
  cargo clippy -p perl-dap -W clippy::all
  ```
- **Compile check (all-features):**
  ```bash
  cargo check -p perl-dap --all-features
  ```

## Callers and consumers

- `VariableReference::encode()` called from: `frames.rs`, `evaluation.rs`, `parsing.rs` (post-migration)
- `VariableReference::decode()` called from: `variables.rs`, `parsing.rs` (post-migration)
- `ScopeKind` enum used in: `frames.rs`, `variables.rs`, `parsing.rs` (post-migration)
- `compute_child_reference()` called from: `parsing.rs` line 131 (not migrated — parent function touches it but does not migrate this call)

## Scope boundary

**Files IN scope (6 files):**
1. `crates/perl-dap/src/debug_adapter/frames.rs` — encode Scope refs
2. `crates/perl-dap/src/debug_adapter/evaluation.rs` — encode EvalResult refs
3. `crates/perl-dap/src/debug_adapter/variables.rs` — decode variablesReference
4. `crates/perl-dap/src/debug_adapter/parsing.rs` — decode Scope kind (2 sites)
5. `crates/perl-dap/src/debug_adapter/mod.rs` — module imports (ensure var_ref is accessible)
6. Tests: `crates/perl-dap/tests/var_ref.rs` — add integration tests for H4 wire-identity

**Files OUT of scope:**
- `parsing/scope_variables.rs` — leave compute_child_reference unchanged (not a VariableReference consumer)
- `variable_cache.rs` — no changes (no arithmetic)
- `dispatch.rs`, `execution.rs`, `output.rs`, `patterns.rs`, `process.rs`, `safe_eval.rs`, `session.rs`, `sync_utils.rs`, `transport.rs` — no variablesReference handling
- All other crates — no changes

## Flags for builder

1. **Codec return types:** Verify the actual signatures:
   - `VariableReference::encode(&self) -> i32` (returns `i32`, saturates on overflow, never `Option`)
   - `VariableReference::decode(raw: i32) -> Option<Self>` (returns `Option`, can be `None` for invalid varref)

2. **Scope encoding simplicity:** For Scope refs, encode() always succeeds (saturates at worst). frame_id ∈ [0, 99_999] is the safe range; larger values saturate but do not panic.

3. **decode() exhaustiveness:** Match on all 4 cases: `Some(Scope{...})`, `Some(EvalResult{...})`, `Some(Child{...})`, `None`. No defaults/underscores.

4. **Test coverage:** Ensure red-tdd adds boundary tests:
   - frame_id = 99_999 (max Scope) → encodes to 999_993 (matches old formula)
   - frame_id = 100_000 (over max) → encode saturates but stays in Scope band
   - variables_ref = 0 (reserved, DAP "no children") → decode returns None
   - variables_ref = -1 (invalid) → decode returns None
   - variables_ref = 1_000_000 (EvalResult base) → decode returns EvalResult { counter: 0 }
   - variables_ref = 2_000_000_000 (Child base) → decode returns Child { parent: 0, index: 0 }

5. **H4 wire invariant:** Prove that for small frame_ids (0-999), the wire values are IDENTICAL to the old formula. This is required for backward-compat with cached client-side varref values.

6. **No raw arithmetic:** After migration, grep for `% 10`, `/ 10`, `* 10` in the 6 consumer files — should find ZERO matches (except in comments or tests).

7. **Scope-child distinction:** compute_child_reference implements Scope-child encoding (parent * 1000 + index). This is NOT the same as VariableReference::Child (parent << 16 | index). Do NOT confuse them; leave compute_child_reference as-is.
