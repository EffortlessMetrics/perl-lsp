# Acceptance Criteria: #1495 evaluate-validation-order

## §Behavior

| Input | Condition | Expected Result |
|-------|-----------|-----------------|
| `{"expression": "", "frameId": 1}` | Adapter has no session (no frames) | `success: false`, `message` contains `"Empty"` |
| `{"expression": "\n$x", "frameId": 1}` | Adapter has no session | `success: false`, `message` contains `"Expression cannot contain newlines"` |
| `{"expression": "$x", "frameId": 999}` | Session exists, stopped, `frameId: 999` not in stack | `success: false`, `message` contains `"Frame not found"` |
| `{"expression": "$x", "frameId": 0}` | Session exists, stopped, `frameId: 0` valid, expression valid | `success: true`, returns variable value |
| `{"expression": "system('rm -rf /')", "frameId": 0}` | Session exists, stopped, `frameId: 0` valid, expression dangerous | `success: false`, `message` contains safety error (policy/SafeEvaluator) |
| `{"expression": "", "frameId": 999}` | **Adversarial:** both invalid (empty expr + bad frame) | `success: false`, `message` contains `"Empty"` (expression error wins) |
| `{"expression": "x\ny", "frameId": 999}` | **Adversarial:** both invalid (newline expr + bad frame) | `success: false`, `message` contains `"newline"` (expression error wins) |

---

## §Hazards

### DAP-1: Protocol Message Ordering (Expression vs. Frame Validation)

| Surface | Hazard | Mitigation | Test |
|---------|--------|-----------|------|
| `DebugAdapter::handle_evaluate()` (lines 20–170) | Frame validation runs before expression validation, masking expression errors when both are invalid. Violates UX principle: "input validation before context lookup". | **Fix:** Move expression-validation block (empty/newline/policy checks) before frame-validity check. Reorder only; no logic changes. | `test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error`, `test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error` |

### DAP-2: Borrow-Scope Collapse (Rust Lifetime Safety)

| Surface | Hazard | Mitigation | Test |
|---------|--------|-----------|------|
| Expression-validation block outer braces (lines 82–138) | The block is wrapped in `{ let expression = &args.expression; ... }` to drop the immutable borrow before the mutable `session` lock below. Flattening/removing these braces will cause a borrow-checker panic (E0502: cannot borrow `self.session` as mutable). | **Preserve:** Keep the outer braces exactly as-is when moving. Do not collapse, flatten, or remove scope wrapper. | Compilation succeeds; `cargo build -p perl-dap` does not raise E0502 |

### DAP-3: Error Message Pinning (Test Assertion Stability)

| Surface | Hazard | Mitigation | Test |
|---------|--------|-----------|------|
| Error message strings in expression-validation block: `"Empty expression"` (line 92), `"Expression cannot contain newlines"` (line 104) | Tests pin exact substrings: `message.contains("Empty")`, `.contains("newline")`. Changing these strings silently breaks existing tests. | **Preserve:** Do not modify any error message text inside expression-validation or frame-validity blocks. | `test_error_handling_evaluate_empty_expression`, `test_error_handling_evaluate_with_newlines`, `test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error`, `test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error` |

### DAP-4: Regression — Frame-Check Behavior on Valid Expressions

| Surface | Hazard | Mitigation | Test |
|---------|--------|-----------|------|
| Frame-validity block (currently lines 34–80, moves to ~90–157) | After move, frame validation on valid non-empty expressions must still return "Frame not found" when `frameId` is invalid. Accidentally modifying the frame-check logic during reorder could break this. | **Guard:** `test_evaluate_stopped_session_frame_not_found_returns_error` (in `dap_evaluate_comprehensive_tests.rs`, line 835) uses a valid expression and seeded session; must continue to pass. This ensures frame-check behavior is unchanged. | `test_evaluate_stopped_session_frame_not_found_returns_error` must pass |

### DAP-5: Scope Boundary Violation (Cross-File Pollution)

| Surface | Hazard | Mitigation | Test |
|---------|--------|-----------|------|
| Files in parallel build: `#1491`, `#1480`, `#1481`, `#1482` may touch `crates/perl-dap/` or shared types | Builder touches wrong files (e.g., modifies `crates/perl-dap/src/protocol.rs` or `crates/perl-dap/src/debug_adapter.rs` instead of only `evaluation.rs`). This causes merge conflicts or cascading breakage in other PRs. | **Strict boundary:** Touch ONLY `crates/perl-dap/src/debug_adapter/evaluation.rs` (production) and `crates/perl-dap/tests/session_lifecycle_tests.rs` (tests). Verify no changes to `dap_evaluate_comprehensive_tests.rs`, `debug_adapter.rs`, `protocol.rs`, or any other DAP file. | Git diff shows only two files changed; `git diff --stat` reports `evaluation.rs` and `session_lifecycle_tests.rs` only |

### DAP-6: Test Coverage Completeness (Both-Invalid Adversarial Cases)

| Surface | Hazard | Mitigation | Test |
|---------|--------|-----------|------|
| Pre-existing tests pin single-invalid cases (bad frame, bad expression separately) but do not cover both-invalid (empty expr + bad frame, newline expr + bad frame). This leaves the ordering choice (expression-first vs. frame-first) untested for the pathological case. | Reorder expression-first without adversarial both-invalid coverage = future dev might flip it back without test failure. | **Require:** Two new adversarial tests added, pinning expression-first precedence for (empty/newline expr + invalid frame). These tests must be in `session_lifecycle_tests.rs`. | `test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error`, `test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error` both present and passing |

---

## §Contracts

### PARSER_CONTRACTS.md (N/A)
**Reason:** This issue touches DAP (debug adapter) protocol, not the parser. No parser contracts involved.

### LSP Protocol (N/A)
**Reason:** This issue is DAP-only. LSP message types are not touched.

### DAP Protocol Compliance
- **Contract:** `EvaluateArguments` struct defines `expression: String` (required, position 1), `frameId: Option<i64>` (optional, position 2). Type hierarchy implies expression is mandatory and frame is optional context. Implementation must validate required fields before optional context.
- **Touched:** `DebugAdapter::handle_evaluate()` argument deserialization and validation chain (lines 20–170).
- **Preservation:** Reorder validates expression (required field) before frame (optional context), aligning implementation with protocol type definition.

---

## §API-Shape

### New Public API
None. This is a reorder-only fix; no new functions, types, or enum variants added.

### Modified Public API
**`DebugAdapter::handle_evaluate()`** — Function signature unchanged. Behavior changed: expression validation now precedes frame validation. This is an implementation detail; the public behavior (return `DapMessage::Response`) is unchanged.

### Internal Types Involved
- `EvaluateArguments` (protocol.rs) — not modified, only deserialization remains the same
- `DapMessage` (protocol.rs) — not modified
- `DebugState` (debug_adapter.rs) — not modified

### ID-Spaces
None added. No new frame IDs, message IDs, or protocol identifiers.

### Dup-Risk Grep
```bash
# Verify no other `handle_evaluate` implementations exist
grep -rn "fn handle_evaluate" crates/perl-dap/src/
# Should return only: crates/perl-dap/src/debug_adapter/evaluation.rs
```

### Caller Count
`handle_evaluate` is called by:
- `DebugAdapter::dispatch_request()` (in `debug_adapter.rs`)
  - Invoked by protocol message router when `"evaluate"` request arrives
  - Argument passing unchanged; only callee reordering

Callers: 1 (dispatch_request). No additional callers added.

---

## §Test-Grid

| Scenario | Test Name | File | Invariant |
|----------|-----------|------|-----------|
| **Positive: Valid expression, valid frame** | `test_evaluate_stopped_session_frame_not_found_returns_error` (used as guard) | `dap_evaluate_comprehensive_tests.rs:835` | Expression passes validation, frame `999` not found → "Frame not found" (regression guard) |
| **Negative: Empty expression, any frame** | `test_error_handling_evaluate_empty_expression` | `session_lifecycle_tests.rs:1181` | `{"expression": "", "frameId": 1}` → `success: false`, `message.contains("Empty")` |
| **Negative: Newline expression, any frame** | `test_error_handling_evaluate_with_newlines` | `session_lifecycle_tests.rs:1157` | `{"expression": "x\ny", "frameId": 1}` → `success: false`, `message.contains("newline")` |
| **Adversarial: Both-invalid (empty + bad frame)** | `test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error` | `session_lifecycle_tests.rs` (new) | `{"expression": "", "frameId": 9999}` no-session → `success: false`, `message.contains("Empty")` (not "Frame not found") |
| **Adversarial: Both-invalid (newline + bad frame)** | `test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error` | `session_lifecycle_tests.rs` (new) | `{"expression": "x\ny", "frameId": 9999}` no-session → `success: false`, `message.contains("newline")` (not "Frame not found") |
| **State transition: Unordered to Ordered** | (implied by all above) | (all tests above) | Before fix: frame-check returns first error. After fix: expression-check returns first error. Grid tests verify the new ordering. |

---

## §Blast-Radius

### Consumers of `handle_evaluate`
- **Internal:** `DispatcherV1::dispatch()` in `debug_adapter.rs` — calls `handle_evaluate()` when DAP request message with command `"evaluate"` arrives.
  - Argument passing: unchanged (`seq`, `request_seq`, `arguments`)
  - Return type: unchanged (`DapMessage`)
  - Impact: None. Caller sees same interface; only internal message validation order changes.

### Downstream Crates
- **perl-dap-eval:** Provides `SafeEvaluator::validate()` — called from within `handle_evaluate` at line 127. No change to function signature or behavior; only invoked earlier in validation order (was already invoked in expression-validation block).
- **perl-dap-stack, perl-dap-breakpoint, perl-dap-variables:** None of these are invoked in `handle_evaluate` flow. No impact.

### Must-Not-Touch Boundary (Strict)
- **Do NOT modify:** `crates/perl-dap/src/debug_adapter.rs` (calls `handle_evaluate`, must not touch)
- **Do NOT modify:** `crates/perl-dap/src/protocol.rs` (defines `EvaluateArguments`, must not touch)
- **Do NOT modify:** `crates/perl-dap/tests/dap_evaluate_comprehensive_tests.rs` (contains regression guard test, must not touch except running it)
- **Do NOT modify:** Any file in PRs #1491, #1480, #1481, #1482 (parallel work, strict isolation)

### Safe to Verify
- **`crates/perl-dap/src/debug_adapter/evaluation.rs`** — safe to reorder; no other file calls internal helper functions in this module
- **`crates/perl-dap/tests/session_lifecycle_tests.rs`** — safe to add tests; test modules are isolated

---

## Acceptance Criteria Checklist

- [ ] `cargo build -p perl-dap` succeeds after production reorder (Step 1)
- [ ] `cargo clippy -p perl-dap` reports no new warnings after reorder
- [ ] `test_error_handling_evaluate_empty_expression` passes (pre-existing test, was failing, now unblocked)
- [ ] `test_error_handling_evaluate_with_newlines` passes (pre-existing test, was failing, now unblocked)
- [ ] `test_evaluate_empty_expression_with_invalid_frameid_returns_empty_error` passes (new adversarial test added)
- [ ] `test_evaluate_newline_expression_with_invalid_frameid_returns_newline_error` passes (new adversarial test added)
- [ ] `test_evaluate_stopped_session_frame_not_found_returns_error` passes (regression guard, must remain green)
- [ ] `cargo test -p perl-dap` passes all tests (no new failures)
- [ ] `cargo xtask fmt` reports no drift after changes
- [ ] Git diff shows changes only to `evaluation.rs` and `session_lifecycle_tests.rs` (scope boundary verified)

---

## Acceptance Summary

This fix reorders validation in `DebugAdapter::handle_evaluate()` so that expression validation (empty, newlines, policy) precedes frame validation. This unblocks two pre-existing failing tests, locks in the expression-first ordering with two new adversarial both-invalid tests, and maintains regression coverage for the frame-check behavior on valid expressions. No new API surface, no type changes, no message changes — pure reordering with comprehensive test coverage.

**Risk profile:** Minimal. Pure block reorder, no logic changes, regression-guard test ensures frame behavior unchanged.
