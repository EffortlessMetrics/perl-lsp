# Verification Auditor Prompt

## Purpose

The Verification Auditor analyzer assesses **Correctness** evidence for a PR. It evaluates test depth, error path coverage, mutation survival, and whether tests verify behavior rather than just structure.

**Quality Surface**: Correctness

## Required Inputs

Provide the following context to the analyzer:

### 1. Test Files in Diff
```
<test_diff>
[Git diff of test files only, or test file contents]
</test_diff>
```

### 2. Test Output/Receipts
```
<test_output>
[Test run output: cargo test output, pass/fail counts]
[If available: CI run results, local test output]
</test_output>
```

### 3. Mutation Testing Results (if available)
```
<mutation_results>
[cargo-mutants output or similar mutation testing results]
[Include surviving mutants if any]
</mutation_results>
```

### 4. Code Changes Being Tested
```
<code_changes>
[Non-test code that was changed, for correlation with test coverage]
</code_changes>
```

### 5. Diff Scout Output (recommended)
```
<diff_scout>
[Hotspots identified by diff-scout for prioritization]
</diff_scout>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: verification-auditor
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

test_inventory:
  added: <count>
  modified: <count>
  removed: <count>
  total_tests_after: <count if known, else "unknown">

test_depth:
  behavior_tests: <count>
  shape_tests: <count>
  error_path_tests: <count>
  property_tests: <count>
  integration_tests: <count>
  error_path_coverage: <yes|partial|no>

test_quality_assessment:
  - test: <test name or file:function>
    type: <behavior|shape|error_path|property|integration>
    coverage: <what code path it exercises>
    strength: <strong|adequate|weak>
    notes: <why this assessment>

mutation_survival:
  score: <percentage, e.g., "87%" or "N/A">
  mutants_tested: <count or "N/A">
  surviving_mutants:
    - location: <file:line>
      mutation: <description of mutation>
      risk: <high|medium|low>

invariants:
  added:
    - invariant: <description>
      enforced_by: <test name or gate>
      evidence: <file:line or test output>
  existing_used:
    - invariant: <description>
      test: <test that exercises it>

regression_coverage:
  bugs_addressed: <count of bugs this PR fixes>
  bugs_with_regression_test: <count>
  missing_regression_tests:
    - bug: <issue number or description>
      reason: <why no test, if known>

hotspot_coverage:
  - hotspot: <from diff-scout>
    tested: <yes|partial|no>
    test_references: [<list of test names>]

findings:
  - id: <unique_id, e.g., "VA-001">
    severity: <P1|P2|P3|info>
    category: <shallow_test|missing_error_path|surviving_mutant|no_regression_test|untested_hotspot>
    summary: <one line>
    evidence:
      - anchor: <file:line or test output>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings:
    - <bullet 1>
    - <bullet 2>
  correctness_delta: <+2|+1|0|-1|-2>

assumptions:
  - <what was assumed>
```

## Key Questions Answered

1. **Are tests shallow or deep?** - Do they verify behavior or just check structure?
2. **Would mutations be caught?** - What's the mutation survival rate?
3. **Are error paths exercised?** - Do tests cover failure modes?
4. **Do bug fixes have regression tests?** - Is the bug preventable from recurring?
5. **Are hotspots adequately tested?** - Do high-risk areas have test coverage?

## Test Classification

### Behavior Tests (Strong)
Tests that verify actual outcomes and side effects:
- Assert on return values that depend on logic
- Verify state changes after operations
- Check error messages and error types
- Validate complex transformations

### Shape Tests (Weak)
Tests that only verify structure exists:
- Assert that function returns `Ok(_)` without checking contents
- Check that collection is non-empty without verifying contents
- Verify type compiles without testing runtime behavior

### Error Path Tests
Tests that exercise failure modes:
- Invalid input handling
- Resource exhaustion
- Timeout behavior
- Malformed data handling

### Property Tests
Generative tests that verify invariants:
- QuickCheck / proptest based
- Fuzzing
- Randomized input testing

## Test Strength Assessment

| Strength | Criteria |
|----------|----------|
| **Strong** | Tests specific behavior, would catch regressions, uses meaningful assertions |
| **Adequate** | Covers happy path, some edge cases, could miss subtle bugs |
| **Weak** | Tautological, only tests structure, easily passes with broken code |

## Example Input

```
<pr_metadata>
PR Number: 251
Title: BrokenPipe fix in LSP harness
Stated Scope: Fix test failures from BrokenPipe during shutdown
</pr_metadata>

<test_diff>
+ #[test]
+ fn test_graceful_shutdown_handles_broken_pipe() {
+     let server = TestServer::new();
+     server.force_disconnect();
+     let result = server.shutdown();
+     assert!(result.is_ok() || matches!(result, Err(LspError::BrokenPipe)));
+ }
+
+ #[test]
+ fn test_server_recovers_from_connection_drop() {
+     let server = TestServer::new();
+     server.simulate_connection_drop();
+     assert!(server.is_recoverable());
+ }
</test_diff>

<test_output>
running 2 tests
test test_graceful_shutdown_handles_broken_pipe ... ok
test test_server_recovers_from_connection_drop ... ok

test result: ok. 2 passed; 0 failed
</test_output>
```

## Example Output

```yaml
analyzer: verification-auditor
pr: 251
timestamp: 2025-01-07T12:00:00Z
coverage: github_plus_agent_logs

test_inventory:
  added: 2
  modified: 0
  removed: 0
  total_tests_after: unknown

test_depth:
  behavior_tests: 1
  shape_tests: 0
  error_path_tests: 2
  property_tests: 0
  integration_tests: 0
  error_path_coverage: yes

test_quality_assessment:
  - test: test_graceful_shutdown_handles_broken_pipe
    type: error_path
    coverage: "Shutdown path when connection is broken"
    strength: strong
    notes: "Tests the specific BrokenPipe scenario that was failing"
  - test: test_server_recovers_from_connection_drop
    type: error_path
    coverage: "Recovery after connection drop"
    strength: adequate
    notes: "Tests recovery but doesn't verify state consistency post-recovery"

mutation_survival:
  score: "N/A"
  mutants_tested: "N/A"
  surviving_mutants: []

invariants:
  added:
    - invariant: "Server handles BrokenPipe without panic"
      enforced_by: test_graceful_shutdown_handles_broken_pipe
      evidence: "tests/harness.rs:45-52"
    - invariant: "Server is recoverable after connection drop"
      enforced_by: test_server_recovers_from_connection_drop
      evidence: "tests/harness.rs:54-58"
  existing_used: []

regression_coverage:
  bugs_addressed: 1
  bugs_with_regression_test: 1
  missing_regression_tests: []

hotspot_coverage:
  - hotspot: "crates/perl-lsp/src/connection.rs"
    tested: yes
    test_references:
      - test_graceful_shutdown_handles_broken_pipe
      - test_server_recovers_from_connection_drop

findings:
  - id: VA-001
    severity: info
    category: missing_error_path
    summary: Recovery test could verify post-recovery state more thoroughly
    evidence:
      - anchor: tests/harness.rs:54-58
        content: "assert!(server.is_recoverable()) - checks bool but not internal state"
    recommendation: Consider adding assertions on server state after recovery
    confidence: medium

summary:
  verdict: pass
  key_findings:
    - Both tests are error-path focused, directly addressing the BrokenPipe issue
    - Regression test exists for the fixed bug
    - Mutation testing not run - recommend adding to CI
  correctness_delta: +1

assumptions:
  - Test output provided is complete
  - No other tests were modified or removed
  - is_recoverable() correctly reflects server state
```

## Trust Model

### Can Be Inferred (High Confidence)
- Test count changes from diff
- Test file structure and naming
- Presence of assertions and their targets
- Error path coverage from test names and code

### Can Be Inferred (Medium Confidence)
- Test strength (requires understanding what code does)
- Behavior vs shape test classification
- Hotspot coverage alignment

### Cannot Be Inferred
- Actual mutation survival (requires running cargo-mutants)
- Whether tests are flaky (requires multiple runs)
- Coverage percentage (requires instrumentation)
- Whether tests actually prevent the bugs they claim to

### Red Flags to Note
- Tests with no assertions or only `assert!(true)`
- Tests that only check `is_ok()` without verifying contents
- Bug fixes without corresponding regression tests
- High-risk hotspots with no test coverage
- Tests that test implementation details instead of behavior

## Integration Notes

Verification Auditor uses:
- **Diff Scout output**: Hotspots to prioritize coverage checks
- **Test receipts**: Actual pass/fail status and output

Verification Auditor feeds into:
- **Dossier synthesis**: Correctness delta for cover sheet
- **Factory delta**: New invariants that should be gated

For mutation testing, run `cargo mutants` separately and provide output.
