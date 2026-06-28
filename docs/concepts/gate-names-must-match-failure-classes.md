# Gate Names Must Match Failure Classes

## The thesis

A required check's name commits to the failure class it detects. When a gate
fails, agents and operators must be able to route the failure to the correct
fix layer by reading the gate's name. If a gate named "coverage" fails due to
a test failure, agents debug the wrong layer. If a gate named "test" fails due
to a coverage measurement error, operators chase the wrong substrate problem.

A gate that fails for a reason its name does not claim is an instrument that
lies about what it measures. Fixing the code will not make the gate pass;
fixing the gate itself is required.

---

## Failure-class taxonomy

Every gate failure belongs to exactly one of these classes:

### `coverage_shortfall`

The gate ran correctly, measured correctly, and found that line or branch
coverage fell below a threshold. The fix is to improve test coverage: add
tests, exercise the uncovered paths, or document why the path is unreachable.

Example: `Codecov / Patch 95` → patch coverage is 92% → add integration tests
for the new code paths.

### `test_failure`

A test executed and failed: assertion failed, panic, timeout, or exit code
nonzero. The fix is in the code under test, not in the gate. Root cause: logic
error, missing edge case, or behavioral regression.

Example: `cargo test --lib` → test panics on `unwrap()` on a None value →
fix the code to handle the None case.

### `setup_failure`

The gate failed to set up or run: build error, dependency missing, environment
not configured, or gate infrastructure broken. The fix is in the test
infrastructure, not the code being tested.

Example: `cargo build --all-targets` → linker error on Windows due to missing
MSVC runtime → fix CI runner environment or dependency chain.

### `routing_skip`

The gate correctly determined that it should not run: no code was changed in
its domain, or a policy says to skip it for this PR. No fix needed. This is a
valid skipped-by-policy verdict.

Example: Codecov / Patch 95 → no production code was changed → skipped →
this is valid, not an error.

### `routing_bug`

The gate's routing logic failed: it selected the wrong target set, applied
wrong filters, or skipped when it should have run. The gate infrastructure is
broken.

Example: Coverage gate → production code changed in `crates/perl-parser/src/`
→ but the routed coverage packs list is empty → routing bug. This must fail
loudly and be distinguished from `routing_skip` (no coverable code changed).

---

## Incidents where gate names lied

### Coverage gate reports test failure as coverage failure (#1457/#1470)

A test suite run in the coverage gate failed (a real test failure in the
implementation). The coverage gate was scoped narrowly to run only specific
test packs. When those tests failed, the gate exited with "coverage failure"
— but the actual failure was a test logic error, not a coverage measurement
problem.

Agents tried to fix coverage (add more inline tests, boost percentages). The
correct fix was to fix the test itself. The gate's name (`Codecov / Patch 95`)
claimed to measure coverage, but it was failing due to a test failure, which is
a different failure class.

**Lesson**: A gate that fails must name its failure class. When "coverage" fails
because of a test failure, the gate is lying about what it measures. Widen the
gate's name or split the gate into separate test and coverage checks, each
claiming only what it measures.

### Required check runs wrong scope, reports clean when master breaks (#651/#1458)

A check named "compile" ran only on library code (`--lib`), not on all targets.
When a change broke integration-test code or binaries, the gate reported green.
Master merged broken; follow-up PRs inherited broken CI and failed unrelated to
their changes.

The gate's name claimed to enforce compilation of all code. It measured
something narrower. Agents trusted the name and routed PRs to merge even though
the gate's scope was insufficient.

**Lesson**: A gate's name is a contract with agents and operators. If the gate
name claims "all targets," it must run `--all-targets`. If it runs a narrower
scope, the name must reflect that.

---

## How to diagnose a gate-name mismatch

When a gate fails and you suspect the name does not match the failure class:

1. **Read the gate's invocation** (in `.ci/gate-policy.yaml`, `.github/workflows/`,
   or `.ci/gates.rs`). What command does it actually run?

2. **Classify the actual failure** (from the gate's output log):
   - Did a test assertion fail? → `test_failure`
   - Did the build break? → `setup_failure`
   - Did coverage drop below threshold? → `coverage_shortfall`
   - Should the gate have run? Did routing select the right packs? → `routing_skip` or `routing_bug`

3. **Compare to the gate's name**: Does the name claim the failure class you
   found? If not, the gate is lying.

4. **File a substrate-correction issue**, not a code-fix PR. The gate
   invocation, scope, or name needs to change.

---

## Repair: enforce name-matching in gate definitions

The repair pattern is enforcement at gate-definition time:

1. **Gate name** must match the **primary failure class** it reports.
   - Example: `unit_tests` → reports test failures, not coverage
   - Example: `coverage_lib` → reports coverage shortfall, not test failure
   - If a gate runs both tests and coverage, split it into two gates, each
     named for its primary class.

2. **Gate invocation** must match the **gate name**.
   - Example: `coverage_lib` → must run `cargo llvm-cov --lib`, not `cargo
     llvm-cov --all-targets` (that would be `coverage_all`)
   - Example: `clippy_workspace` → must run `cargo clippy --workspace`, not
     `cargo clippy -p perl-parser` (that would be `clippy_parser`)

3. **Routing logic** must distinguish between `routing_skip` (valid) and
   `routing_bug` (must fail loud).
   - Example: Coverage gate routes zero packs → is this because no production
     code changed (valid skip), or because the routing logic is broken
     (routing bug)? Must be distinguished in the gate output.

---

## Position in the pipeline

This pattern is a specialization of "verify-the-instrument.md": a gate name
is an instrument claim about what failure class the gate detects. When a gate
fails for a reason its name does not claim, the gate itself is the bug.

It is also foundational to the shift-left ladder: agents rely on gate names
to route failures to the correct fix layer. A misnamed gate routes agents to
the wrong layer, wasting cycles on code fixes when the substrate is broken.

Related patterns:
- **verify-the-instrument.md**: Gate names are claims; verify against the
  actual failure output.
- **non-exhaustive-check-silent-drop.md**: A gate that silently skips (does
  not output why it skipped) is a silent-drop of the failure signal.
- **hazard-class-invariants.md**: Failure classes have corresponding
  adversarial test patterns; gate routing must respect these invariants.

---

## Summary

| Gate name | Must fail for | Must NOT report | Fix when fails | Fix when broken |
|---|---|---|---|---|
| `cargo test` | test assertion failed | coverage drop | improve logic | N/A |
| `Codecov / Patch 95` | coverage shortfall | test assertion fail | add test coverage | widen routed pack scope or split into test + coverage gates |
| `cargo build` | build error | test failure | fix compilation | fix environment / dependencies |
| Coverage (routed packs) | coverage shortfall OR routing_bug | test_failure, setup_failure | add coverage OR fix routing | fix routing logic to distinguish skip from bug |

A gate that reports `failure_class_A` when it should report `failure_class_B`
is an instrument error. The code is not broken; the gate is. Fix the gate.

