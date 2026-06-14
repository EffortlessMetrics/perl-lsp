# Implementation Checklist: #1445 — fix(dap): fallback_scope_variables child refs collide with EvalResult wire band

## Change order (compiles at each step)

### Step 1: Understand the collision and the fix
- **Context:** `fallback_scope_variables` in `crates/perl-dap/src/debug_adapter/parsing.rs` (lines 234–282) computes placeholder child references using `variables_ref.saturating_mul(100) + offset`. For Scope refs with `frame_id > ~10_000`, this arithmetic lands in the EvalResult wire band `[1_000_000, 1_999_999_999]`, causing a collision (issue #1219 class, already retired in #1430/#1432 elsewhere). The fix is to use `VariableReference::Child` codec instead, which occupies the disjoint band `[2_000_000_000, i32::MAX]`.
- **Current problematic code:** Lines 248 and 256 in `crates/perl-dap/src/debug_adapter/parsing.rs`:
  - `variables_ref.saturating_mul(100) + 2` → becomes `variables_ref.saturating_mul(100) + 1`
- **Files to change:**
  1. `crates/perl-dap/src/debug_adapter/parsing.rs` — `fallback_scope_variables()` function
  2. `crates/perl-dap/tests/` — add a new integration test for deep-frame fallback child refs

### Step 2: Add integration test (RED — failing)
- **File:** CREATE `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs`
- **Change:** Write a test named `test_fallback_scope_variables_deep_frame_child_ref_no_collision` that:
  1. Creates a Scope ref with a high frame_id (e.g., `frame_id = 50_000`) via `VariableReference::Scope { frame_id: 50_000, kind: ScopeKind::Locals }.encode()`.
  2. Calls `fallback_scope_variables(scope_ref, 0, 10)` to get placeholder variables with child refs.
  3. For each Variable with a non-zero `variables_reference`, decodes it via `VariableReference::decode()`.
  4. Asserts that every child ref decodes as `VariableReference::Child` (NOT `EvalResult`).
- **Details:** The test documents the expected behavior: a high-frame-id Scope ref's children must not collide with EvalResult band. This is RED until Step 3 is completed.
- **Verify:** `cargo test -p perl-dap --test dap_fallback_scope_variables_collision_tests -- --nocapture` (will fail with current code)

### Step 3: Migrate fallback_scope_variables to use VariableReference::Child
- **File:** `crates/perl-dap/src/debug_adapter/parsing.rs`
- **Change:** Modify `fallback_scope_variables()` function (lines 234–282):
  1. For each Variable with an expandable type (`hash`, `array`), compute its child ref using the `VariableReference::Child` codec instead of raw arithmetic.
  2. Replace lines 248 and 256:
     - Line 248: `variables_ref.saturating_mul(100) + 2` → Use `VariableReference::Child { parent: variables_ref, index: <child_index> }.encode().unwrap_or(0)`
     - Line 256: `variables_ref.saturating_mul(100) + 1` → Use `VariableReference::Child { parent: variables_ref, index: <child_index> }.encode().unwrap_or(0)`
  3. Choose stable indices for each placeholder child (e.g., index=0 for `$self`, index=1 for `@_`).
  4. The encode() call will saturate and produce a wire value in the `[2_000_000_000, i32::MAX]` band (no collision with EvalResult).
- **Details:**
  - The `VariableReference::Child::encode()` function is in `crates/perl-dap/src/debug_adapter/var_ref.rs` (lines 204–220). It uses the formula `2_000_000_000 + (parent << 16 | (index & 0xFFFF))` to pack parent and index into the Child band.
  - If `parent < 0`, `encode()` returns `None` — fallback to `unwrap_or(0)` (DAP "no children" sentinel) is safe.
  - The migration is mechanical: replace ad-hoc `* 100 + offset` with `VariableReference::Child { parent, index }.encode().unwrap_or(0)`.
- **Depends on:** None — this is a standalone migration.
- **Verify:** `cargo check -p perl-dap`

### Step 4: Add mechanical guard test (no raw arithmetic outside var_ref.rs)
- **File:** CREATE `crates/perl-dap/tests/dap_var_ref_arithmetic_guard_tests.rs`
- **Change:** Write a test named `test_var_ref_codec_no_raw_arithmetic_in_parsing` that:
  1. Uses `grep` (via std::process::Command) to scan `crates/perl-dap/src/debug_adapter/parsing.rs`.
  2. Asserts that there are NO patterns matching raw variablesReference arithmetic outside `var_ref.rs`: `% 10`, `/ 10`, `* 10`, `* 100`, `1_000_000 +`, `2_000_000_000`, etc.
  3. If a match is found, fail with a message: `"Found raw variablesReference arithmetic in parsing.rs: <line>. Use VariableReference::encode/decode in var_ref.rs instead."`.
  4. This test runs at compile-time via `#[test]` and can be executed before each build as a lint.
- **Details:** This is the "mechanical guard" requirement — it prevents future instances of this class of bug by enforcing that only `var_ref.rs` produces or consumes variablesReference via arithmetic. All other files must use the codec.
- **Verify:** `cargo test -p perl-dap --test dap_var_ref_arithmetic_guard_tests -- --nocapture`

### Step 5: Verify round-trip and decodeability
- **File:** `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` (extend from Step 2)
- **Change:** Add a second test named `test_child_ref_encode_decode_roundtrip_deep_frame`:
  1. For a high-frame-id Scope ref (e.g., `frame_id = 99_999`), manually construct a `VariableReference::Child { parent: scope_wire, index: 0 }`.
  2. Call `encode()` and then `decode()` on the wire value.
  3. Assert that decode returns `Some(VariableReference::Child { parent: scope_wire, index: 0 })`.
  4. Assert that the wire value is >= `2_000_000_000` (in the Child band).
- **Details:** This test confirms the chosen encoding round-trips correctly and lives in the disjoint Child band.
- **Verify:** `cargo test -p perl-dap --test dap_fallback_scope_variables_collision_tests`

### Step 6: Run full test suite and formatting
- **Verify:** 
  - `cargo test -p perl-dap --lib` (all unit tests pass)
  - `cargo test -p perl-dap --test dap_fallback_scope_variables_collision_tests` (new test passes)
  - `cargo test -p perl-dap --test dap_var_ref_arithmetic_guard_tests` (guard test passes)
  - `cargo clippy -p perl-dap --lib -- -D warnings` (no clippy warnings)
  - `cargo xtask fmt` (formatting correct)

### Step 7: Verify workspace-wide integration
- **Verify:**
  - `cargo test --workspace --lib` (no regressions in other crates)
  - `cargo test --workspace --test '*'` (integration tests pass)
  - Specifically: `crates/perl-lsp-rs` tests that consume DAP refs (via `handle_variables`, `handle_scopes`) still pass.

### Step 8: Deep-review checkpoint
- **Context:** DAP wire-band invariants (disjoint bands, no collisions) are identity semantics that must not regress. A reviewer must confirm:
  1. The new child refs are provably in the `[2_000_000_000, i32::MAX]` band.
  2. No Scope, EvalResult, or Child ref can collide by construction.
  3. The guard test prevents future raw arithmetic.
  4. All existing variable-expansion behavior (from `handle_variables`) is preserved.
- **Flag:** This fix touches DAP core protocol semantics. Deep-review is required (per PR #1444 closure).

## Callers and consumers

- `fallback_scope_variables()` is called from:
  - `crates/perl-dap/src/debug_adapter/variables.rs:251` — `handle_variables()` function, fallback path when debugger output is unavailable
  - Tests in `crates/perl-dap/tests/dap_scope_filtering_tests.rs`

- `VariableReference::Child` encoder is called from (newly):
  - `crates/perl-dap/src/debug_adapter/parsing.rs:fallback_scope_variables()` — **this change**
  - `crates/perl-dap/src/debug_adapter/parsing.rs:~142` (existing) — `parse_scope_variables_from_lines()` via `compute_child_reference()`

## Scope boundary

**Files IN scope:**
- `crates/perl-dap/src/debug_adapter/parsing.rs` — modify `fallback_scope_variables()` (lines 234–282)
- `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` — CREATE (integration test)
- `crates/perl-dap/tests/dap_var_ref_arithmetic_guard_tests.rs` — CREATE (mechanical guard)

**Files OUT of scope:**
- `crates/perl-dap/src/debug_adapter/var_ref.rs` — no changes (codec is stable)
- `crates/perl-dap/src/debug_adapter/parsing/scope_variables.rs` — no changes (compute_child_reference is stable, uses different arithmetic)
- All LSP crates — no changes (consumer code is unaffected; wire values are decoded transparently)
- All DAP handler code — no changes (handlers only consume via `VariableReference::decode()`)

## Flags for builder

1. **Child index assignment:** The spec does not prescribe exact indices for placeholder children (e.g., is `$self` index=0 or index=1?). The builder must choose a stable, deterministic scheme. Recommend: use position in the generated vec (0 for first child, 1 for second) to match existing variable order.

2. **Saturate vs panic:** If `VariableReference::Child::encode()` returns `None` (e.g., negative parent), fallback to `variables_reference: 0` (DAP "no children" sentinel). This is correct and safe.

3. **Guard test implementation:** The grep-based guard test in Step 4 must exclude comments, strings, and test code. Recommended approach: use a simple pattern like `grep -E '(saturating_mul|\\*.*[0-9]{2,}|\\/[0-9]|%[0-9])'` on the production file, filtered by line number to exclude test regions. Alternatively, use a structured AST scan if rustfmt/clippy patterns are available.

4. **Wire-band invariant:** After the fix, the invariant is: no child ref can ever fall in `[1_000_000, 1_999_999_999]`. This is provable by inspection (Child band is `[2_000_000_000, i32::MAX]`) and should be documented in a comment near the change.

5. **Deep-review required:** Per PR #1444 closure, DAP identity-semantics changes require deep-review sign-off before merge. This fix is scoped and low-risk, but the reviewer must confirm the band separation and round-trip correctness.
