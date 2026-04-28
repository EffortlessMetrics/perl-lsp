# ADR-0042: No-Assertion Test Triage Classification Framework

## Status
Proposed

## Context

Issue #3259 identified ~620 test functions (~3% of 20,740 total) with no assertion machinery. However:
- Issue #3259's foundational examples cite non-existent files (`perl-ast-utils` crate does not exist)
- No explicit classification criteria distinguish (a) missing assertions from (b) intentional smoke tests from (c) misclassified helpers
- Prior research sampled only perl-parser (174 tests), which may not represent other 7 crates

The work requires human judgment to classify each test, so a systematic framework is needed before sampling begins.

## Decision

### Classification Criteria (Authoritative)

We establish the following explicit rules for categorizing no-assertion tests:

| Category | Definition | Examples |
|----------|------------|----------|
| **(a) Missing Assertion** | Test calls a function, discards result, and does NOT verify behavior through any mechanism | `let _ = parse(src);` with no post-check |
| **(b) Intentional Smoke Test** | Test verifies "code path does not panic" via helper with embedded panic-on-failure | `test_parse() → unreachable!()` or `must()` with no post-result assertion + explicit smoke test naming/comment |
| **(c) Helper Misclassified** | Function marked `#[test]` but is ONLY called by other tests as a utility | `fn setup_parser()` called by multiple tests |

**Key Clarifications:**
- `must()` / `must_some()` / `?` operators are **NOT** explicit assertions — they are panic helpers
- Tests using `must()` without post-result assertions are **category (b)** only if they document smoke test intent
- Mock-side-effect assertions (`assert_eq!(invocations.len(), 1)`) ARE assertions — those tests are correctly classified
- `unreachable!()` in a helper IS an implicit assertion (helper panics if expectation fails)

### Sampling Methodology

**Stratified sampling across all 8 crates:**
1. Weight each crate by its no-assertion count (from issue #3259)
2. Sample proportionally: perl-parser (~28%), perl-parser-core (~15%), perl-lsp (~15%), tree-sitter-perl-rs (~10%), others (~32%)
3. Target: 40-50 samples total
4. Hard cap: 60 samples maximum

### Decision Tree for Ambiguous Cases

```
Is the test function called by other tests as a utility?
  YES → Category (c) Helper Misclassified
  NO
    Does the test have any assert!/assert_eq! on result or side effects?
      YES → Correctly classified (not no-assertion)
      NO
        Does a helper (test_parse, parse_clean) use unreachable! on failure?
          YES → Category (b) if edge case, otherwise re-examine
          NO
            Is result discarded (let _ = ...) with no comment?
              YES → Category (a) Missing Assertion
              NO (result is used somehow)
                Is it a smoke test naming pattern (_smoke, _edge_case)?
                  YES → Category (b) Intentional Smoke Test
                  NO → Category (a) Missing Assertion (ambiguous)
```

## Consequences

### Benefits
- Explicit criteria reduce classification subjectivity
- Stratified sampling gives representative distribution estimate across codebase
- Triage output guides Phase 2 effort allocation

### Tradeoffs
- Phase 1 triage delays code changes by ~1-2 weeks
- Ambiguous cases will exist (documented as "uncertain")
- Issue #3259's specific citations cannot be used — must re-discover independently

### Risks
1. **Misclassification**: Despite criteria, borderline cases will require judgment calls
2. **Sample bias**: Even stratified sampling may not capture all patterns
3. **Scope expansion**: `?` operator tests could significantly increase category (a) count

## Alternatives Considered

### Alternative 1: Trust Issue #3259's File Citations
- **Rejected**: Verification agent confirmed cited files do not exist in repository
- **Impact**: Would produce misleading triage report

### Alternative 2: Single-Crate Deep Dive (perl-parser Only)
- **Rejected**: Plan-reviewer identified sample bias — perl-parser ratio may differ from other crates
- **Impact**: Would underestimate effort for perl-lsp, perl-parser-core, etc.

### Alternative 3: Immediate Remediation Skipping Triage
- **Rejected**: Without classification data, cannot prioritize (a) vs (b) vs (c) remediation
- **Impact**: Wasted effort on well-intentioned but potentially misclassified fixes

## Notes

- This ADR covers **Phase 1 (Triage)** only
- Phase 2 (Remediation) requires separate ADR based on triage findings
- `perl-test-must` crate's `must()`/`must_some()` helpers are intentionally NOT classified as assertions per issue #3259 guidance