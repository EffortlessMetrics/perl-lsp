# Acceptance Criteria: Type-Separated variablesReference Codec

## Behavior Preservation

The codec preserves all existing wire ranges and round-trip semantics. The migration sites will use the codec to encode/decode wire values, maintaining backward compatibility with clients.

| Variant | Wire Range | Encoding | Round-trip Test |
|---------|-----------|----------|-----------------|
| **Scope** | [1, 9_999_999] | `frame_id * 10 + kind` (1-3) | `test_h2_roundtrip_scope_locals_5000` |
| **EvalResult** | [1_000_000, ...) | `1_000_000 + counter` | `test_h2_roundtrip_evalresult_counter_*` |
| **Child** | [2_000_000_000, ...) | `2_000_000_000 + packed(parent, index)` | `test_h2_roundtrip_child_*` |

---

## Hazard-Class Invariants

### H1: No-collision (ID/ref-space collision, #1219 repro)

**Invariant:** Scope and EvalResult ranges never overlap. Decoding a Scope wire value never returns EvalResult, and vice versa.

**Test:** `test_h1_no_collision_scope_vs_evalresult`

**Acceptance:**
- [ ] Scope{frame_id: 5000, kind: Locals} encodes to 50_001
- [ ] EvalResult{counter: 1} encodes to 1_000_001
- [ ] 50_001 != 1_000_001 (wire values are distinct)
- [ ] Decode(50_001) returns Scope{frame_id: 5000, kind: Locals}, not EvalResult
- [ ] Decode(1_000_001) returns EvalResult{counter: 1}, not Scope

**Why:** #1219 identified a collision where `frame_id * 10 + scope_type` (max 50_001 with frame_id 5000, kind 1) collided with early EvalResult values. The fix uses separate base 1_000_000 for EvalResult and decodes by range precedence (Child → EvalResult → Scope).

---

### H2: Protocol-safe round-trip (wire encode/decode preserves type)

**Invariant:** For any VariableReference variant, `decode(encode(v)).unwrap() == v`.

**Tests:**
- `test_h2_roundtrip_scope_locals_5000`: Scope{frame_id: 5000, kind: Locals}
- `test_h2_roundtrip_scope_globals`: Scope with Globals kind (3)
- `test_h2_roundtrip_evalresult_counter_1_000_000`: EvalResult with large counter
- `test_h2_roundtrip_evalresult_counter_max`: EvalResult near counter max
- `test_h2_roundtrip_child_2_billion_plus`: Child at 2_000_000_000 base

**Acceptance:**
- [ ] Each test constructs variant V, encodes to wire W, decodes back to V', asserts V' == V
- [ ] All three variant types (Scope, EvalResult, Child) round-trip correctly
- [ ] No precision loss in round-trip (saturating arithmetic preserves value semantics)

---

### H3: Bounds/overflow safety (extreme inputs don't panic)

**Invariant:** Encoding or decoding extreme i32 values (i32::MAX, i32::MIN, near-boundary values) must not panic. Result may be saturated/clamped, but must be safe.

**Tests:**
- `test_h3_encode_i32_max_no_panic`: Encode with component = i32::MAX
- `test_h3_decode_i32_max_no_panic`: Decode(i32::MAX) → Some or None, never panic
- `test_h3_child_index_u32_max_no_panic`: Child{parent: i32::MAX, index: u32::MAX} encodes safely

**Acceptance:**
- [ ] No `unwrap()`, `expect()`, or panic-causing operations on user input
- [ ] `saturating_mul()`, `saturating_add()`, `saturating_shr()` used for arithmetic
- [ ] Decode handles all i32 values (including negative, i32::MAX, i32::MIN) without panicking
- [ ] Result is Some(variant) or None, never an error that unwinds

---

### H5: Exhaustive decode (invalid ranges return None)

**Invariant:** Wire values outside all three ranges [1..9_999_999], [1_000_000..2_000_000_000), [2_000_000_000..i32::MAX] must decode to None, not panic or return a default.

**Tests:**
- `test_h5_decode_zero_returns_none`: Decode(0) → None
- `test_h5_decode_negative_returns_none`: Decode(-1) → None
- `test_h5_decode_gap_999999_returns_none`: Decode(999_999) → None (Scope max is 999_999; next is 1_000_000 EvalResult)
- `test_h5_decode_invalid_scope_kind`: Scope encoding check (Scope kind must be 1-3, not 0 or 4+)

**Acceptance:**
- [ ] Decode(0) returns None (out of range)
- [ ] Decode(-1) returns None (out of range)
- [ ] Decode(999_999) returns None or valid Scope (if 999_999 is valid Scope wire)
  - If 999_999 encodes to a Scope (e.g., frame_id 99_999, kind 9), kind 9 is invalid → None
  - Or the gap check: if Scope max boundary is < 999_999, then 999_999 is gap → None
- [ ] Decode(i32::MIN) returns None (out of range)

---

## Code Quality

| Requirement | Acceptance |
|-------------|-----------|
| **No banned patterns** | `var_ref.rs` production code contains no `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()`, `std::panic::catch_unwind()` |
| **Format compliance** | `cargo fmt --all` produces no changes to var_ref.rs |
| **Clippy clean** | `cargo clippy -p perl-dap --lib` produces no new warnings in var_ref.rs |
| **Module structure** | `var_ref.rs` is a single file under `crates/perl-dap/src/debug_adapter/` |
| **Public exports** | `pub enum VariableReference`, `pub enum ScopeKind`, `pub enum VariableReferenceError` exported from lib.rs |
| **TryFrom implemented** | `impl TryFrom<i32> for ScopeKind` returns `Result<Self, VariableReferenceError>` |

---

## Test Suite

### Red Tests (must exist and fail before implementation)

- [ ] `test_h1_no_collision_scope_vs_evalresult` — H1 invariant: Scope vs EvalResult collision is retired
- [ ] `test_h2_roundtrip_scope_locals_5000` — H2 invariant: Scope round-trip
- [ ] `test_h2_roundtrip_scope_globals` — H2 variant: Globals kind
- [ ] `test_h2_roundtrip_evalresult_counter_1_000_000` — H2 invariant: EvalResult round-trip
- [ ] `test_h2_roundtrip_evalresult_counter_max` — H2 boundary: large counter
- [ ] `test_h2_roundtrip_child_2_billion_plus` — H2 invariant: Child round-trip
- [ ] `test_h3_encode_i32_max_no_panic` — H3 invariant: extreme encode is safe
- [ ] `test_h3_decode_i32_max_no_panic` — H3 invariant: extreme decode is safe
- [ ] `test_h3_child_index_u32_max_no_panic` — H3 boundary: u32::MAX index
- [ ] `test_h5_decode_zero_returns_none` — H5 invariant: out-of-range → None
- [ ] `test_h5_decode_negative_returns_none` — H5 invariant: negative → None
- [ ] `test_h5_decode_gap_999999_returns_none` — H5 gap: Scope-EvalResult boundary
- [ ] `test_h5_decode_invalid_scope_kind` — H5 exhaustiveness: invalid discriminant

---

## CI Verification

The following CI checks must pass (3 gates):

1. **Rust test compile and run** — `cargo test -p perl-dap --test var_ref_codec_tests --no-run` and `cargo test -p perl-dap --test var_ref_codec_tests` all H1-H5 tests pass ✓
2. **Clippy and fmt** — `cargo clippy -p perl-dap --lib` and `cargo fmt --all` produce no new issues ✓
3. **Integration compile** — `cargo build -p perl-dap` succeeds; var_ref module is public and linked ✓

---

## Diff Audit

- [ ] Exactly 1 file created: `crates/perl-dap/src/debug_adapter/var_ref.rs`
- [ ] Exactly 2 files modified: `crates/perl-dap/src/debug_adapter/mod.rs` (pub mod var_ref), `crates/perl-dap/src/lib.rs` (pub use)
- [ ] 1 test file created: `crates/perl-dap/tests/var_ref_codec_tests.rs` (not modified, created fresh)
- [ ] No unintended diffs (no formatting of unrelated code)
- [ ] Commit messages: `spec(dap):` for spec files, `test(dap):` for red tests, `feat(dap):` for var_ref.rs

---

## Functional Edge Cases

- [ ] **Scope kind validation:** ScopeKind::try_from() on invalid discriminant (0, 4, -1) returns Err, not panic
- [ ] **Empty frame_id:** Scope{frame_id: 0, kind: Locals} encodes as 1 (valid, in range [1..9_999_999])
- [ ] **Negative counter:** EvalResult{counter: -1} encodes as 999_999; behavior defined by saturating_add
- [ ] **Child parent overflow:** Child with parent near i32::MAX and index > 0 saturates, doesn't panic

---

## Behavioral Guarantees

After implementation:

1. **No collision:** Any Scope wire value decodes to Scope; any EvalResult wire value decodes to EvalResult. No mixing.
2. **Round-trip:** encode(decode(w).unwrap()) == w for all valid w.
3. **Safe bounds:** All extreme inputs (i32::MAX, u32::MAX) are handled with saturating arithmetic.
4. **Exhaustive decode:** Every i32 maps to exactly one of: Scope, EvalResult, Child, or None.

---

## Sign-Off

Red-TDD: Verify red tests exist, compile, and fail. Commit as `test(dap): red H1-H5 codec tests for var_ref type-separation (#1351)`. Apply `red-tdd-reviewed` label.

Builder: Implement var_ref.rs per checklist, run all H1-H5 tests until green. Commit as `feat(dap): introduce VariableReference codec with type-separated ref-spaces (#1351)`. Migration to 6 files comes in a follow-up or same PR per builder judgment.

Reviewer: Verify all H1-H5 hazard-class invariants are tested and passing. Check that round-trip is exact (no precision loss). Verify saturating arithmetic is correct at boundaries.
