# Context: #1495 evaluate-validation-order

## Problem Statement

`DebugAdapter::handle_evaluate()` validates frame IDs before validating expressions. When a single request is both invalid (empty/newline expression AND unknown frameId), the frame validation runs first and returns "Frame not found" or "No debugger session", masking the expression error. This violates UX expectations: input validation should precede context lookup.

### Evidence

Two pre-existing tests in `crates/perl-dap/tests/session_lifecycle_tests.rs` are failing:

1. **`test_error_handling_evaluate_empty_expression` (line 1181)**
   - Request: `{"expression": "", "frameId": 1}` (no session, invalid frame)
   - Expected message: `"Empty expression"`
   - Actual message: `"No debugger session"` (from frame-validity block)
   - Status: **FAILING** (masked by frame-first precedence)

2. **`test_error_handling_evaluate_with_newlines` (line 1157)**
   - Request: `{"expression": "x\ny", "frameId": 1}` (no session, invalid frame)
   - Expected message: `"Expression cannot contain newlines"`
   - Actual message: `"No debugger session"` (from frame-validity block)
   - Status: **FAILING** (masked by frame-first precedence)

### Root Cause

In `crates/perl-dap/src/debug_adapter/evaluation.rs`, lines 34–80 check frame validity BEFORE lines 82–138 check expression validity. The frame block returns early if the frame is invalid, preventing the expression checks from running.

### Why It Surfaced Now

The CI gate change #1485 started running the `unit_routed_full` gate on all PRs (not just post-merge). This gate includes `cargo test -p perl-dap`, which runs all perl-dap tests. The merge-gate green is not a required check, but it's now visible to every PR that touches `perl-ast`, `perl-dap`, `perl-lsp-rs`, or `tree-sitter-perl-rs`. This includes #1491, #1480, #1481, #1482, which are currently blocked on this latent bug.

---

## Spec Decision: Fix A vs. Fix B

### Option A: Production Fix — Hoist Expression Validation (CHOSEN)

Reorder the validation block: move expression validation above frame validation. Rationale:

- **Correct contract:** Required fields (expression) should be validated before optional context (frame)
- **UX principle:** Input validation before context lookup
- **Minimal diff:** Pure block reorder, zero logic changes
- **Tests:** Heals two failing tests without modifying test setup
- **Protocol alignment:** `EvaluateArguments` struct defines `expression: String` (required), `frameId: Option<i64>` (optional) — type hierarchy matches this fix
- **No breaking change:** Callers of `handle_evaluate` see no API change; only internal message validation order changes

### Option B: Test Fix — Seed a Valid Frame

Add session/frame setup to the two tests so they exercise the expression-validation path. Rationale:

- Preserves frame-first precedence if that's intentional
- Requires session scaffolding in two test functions

### Rejection of Option B

Option B was rejected because:
1. The masked-error behavior is itself a correctness issue (minor but real UX wart)
2. Frame-first precedence is not intentional — there's no architectural reason to validate optional context before required fields
3. Option A is simpler (reorder only, no test rewrites needed)
4. The DAP protocol's own type hierarchy (required expression, optional frame) supports Option A

---

## Prior Art and Related Specs

### Existing Validation Layering

The current validation chain in `handle_evaluate` is:
1. Deserialize args (required)
2. Frame validity (optional context)
3. Expression validity (required)
4. Expression policy (required)
5. Debugger dispatch (frame context)

The chosen fix reorders to:
1. Deserialize args (required)
2. Expression validity (required)
3. Expression policy (required)
4. Frame validity (optional context)
5. Debugger dispatch (frame context)

This aligns with the principle of "check the input before using the context".

### DAP Hazard Defaults

The spec was seeded with DAP hazard defaults from `docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md`:

- **DAP-1:** Protocol message ordering (expression vs. frame) — directly applicable
- **DAP-2:** Borrow-scope collapse (Rust lifetime safety) — critical for block move
- **DAP-3:** Error message pinning (test assertion stability) — guards against accidental string changes
- **DAP-4:** Regression guard for frame-check behavior — ensures frame validation still works on valid expressions
- **DAP-5:** Scope boundary violation (cross-file pollution) — strict isolation from parallel PRs #1491/#1480/#1481/#1482
- **DAP-6:** Test coverage completeness (both-invalid adversarial cases) — locks in the ordering choice

All six hazard classes are represented in the acceptance.md §Hazards section with specific mitigations and tests.

### Regression Testing Strategy

The plan-review identified one critical regression test:

- **`test_evaluate_stopped_session_frame_not_found_returns_error`** (in `dap_evaluate_comprehensive_tests.rs:835`)
  - Uses a valid non-empty expression (`"$x"`) with a seeded stopped session
  - Tests `frameId: 999` (invalid frame in a valid session)
  - After the reorder, expression validation passes, then frame validation fails with "Frame not found"
  - This test acts as a guard: if someone accidentally changes frame-check logic during the reorder, this test will catch it

---

## Decisions and Trade-offs

### Decision 1: Pure Reorder, No Logic Changes
**Chosen:** Move the expression-validation block intact (lines 82–138).
**Alternative:** Restructure the validation into separate functions and reorder calls. Rejected because it increases diff size and introduces untested helper functions.
**Trade-off:** Preserves current code structure and ensures no logic drift.

### Decision 2: Preserve Borrow-Scope Braces
**Chosen:** Keep the outer `{ let expression = &args.expression; ... }` wrapper when moving.
**Alternative:** Flatten or remove the braces. Rejected because the braces are necessary for borrow-checker semantics (drop the immutable borrow before the mutable lock below).
**Trade-off:** Diff includes the braces; must be explained in the spec so the builder doesn't accidentally remove them.

### Decision 3: Two New Adversarial Tests
**Chosen:** Add both-invalid tests (empty/newline expr + bad frame).
**Alternative:** No new tests, rely on existing negative tests. Rejected because without both-invalid coverage, the ordering choice (expression-first vs. frame-first) is not explicitly tested.
**Trade-off:** +2 test functions (~40 lines); locks in the ordering choice for all future maintenance.

### Decision 4: Strict Scope Boundary
**Chosen:** Touch ONLY `evaluation.rs` and `session_lifecycle_tests.rs`.
**Alternative:** Modify related files (e.g., refactor `protocol.rs` types, extract helpers into `debug_adapter.rs`). Rejected to avoid conflicts with parallel PRs #1491/#1480/#1481/#1482.
**Trade-off:** No refactoring, keeps the fix minimal and mergeable in parallel workflows.

---

## Links and References

### Issue and Plan-Review
- **Issue:** #1495 — `fix(dap): evaluate checks frame validity before empty-expression, so empty-expr error is masked`
- **Plan-review comment:** https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1495#issuecomment-4704922825
  - Identified second failing test (`test_error_handling_evaluate_with_newlines`)
  - Confirmed conflict analysis with regression-guard test
  - Specified two new adversarial tests
  - Declared Fix A ready for builder

### Related PRs (Parallel Work)
- **#1491:** Some fix (blocker: `unit_routed_full` red due to #1495 latent bug)
- **#1480, #1481, #1482:** Other DAP work (blocked by #1495)

**Strict isolation:** Do not touch files in these PRs. This spec-planner creates the impl branch and fix; red-tdd and builder execute on the same branch. Once merged, the gate unblocks the parallel PRs.

### Documentation
- **`docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md`** — DAP hazard class definitions (DAP-1 through DAP-7)
- **`docs/reference/SPEC_TEMPLATE.md`** — Spec checklist, acceptance, and context format
- **`crates/perl-dap/CLAUDE.md`** — Crate-specific guidance (test patterns, architecture)

### Test Corpus
- **`crates/perl-dap/tests/session_lifecycle_tests.rs`** — Integration tests for debug adapter lifecycle (currently lines 1157 and 1181 failing)
- **`crates/perl-dap/tests/dap_evaluate_comprehensive_tests.rs`** — Comprehensive evaluation tests (line 835 regression guard)

---

## Specification Closure

### Gaps Filled by Plan-Review

Plan-reviewer expanded the spec with:
1. **Second failing test:** `test_error_handling_evaluate_with_newlines` (line 1157) identified as having the same root cause
2. **Conflict analysis:** Verified that `test_evaluate_stopped_session_frame_not_found_returns_error` is unaffected (uses valid expression, seeded session)
3. **Adversarial test requirement:** Added two both-invalid tests to pin the ordering choice
4. **Exact function structure:** Spelled out the reorder so the builder cannot drift

### Confidence Level

**HIGH.** The spec is unambiguous:
- File paths verified against current checkout
- Line numbers verified with grep
- Test names verified
- Error message strings verified
- Regression guard test identified and verified
- Scope boundary is strict and documented

---

## Handoff Notes for Builder

1. **This is a pure reorder.** Move lines 82–138 above the frame-validity block (lines 34–80). No logic changes.

2. **Brace scoping is critical.** The outer `{ let expression = &args.expression; ... }` wrapper must be preserved. This drops the immutable borrow before the mutable `session` lock below. Removing these braces will cause a borrow-checker error (E0502).

3. **Error messages are pinned.** Tests check for exact substrings:
   - `"Empty"` in empty-expression error
   - `"newline"` in newline-expression error
   - `"Frame not found"` in frame-not-found error
   - Do not change these strings.

4. **Red-TDD note:** The two existing failing tests (`test_error_handling_evaluate_empty_expression` and `test_error_handling_evaluate_with_newlines`) are already in the codebase and are currently FAILING. Red-TDD only needs to ADD the two new adversarial both-invalid tests. The production reorder unblocks the two existing tests.

5. **Regression guard:** After implementing, verify that `test_evaluate_stopped_session_frame_not_found_returns_error` still passes. This test ensures that frame validation still works correctly on valid expressions.

6. **Scope boundary:** Touch ONLY `evaluation.rs` and `session_lifecycle_tests.rs`. Do not modify `debug_adapter.rs`, `protocol.rs`, or any file in #1491/#1480/#1481/#1482.

---

## Retrospective (Plan-Review Findings)

The most impactful finding during plan-review was discovering the second failing test (`test_error_handling_evaluate_with_newlines`). The issue body named only `test_error_handling_evaluate_empty_expression`, but running the test suite revealed that the newline test has the identical bug. This expanded the fix scope but also confirmed that the reorder-only approach heals both failures simultaneously without test rewrites.

The spec-builder workflow was not invoked for this issue because:
1. It is a trivial fix (one file reordered, 2 test functions added)
2. It introduces no new public API surface
3. It does not touch any protocol handler (LSP/DAP/stdin), only internal validation order
4. The DAP hazard defaults were seeded manually and verified against the checklist

However, the DAP hazard defaults (DAP-1 through DAP-6) were copied verbatim from `docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md` and validated against the specific surfaces touched by this fix, ensuring comprehensive hazard coverage.
