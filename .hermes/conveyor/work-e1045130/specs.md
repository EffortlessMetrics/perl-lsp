# Spec: E2E Hover Evaluation for Variable Inspection

## Feature Description
Add an end-to-end integration test that validates DAP hover evaluation works correctly with a real `perl -d` debugging session. The test launches a Perl script under the debugger, hits a breakpoint after variable declarations, and verifies that `evaluate` requests with `context: "hover"` return correct results for `$scalar`, `@array`, `%hash`, and `$ref` variable types.

## Non-Goals
- This spec does **not** include implementing `frame_id`-based frame scoping (currently `frame_id` is accepted but unused in `handle_evaluate()`)
- This spec does **not** include modifying how `context` affects evaluation behavior (it is intentionally a no-op)
- This spec does **not** include blessed object (`$obj`) testing — creating a test fixture with a Perl class is deferred to a follow-up
- This spec does **not** include modifying `variables_reference` to be non-zero (it is intentionally hardcoded to 0 for evaluate responses)

## Dependencies
- `perl` available on `PATH` (checked via `perl_available()` helper)
- `workflow_timeout()` helper (15s normal, 60s under coverage)
- `DapWorkflowSession` test helper infrastructure
- BP_LINE constants from existing workflow tests

## Behavioral Constraints (Verified)
| Field | Constraint |
|-------|-----------|
| `variables_reference` | Must be `0` in evaluate responses (hardcoded in `evaluation.rs:171`) |
| `frame_id` | Accepted in request but has no effect on evaluation (not used in `handle_evaluate()`) |
| `context` | Accepted in request but has no effect on evaluation (not used in `handle_evaluate()`) |
| `type_` | Must be determined empirically via probing — no predefined type strings guaranteed |

## Acceptance Criteria

### AC1: `evaluate()` helper method
`DapWorkflowSession` in `crates/perl-dap/tests/common/mod.rs` has a method with signature:
```rust
pub fn evaluate(
    &mut self,
    expression: &str,
    context: &str,
    frame_id: Option<i64>,
) -> Result<Value, String>
```
The helper:
- Builds a JSON args object with `expression`, `context`, and `frameId` fields
- Calls `self.request("evaluate", Some(args))`
- Returns `self.expect_success(&resp, "evaluate")` parsed as `Value`

### AC2: Breakpoint hits correctly
The test script declares variables at lines 1-4 and sets a breakpoint at `BP_LINE_2` (line 5, after declarations). The `configurationDone()` initial `c` skips the implicit stop at `BP_LINE_1` (line 4), so the breakpoint at `BP_LINE_2` is reliably hit.

### AC3: Hover evaluation for scalar ($scalar)
Given a Perl script:
```perl
use strict;
use warnings;
my $scalar = 42;
print "break here\n";  # BP_LINE_2
```
When the debugger stops at the breakpoint and sends:
```json
{ "expression": "$scalar", "context": "hover" }
```
Then the response has:
- `success: true`
- `result` containing "42"
- `type_` matching the renderer output for a number (value determined by probe)

### AC4: Hover evaluation for array (@array)
Given `@array = (1, 2, 3)` in the test script, evaluating `"@array"` returns results containing "1", "2", "3" (exact format from `x @array` output, determined by probe).

### AC5: Hover evaluation for hash (%hash)
Given `%hash = (a => 1, b => 2)` in the test script, evaluating `"%hash"` returns results containing key-value pairs (exact format from `x %hash` output, determined by probe).

### AC6: Hover evaluation for scalar reference ($ref)
Given `$ref = \$scalar` in the test script, evaluating `"$ref"` returns a result containing a reference address or dereferenced value (format determined by probe).

### AC7: `variables_reference` is always 0
For all evaluate responses (scalar, array, hash, ref), `variables_reference == 0`. This is correct behavior — the `x` debugger command returns a printed string, not an expandable variable reference. Assertions must reflect this, not `> 0`.

### AC8: Test skips gracefully when perl unavailable
The test uses `perl_available()` to skip when `perl` is not on `PATH`, consistent with other DAP workflow tests.

## Test Structure
```
test_e2e_hover_evaluation
├── launch perl -d with test script
├── set breakpoint at BP_LINE_2
├── configuration_done → continue → wait_stopped
├── stack_trace (to get frame_id, even though unused)
└── for each variable ($scalar, @array, %hash, $ref):
    └── evaluate(expression, "hover", frame_id)
        └── assert success: true
        └── assert result: non-empty
        └── assert type_: matches probe output
        └── assert variables_reference == 0
```

## Probe Requirement
Before finalizing `type_` assertions, a probe script must be run to determine what `PerlVariableRenderer` produces for each variable type. The probe should:
1. Launch a minimal perl -d session with each variable type
2. Evaluate `$var` with `context: "hover"`
3. Log the raw response including `type_` field
4. Use the observed values to write concrete assertions

This is necessary because the exact type strings (e.g., `SCALAR`, `ARRAY`, `HASH`, `REF`) depend on how `VariableParser` and `PerlVariableRenderer` process debugger output, and these have varied historically.

## Future Work (Out of Scope)
- Blessed object (`$obj`) testing — requires package definition in fixture
- Frame-scoped evaluation with `frame_id` — `frame_id` is currently unused
- Expandable `variables_reference` for evaluate — currently always 0 by design
