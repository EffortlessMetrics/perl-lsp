# ADR: E2E Test for DAP Hover Evaluation

## Status
Proposed

## Context
GitHub issue #3481 reports that DAP hover evaluation for variable inspection is untested. While `handle_evaluate()` in `evaluation.rs` implements the `evaluate` request with `context: "hover"` support, there is no integration test that validates hover evaluation works end-to-end with a real `perl -d` debugging session.

The existing test `test_evaluate_hover_context_passes_safety()` only validates that expressions with `context: "hover"` pass the safety validator — it never creates a perl -d process, hits a breakpoint, or inspects actual variable values.

The verification agent identified three critical factual errors in the initial plan that must be corrected:
1. `variables_reference` in `EvaluateResponseBody` is **hardcoded to 0** (line 171 of `evaluation.rs`) — the plan's assertion `variables_reference > 0` for collections would always fail
2. `frame_id` in `EvaluateArguments` is **accepted but unused** — `handle_evaluate()` deserializes it but never uses it; all evaluations use the top frame regardless
3. `type_` assertions are **undefined** without empirical probing of what `PerlVariableRenderer` produces for each variable type

## Decision
We will add an E2E integration test for hover evaluation by:

1. **Adding an `evaluate()` helper to `DapWorkflowSession`** (`crates/perl-dap/tests/common/mod.rs`)
   - Sends `evaluate` DAP request with `expression`, `context`, and optional `frame_id`
   - Returns parsed response body or error
   - Mirrors the pattern of existing helpers (`variables()`, `stack_trace()`, etc.)

2. **Adding `test_e2e_hover_evaluation` in `dap_e2e_workflow_tests.rs`**
   - Uses a real `perl -d` session with a breakpoint after variable declarations
   - Evaluates `$scalar`, `@array`, `%hash`, and `$ref` with `context: "hover"`
   - Asserts `success: true`, non-empty `result`, and concrete `type_` values determined by probing first
   - Does **NOT** assert `variables_reference > 0` (it is always 0 for evaluate responses)
   - Does **NOT** frame the test as validating "frame context" (frame_id is unused)
   - Defers `$obj` (blessed objects) to a follow-up test requiring a package definition

## Key Behavioral Facts (Verified)

| Field | Behavior |
|-------|----------|
| `variables_reference` in evaluate response | **Always 0** — hardcoded at `evaluation.rs:171`. The `x` debugger command returns a printed string, not an expandable variable reference. |
| `frame_id` in `EvaluateArguments` | **Accepted but unused** — parsed at line 17 but never referenced in `handle_evaluate()`. Evaluations always go to the top frame. |
| `context` in `EvaluateArguments` | **Accepted but unused** — parsed but has no effect on evaluation behavior. All contexts use the same `x {expression}` command path. |

## Consequences

### Benefits
- Closes issue #3481 and moves "Evaluate correctness (session)" metric from deferred to passing
- Adds a reusable `evaluate()` helper enabling future hover/watch/repl/clipboard context tests
- Establishes a pattern for E2E evaluate testing in the perl-dap crate
- Documents actual behavior (`variables_reference: 0`) in a regression test

### Tradeoffs
- **Probe required before finalizing assertions**: `type_` strings from `PerlVariableRenderer` must be determined empirically by running a probe script, not guessed
- **Initial scope excludes blessed objects**: Creating a `$obj` requires a `package` definition in the fixture; deferred to follow-up to keep this change focused
- **15s timeout is tight**: Multi-evaluate test increases timing risk; use `workflow_timeout()` consistently

## Alternatives Considered

### 1. Mock-based test only
Do not add a real-session test; rely on existing mocked safety validation.
- **Rejected**: Issue #3481 explicitly requests real-session E2E testing. Mocked tests cannot validate actual perl -d output parsing.

### 2. Implement frame_id-based frame scoping before testing
Treat `frame_id` as a bug and implement proper frame scoping before adding tests.
- **Rejected**: Frame scoping is out of scope for issue #3481 (which is about testing existing behavior). The verification confirmed `frame_id` is accepted but unused — this is a separate issue to file.

### 3. Assert `variables_reference > 0` for collections
Plan claimed collections would return non-zero `variables_reference`.
- **Rejected**: Verified in `evaluation.rs:171` that `variables_reference` is hardcoded to `0` for all evaluate responses. This assertion would always fail.

## References
- Issue #3481: "DAP: Hover evaluation for variable inspection untested"
- Status doc: `docs/project/status/dap.md` line 35 — "Evaluate correctness (session) | #3481 | Existing mocked tests; real-session E2E deferred"
- `crates/perl-dap/src/debug_adapter/evaluation.rs` — `handle_evaluate()` implementation
- `crates/perl-dap/tests/common/mod.rs` — `DapWorkflowSession` helper
- `crates/perl-dap/tests/dap_e2e_workflow_tests.rs` — existing workflow tests for pattern reference
