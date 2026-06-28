# Implementation Checklist: Type-Separated variablesReference Codec

## Overview

Introduce a unified `VariableReference` enum codec to retire the #1219 ID collision hazard class at the type level. This refactoring preserves all wire-range semantics while enforcing type safety through ScopeKind and exhaustive range-based decoding.

**File created:** 1 (new module)  
**Lines added:** ~150 (codec module with encode/decode, ScopeKind enum)  
**Behavior:** Wire-compatible with existing ranges (Scope=[1,9_999_999], EvalResult=[1_000_000,...], Child=2_000_000_000+)  
**Migration:** 6 files (frames.rs, evaluation.rs, variables.rs, parsing.rs, parsing/scope_variables.rs, variable_cache.rs) will call the codec after implementation  
**Test location:** `crates/perl-dap/tests/var_ref_codec_tests.rs` (integration tests, no inline #[cfg(test)])

---

## Step 1: Create var_ref.rs module

**File:** `crates/perl-dap/src/debug_adapter/var_ref.rs`

**Module structure:**

The module exports:
- `enum ScopeKind { Locals=1, Package=2, Globals=3 }`
- `enum VariableReference { Scope{frame_id:i32, kind:ScopeKind}, EvalResult{counter:i32}, Child{parent:i32, index:u32} }`
- `fn encode(&self)->i32` on VariableReference
- `fn decode(i32)->Option<VariableReference>`
- `enum VariableReferenceError` for TryFrom failures

**Wire range encoding:**
- **Scope:** [1, 9_999_999] — encoded as `frame_id * 10 + kind` (1-3)
- **EvalResult:** [1_000_000, ...) — encoded as `1_000_000 + counter`
- **Child:** [2_000_000_000, ...) — encoded as `2_000_000_000 + packed(parent, index)`

**Decoding precedence:** Child (highest) → EvalResult → Scope (lowest) to avoid collisions.

**Changes:**
1. Create new file `crates/perl-dap/src/debug_adapter/var_ref.rs`
2. Add `pub mod var_ref;` to `crates/perl-dap/src/debug_adapter/mod.rs`
3. Add public exports to `crates/perl-dap/src/lib.rs`

**Verify command (after Step 1):**
```bash
cargo build -p perl-dap 2>&1 | grep -E "error|warning" | head -20
```

---

## Step 2: Red-TDD tests

**File:** `crates/perl-dap/tests/var_ref_codec_tests.rs`

Tests must compile but fail (assertions fail because codec may be unimplemented or only stubbed).

**Test naming convention:** `test_h<N>_<description>`

**Hazard-class tests to implement:**

1. **H1: No-collision (ID/ref-space collision) — the #1219 repro case**
   - Test: `test_h1_no_collision_scope_vs_evalresult`
   - Encode Scope{frame_id: 5000, kind: Locals} → 50_001 (frame_id*10 + 1)
   - Encode EvalResult{counter: 1} → 1_000_001
   - Assert they are different (50_001 != 1_000_001)
   - Decode(50_001) should return Scope (frame_id 5000, Locals), NOT EvalResult
   - This directly tests the #1219 fix

2. **H2: Wire round-trip (protocol-safe encoding/decoding)**
   - Test: `test_h2_roundtrip_scope_locals_5000`
     - Encode Scope{frame_id: 5000, kind: Locals}, decode result, encode again → should equal first encoding
   - Test: `test_h2_roundtrip_evalresult_counter_1_000_000`
     - Round-trip EvalResult{counter: 1_000_000}
   - Test: `test_h2_roundtrip_evalresult_counter_max`
     - Round-trip EvalResult with large counter value
   - Test: `test_h2_roundtrip_child_2_billion_plus`
     - Round-trip Child{parent: 1_000, index: 50} at high wire base

3. **H3: Bounds/overflow safety (no panic on extreme inputs)**
   - Test: `test_h3_encode_i32_max_no_panic`
     - Attempt to encode/decode at boundaries; must use saturating arithmetic, not panic
   - Test: `test_h3_decode_i32_max_no_panic`
     - Decode(i32::MAX) should return Some or None, never panic
   - Test: `test_h3_child_index_u32_max_no_panic`
     - Child with index=u32::MAX should not panic

4. **H5: Exhaustive decode (invalid ranges return None)**
   - Test: `test_h5_decode_zero_returns_none`
     - Decode(0) → None (not in any range [1..])
   - Test: `test_h5_decode_negative_returns_none`
     - Decode(-1) → None
   - Test: `test_h5_decode_gap_999999_returns_none`
     - Decode(999_999) → None (gap between Scope max 999_999 and EvalResult base 1_000_000)
   - Test: `test_h5_decode_bad_scope_kind`
     - Encode Scope{frame_id: 0, kind: Locals} → 1, but Decode(0) should return None (discriminant 0 is invalid)

**Test structure pattern:**
```rust
use perl_dap::var_ref::{VariableReference, ScopeKind};
use perl_tdd_support::must;

#[test]
fn test_h1_no_collision_scope_vs_evalresult() -> Result<(), Box<dyn std::error::Error>> {
    let scope = VariableReference::Scope {
        frame_id: 5000,
        kind: ScopeKind::Locals,
    };
    let eval = VariableReference::EvalResult { counter: 1 };
    
    let scope_wire = scope.encode();
    let eval_wire = eval.encode();
    
    assert_ne!(scope_wire, eval_wire, "H1: Scope and EvalResult must have different wire values");
    assert_eq!(scope_wire, 50_001, "Scope{frame_id: 5000, kind: Locals} should encode as 50_001");
    assert_eq!(eval_wire, 1_000_001, "EvalResult{counter: 1} should encode as 1_000_001");
    
    // Critical: decode(50_001) must return Scope, not EvalResult
    let decoded = VariableReference::decode(50_001).expect("H1: decode(50_001) should succeed");
    match decoded {
        VariableReference::Scope { frame_id, kind } => {
            assert_eq!(frame_id, 5000, "H1: decoded Scope frame_id should be 5000");
            assert_eq!(kind, ScopeKind::Locals, "H1: decoded Scope kind should be Locals");
        }
        _ => panic!("H1: decode(50_001) must be Scope, not {:?}", decoded),
    }
    
    Ok(())
}
```

**Verify command (after Step 2):**
```bash
cargo test -p perl-dap --test var_ref_codec_tests --no-run 2>&1 | tail -10
```

Expected: Compilation succeeds, but tests are RED (assertions fail or module doesn't exist).

---

## Step 3: Verify red state

**Command:**
```bash
cargo test -p perl-dap --test var_ref_codec_tests 2>&1 | grep -E "^test |FAILED" | head -20
```

**Expected:** All H1-H5 tests exist and are RED (failed or error).

---

## Step 4: Lint and format

**Commands:**
```bash
cargo fmt --all
cargo clippy -p perl-dap --lib 2>&1 | grep -E "error\[|warning:" | head -10
```

---

## Step 5: Commit to impl branch

```bash
# Commit spec files
git add .spec/1351-dap-var-ref-typed/ && \
  git commit -m "spec(dap): add var_ref type-separation codec spec for #1351"

# Commit red tests
git add crates/perl-dap/tests/var_ref_codec_tests.rs && \
  git commit -m "test(dap): red H1-H5 codec tests for var_ref type-separation (#1351)"

# Push
git push origin impl/1351-dap-var-ref-typed
```

---

## Builder handoff

The builder will:
1. Verify spec files are present
2. See that red tests exist (currently failing or compile-failing)
3. Implement var_ref.rs module to make all H1-H5 tests pass
4. Migrate 6 files to use the new codec
5. Commit and PR

The builder must not change the test assertions—only implement the codec to match.

---

## Scope boundaries

### In scope
- New module `var_ref.rs` with VariableReference enum and codec
- ScopeKind enum with TryFrom<i32> implementation
- H1-H5 hazard-class tests covering collision, round-trip, bounds, and exhaustive decode
- No behavior changes to existing code (migrations come after)

### Out of scope
- Migration of 6 existing files (done after codec implementation verified)
- Any change to existing DAP message flow
