# Specification: No-Assertion Test Triage (Phase 1)

## Feature Description

Conduct a systematic triage of ~620 test functions across 8 crates that were flagged by issue #3237's audit as having "no assertion machinery". The goal is to classify each sampled test into one of three categories and produce a representative distribution estimate for the entire corpus.

**This is Phase 1 (triage) only.** No code changes are made. Phase 2 (remediation) depends on this triage's findings.

## Classification Categories

| Category | Description |
|----------|-------------|
| **(a) Missing Assertion** | Test calls a function, discards result, and provides no mechanism to verify correctness |
| **(b) Intentional Smoke Test** | Test verifies a code path does not panic; uses helper with embedded panic-on-failure or explicit smoke test documentation |
| **(c) Helper Misclassified** | Function marked `#[test]` but functions as a utility called by other tests |

## Non-Goals (Out of Scope for Phase 1)

1. No remediation actions (adding assertions, adding SMOKE TEST comments, refactoring helpers)
2. No changes to test infrastructure (`perl-test-must`, `test_parse` helpers, etc.)
3. No validation of issue #3259's specific file:line citations (they are unreliable per verification agent)
4. No modification of CI configuration

## Acceptance Criteria

### AC1: Stratified Sample
- [ ] **Sample size**: Minimum 40 tests, maximum 60 tests
- [ ] **Coverage**: All 8 crates represented proportionally by no-assertion count
- [ ] **Documentation**: Each sample includes:
  - Exact file path and line number
  - Test function name
  - Snippet of relevant code (5-10 lines)
  - Classification decision with reasoning
  - Confidence level (confident / uncertain)

### AC2: Classification Summary Table
- [ ] Markdown table with columns: `file:line | test_name | category | reasoning | confidence`
- [ ] Summary row with per-category counts and percentages
- [ ] 90% confidence interval for (a):(b):(c) distribution estimate

### AC3: Ambiguity Handling
- [ ] Tests that cannot be confidently classified are labeled "uncertain" with both plausible categories noted
- [ ] Decision tree applied consistently (documented in ADR-0032)

### AC4: Effort Estimation
- [ ] Per-category remediation effort estimates:
  - Category (a): Estimated time to add explicit assertions
  - Category (b): Estimated time to add `/// SMOKE TEST:` documentation comments
  - Category (c): Estimated time to refactor helpers to `#[cfg(test)]` module

### AC5: GitHub Issue Update
- [ ] Triage report posted as comment on issue #3259
- [ ] Report includes stratified sample table, distribution estimates, and effort projections
- [ ] Notes which of issue #3259's citations were verified vs. could not be found

### AC6: Reproducibility
- [ ] Commands used to discover and sample tests are documented
- [ ] Random seed (if used) is recorded for reproducibility

## Dependencies

1. **Issue #3237 audit data**: The ~620 count and per-crate breakdown (source of ground truth for stratification)
2. **perl-test-must crate**: `must()`, `must_some()`, `must_err()` helpers (consulted for classification)
3. **No external dependencies**: All work done within existing codebase

## Key Definitions

| Term | Definition |
|------|------------|
| "No assertion" test | Test marked `#[test]` that lacks `assert!`, `assert_eq!`, or mock-side-effect assertions on the result |
| Panic helper | `must()`, `must_some()`, `?` operator — these cause panic on failure but do NOT verify correctness |
| Embedded assertion | `unreachable!()` in a helper that panics if helper's expectation fails (counts as implicit assertion) |
| Smoke test | Test verifying "this code path does not panic" — NOT verifying correctness |

## Verification Method

1. Run the sampling commands documented in the report
2. Spot-check 5 random samples against the classification criteria
3. Verify GitHub issue #3259 contains the posted triage comment

## Open Questions (Deferred to Phase 2)

1. What exactly counts as a "no-assertion" test when `must()` is used with post-result mock assertions?
2. Should category (b) smoke tests be required to have `/// SMOKE TEST:` comments before merge?
3. What's the target (a):(b):(c) ratio that triggers vs. defers remediation?