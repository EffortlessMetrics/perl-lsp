# Implementation Checklist: #1495 evaluate-validation-order

## Overview

Reorder expression validation to run BEFORE frame-validity checks in `DebugAdapter::handle_evaluate`. This fixes two failing tests masked by frame-first precedence:
- `test_error_handling_evaluate_empty_expression` (line 1181)
- `test_error_handling_evaluate_with_newlines` (line 1157)

**Files touched:** 2 (1 production, 1 test)
**Scope boundary:** STRICT — only `crates/perl-dap/src/debug_adapter/evaluation.rs` and `crates/perl-dap/tests/session_lifecycle_tests.rs`. Do NOT modify any file in #1491, #1480, #1481, #1482.

## Change Order

### Step 1: Move expression-validation block above frame-validity check
**File:** `crates/perl-dap/src/debug_adapter/evaluation.rs` (production only)

**Current state:**
- Lines 20–32: Deserialize `args`
- Lines 34–80: Frame-validity check (`if let Some(requested_frame_id)`)
- Lines 82–138: Expression-validation block (empty check, newline check, policy check)

**Target state after move:**
- Lines 20–32: Deserialize `args` (unchanged)
- Lines 33–~89: Expression-validation block (moved up, ~57 lines)
- Lines ~90–157: Frame-validity check (moved down, ~48 lines)
- Lines ~158+: Debugger dispatch and parsing (unchanged)

**Exact mechanics:**
1. Copy lines 82–138 (the entire expression-validation block with outer braces intact)
2. Insert the copied block immediately after line 32 (after the args deserialization closing brace)
3. Delete the original lines 82–138 (now lines 140–196 due to insertion)
4. Verify brace nesting: the outer `{ let expression = ... }` must remain to preserve borrow-scoping

**Verification at this step:**
```bash
cargo build -p perl-dap
cargo clippy -p perl-dap
```
Both must succeed. Function must compile and the logic must be unchanged — only position changes.

---

### Step 2: Verify existing failing tests now pass
**File:** `crates/perl-dap/tests/session_lifecycle_tests.rs` (no changes yet, just verify)

**Existing tests that were masked (now unblocked):**
- Line 1157: `test_error_handling_evaluate_with_newlines` — was failing with "No debugger session", now should pass with "Expression cannot contain newlines"
- Line 1181: `test_error_handling_evaluate_empty_expression` — was failing with "No debugger session", now should pass with "Empty expression"

**Verify command:**
```bash
cargo test -p perl-dap --test session_lifecycle_tests test_error_handling_evaluate_empty_expression -- --exact
cargo test -p perl-dap --test session_lifecycle_tests test_error_handling_evaluate_with_newlines -- --exact
```
Both tests must PASS.

**Regression test (must still pass):**
```bash
cargo test -p perl-dap --test dap_evaluate_comprehensive_tests test_evaluate_stopped_session_frame_not_found_returns_error -- --exact
```
This test uses a valid non-empty expression (`"$x"`) with a seeded stopped session and an unknown `frameId`. After the reorder, the expression passes all input checks, then frame resolution fails with "Frame not found" — behavior unchanged. Test must PASS.

---

### Step 3: Add two new adversarial both-invalid tests
**File:** `crates/perl-dap/tests/session_lifecycle_tests.rs` (test additions only)

**Test 1: Empty expression + invalid frameId**
```rust
#[test]
fn test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error() {
    // AC:5.4 adversarial case — both-invalid
    // Regression: with frame check first, this returned "Frame not found" / "No debugger session".
    // Expression validation must win regardless of frameId validity.
    let (mut adapter, _rx) = create_test_adapter(); // no session
    let args = json!({"expression": "", "frameId": 9999});
    let response = adapter.handle_request(1, "evaluate", Some(args));
    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success);
            let msg = must_some(message);
            assert!(
                msg.contains("Empty"),
                "empty expression + invalid frame must report expression error, not frame error; got: {msg}"
            );
        }
        _ => must(Err::<(), _>("Expected Response message".to_string())),
    }
}
```

Insert after line 1181 (or immediately after the `test_error_handling_evaluate_empty_expression` test).

**Test 2: Newline expression + invalid frameId**
```rust
#[test]
fn test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error() {
    // AC:5.4 adversarial case — both-invalid
    let (mut adapter, _rx) = create_test_adapter(); // no session
    let args = json!({"expression": "system('rm -rf /')\\n$x", "frameId": 9999});
    let response = adapter.handle_request(1, "evaluate", Some(args));
    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success);
            let msg = must_some(message);
            assert!(
                msg.contains("newline"),
                "newline expression + invalid frame must report newline error, not frame error; got: {msg}"
            );
        }
        _ => must(Err::<(), _>("Expected Response message".to_string())),
    }
}
```

Insert after the first new test (after `test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error`).

**Verify commands:**
```bash
cargo test -p perl-dap --test session_lifecycle_tests test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error -- --exact
cargo test -p perl-dap --test session_lifecycle_tests test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error -- --exact
```
Both must PASS.

---

## Full Verification Suite

After completing all steps, run the complete test suite to ensure no regressions:

```bash
# Test all five named tests (2 existing + 2 new both-invalid + 1 regression guard)
cargo test -p perl-dap --test session_lifecycle_tests test_error_handling_evaluate_empty_expression -- --exact
cargo test -p perl-dap --test session_lifecycle_tests test_error_handling_evaluate_with_newlines -- --exact
cargo test -p perl-dap --test session_lifecycle_tests test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error -- --exact
cargo test -p perl-dap --test session_lifecycle_tests test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error -- --exact
cargo test -p perl-dap --test dap_evaluate_comprehensive_tests test_evaluate_stopped_session_frame_not_found_returns_error -- --exact

# Full test suite for perl-dap
cargo test -p perl-dap

# Linting and formatting
cargo clippy -p perl-dap
cargo xtask fmt
```

All tests must pass. No clippy warnings. No fmt drift.

---

## Flags for Next Agent

### Red-TDD Builder Note
The two existing tests (`test_error_handling_evaluate_empty_expression` and `test_error_handling_evaluate_with_newlines`) are already in the codebase and are currently FAILING (masked by frame-first precedence). The spec-planner is NOT writing these tests. The red-TDD builder only needs to ADD the two new adversarial both-invalid tests (Step 3 above), then verify all four named tests pass. The production reorder (Step 1) is the spec-planner's responsibility; the builder implements that in the impl branch.

### Critical Brace-Scoping Preservation
The expression-validation block at lines 82–138 is wrapped in outer braces specifically to drop the immutable borrow on `args.expression` before the mutable lock on `self.session` below. When moving this block, preserve the outer `{ let expression = &args.expression; ... }` wrapper exactly as-is. Do not collapse, flatten, or remove these braces — borrow-scope management depends on them.

### Error Message Strings Are Pinned
The tests pin exact error message substrings:
- `"Empty"` in `test_error_handling_evaluate_empty_expression`
- `"newline"` in `test_error_handling_evaluate_with_newlines`
- `"newline"` in `test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error`
- `"Empty"` in `test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error`
- `"frame not found"` in `test_evaluate_stopped_session_frame_not_found_returns_error` (regression guard)

Do NOT change any error message strings inside the validation blocks.

### Scope Boundary (STRICT)
- Production: ONLY `crates/perl-dap/src/debug_adapter/evaluation.rs`
- Tests: ONLY `crates/perl-dap/tests/session_lifecycle_tests.rs`
- Do NOT touch `dap_evaluate_comprehensive_tests.rs` (only verify regression test passes)
- Do NOT touch any file in #1491, #1480, #1481, #1482

---

## Compilation Order

1. After Step 1 (move block): `cargo build -p perl-dap` and `cargo clippy -p perl-dap` must succeed
2. After Step 2 (verify existing tests): `cargo test -p perl-dap --test session_lifecycle_tests` must pass for those two tests
3. After Step 3 (add new tests): `cargo test -p perl-dap` must pass all tests

This ordering guarantees that Rust will catch any accidental logic changes, brace imbalances, or scope errors immediately.

---

## Summary for Builder

**What you are implementing:**
- One production reorder: move ~57 lines of expression validation above ~48 lines of frame validation
- Add two new adversarial tests pinning the chosen expression-first precedence
- Verify two existing failing tests now pass, and one regression-guard test remains green

**What you are NOT changing:**
- No logic inside either the expression-validation block or the frame-validity block
- No error messages
- No test structure or setup

**Size estimate:** S (1 file reordered, 2 test functions added, ~100 lines total diff)

**Risk:** Minimal (pure block reorder, no semantic changes, regression-guard test catches frame-check regressions)
