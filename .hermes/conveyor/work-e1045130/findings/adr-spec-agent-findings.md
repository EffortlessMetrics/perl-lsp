# ADR/Spec Findings — work-e1045130

## What This ADR Decides
Add an E2E integration test for DAP hover evaluation (issue #3481) by extending `DapWorkflowSession` with an `evaluate()` helper and adding a real-session test in `dap_e2e_workflow_tests.rs`. The ADR corrects three critical factual errors from the initial plan.

## Key Decision
The decision is to add the test with **corrected assertions** that match verified behavior:
- `variables_reference` is **always 0** (hardcoded at `evaluation.rs:171`), not `> 0` as the plan claimed
- `frame_id` is accepted but **unused** in `handle_evaluate()` — test does not validate frame scoping
- `type_` assertions require **empirical probing** to determine actual renderer output strings
- Blessed objects (`$obj`) are **deferred** to a follow-up (require package definition)

## Alternatives Considered
1. **Mock tests only** — rejected, issue #3481 explicitly requests real-session E2E
2. **Implement frame_id scoping before testing** — rejected as out of scope for #3481 (separate bug)
3. **Assert `variables_reference > 0`** — rejected, always 0 in code

## Consequences
- **Benefit**: Closes #3481, adds reusable `evaluate()` helper
- **Risk**: `type_` assertions need probe-first approach to determine actual strings
- **Risk**: 15s timeout tight for multi-evaluate test

## Acceptance Criteria
1. `evaluate()` helper added to `DapWorkflowSession`
2. Test validates hover evaluation for `$scalar`, `@array`, `%hash`, `$ref`
3. `variables_reference == 0` assertions (not `> 0`)
4. Test uses real perl -d session with breakpoint at `BP_LINE_2`
5. `perl_available()` skip guard used
6. Probe-first approach for `type_` assertions
