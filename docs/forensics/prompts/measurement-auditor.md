# Measurement Auditor Prompt

## Purpose

The Measurement Auditor analyzer validates **Metrics Integrity** - auditing whether the numbers reported in PR forensics dossiers, status updates, and receipts actually match what was measured. It detects theater metrics, validates provenance chains, and ensures reproducibility.

**Quality Surface**: Measurement Honesty

## Required Inputs

Provide the following context to the analyzer:

### 1. Dossier Draft or Status Update
```
<dossier_draft>
[The document containing metrics claims to be audited]
[PR forensics dossier, CURRENT_STATUS.md update, or receipt]
</dossier_draft>
```

### 2. Tool Outputs (Receipts)
```
<tool_receipts>
[Raw output from measurement tools:]
[- cargo test output with counts]
[- cargo mutants results]
[- git log/diff output]
[- CI run logs]
[- just status-check output]
[- bash scripts/ignored-test-count.sh output]
</tool_receipts>
```

### 3. Git Context
```
<git_context>
[Commit SHAs being compared (base and head)]
[Merge base information if applicable]
[Output from: git log --oneline base..head]
[Output from: git diff --stat base..head]
</git_context>
```

### 4. CI/Gate Outputs (if available)
```
<ci_outputs>
[CI run results with metrics]
[Local gate outputs: just ci-gate]
[Benchmark results if performance claimed]
</ci_outputs>
```

### 5. Measurement Commands Used
```
<measurement_commands>
[Commands that were run to obtain metrics]
[Scripts invoked for measurements]
[Environment variables used (e.g., RUST_TEST_THREADS)]
</measurement_commands>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: measurement-auditor
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

provenance_audit:
  metrics_with_source:
    - metric: <metric name>
      claimed_value: <value in dossier>
      source: <tool/command that produced it>
      evidence_anchor: <where raw output appears>
      verified: <yes|no|partial>
      confidence: <high|medium|low>
  metrics_without_source:
    - metric: <metric name>
      claimed_value: <value>
      severity: <P1|P2|P3>
      notes: <why source is missing>
  unanchored_claims:
    - claim: <statement without evidence>
      location: <where claim appears>
      severity: <P1|P2|P3>

reproducibility:
  commands_provided: <true|false>
  receipts_present: <true|false>
  gaps:
    - metric: <metric name>
      missing: <command|receipt|both>
      impact: <prevents_reproduction|reduces_confidence>
      severity: <P1|P2|P3>
  reproducibility_section_quality: <excellent|adequate|weak|missing>

theater_detection:
  cherry_picked:
    - metric: <metric name>
      suspected_reason: <why this seems cherry-picked>
      evidence: <what's suspicious>
      severity: <P1|P2|P3>
      confidence: <high|medium|low>
  inflated:
    - metric: <metric name>
      claimed_value: <what dossier says>
      actual_value: <what receipts show>
      inflation_factor: <percentage or description>
      severity: <P1|P2|P3>
  misleading_deltas:
    - metric: <metric name>
      issue: <misleading_comparison|wrong_baseline|unfair_context>
      explanation: <what's misleading>
      severity: <P1|P2|P3>

coverage_honesty:
  explicit_not_measured:
    - metric: <metric name>
      explicitly_stated: <yes|no>
      evidence: <where "N/A" or "not measured" appears>
  implicit_gaps:
    - category: <what wasn't measured>
      should_be_measured: <yes|no|maybe>
      severity: <P1|P2|P3>
      notes: <explanation>
  unknown_unknowns:
    - area: <what might be missing>
      confidence: <high|medium|low>

delta_correctness:
  base_sha_verified: <true|false|N/A>
  head_sha_verified: <true|false|N/A>
  merge_strategy_accounted: <true|false|N/A>
  issues:
    - issue: <wrong_base|wrong_head|merge_not_considered|SHAs_missing>
      evidence: <what's wrong>
      impact: <on metric accuracy>
      severity: <P1|P2|P3>

calculation_audit:
  - metric: <metric name>
    formula_stated: <yes|no>
    inputs_traceable: <yes|partial|no>
    calculation_verified: <yes|no|cannot_verify>
    discrepancies:
      - location: <where calculation appears>
        expected: <correct value>
        actual: <claimed value>
        severity: <P1|P2|P3>

findings:
  - id: <unique_id, e.g., "MA-001">
    severity: <P1|P2|P3|info>
    category: <missing_provenance|theater_metric|calculation_error|unreproducible|coverage_gap|delta_error>
    summary: <one line>
    evidence:
      - anchor: <file:line or output section>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings:
    - <bullet 1>
    - <bullet 2>
  measurement_integrity_assessment: <honest|concerning|theater>

assumptions:
  - <what was assumed>
```

## Key Questions Answered

1. **Does every number have a clear source?** - Can we trace metrics back to tool output?
2. **Can measurements be reproduced?** - Are commands and receipts sufficient to re-run?
3. **Are any metrics inflated or cherry-picked?** - Do numbers represent honest measurement?
4. **Is "not measured" explicitly stated?** - Are coverage gaps acknowledged?
5. **Are delta calculations correct?** - Do base/head comparisons use correct SHAs and account for merge strategy?

## Provenance Requirements

### Strong Provenance (High Confidence)

Metrics with clear source chain:
- Test count from `cargo test` output excerpt
- Mutation score from `cargo mutants` output with summary
- Line count from `git diff --stat` output
- CI timing from linked CI run
- Ignored test count from `.ignored-baseline` file or script output

### Weak Provenance (Low Confidence)

Metrics without clear source:
- "Improved performance" without benchmark results
- "Better coverage" without coverage tool output
- "Fixed N bugs" without issue references
- Percentages calculated without showing inputs
- Deltas without base/head SHAs

### No Provenance (Unverifiable)

Unanchored claims:
- Qualitative assessments presented as metrics
- Aggregations without component values
- Historical comparisons without historical data
- "Estimated" values without estimation method

## Theater Metric Patterns

### Cherry-Picking Red Flags

- Reporting only passing tests, not total test suite
- Highlighting one fast operation, ignoring slow paths
- Showing coverage for one module, not workspace
- Reporting mutation score for subset of code

### Inflation Patterns

- Rounding up percentages aggressively (87.3% → 90%)
- Counting tests that were commented out as "added"
- Including unrelated changes in delta metrics
- Presenting LOC as proxy for complexity/value

### Misleading Comparisons

- Comparing optimized build to debug baseline
- Using wrong merge base for delta calculation
- Comparing different test configurations (threaded vs single)
- Cherry-picking time window for performance comparison

## Coverage Honesty Assessment

### Excellent

- Every major quality dimension addressed
- "Not measured" explicitly stated with reason
- Known limits section acknowledges gaps
- Reproducibility commands provided

### Adequate

- Most quality dimensions covered
- Some implicit "not measured" (omitted metrics)
- Basic reproducibility possible
- Minor gaps acknowledged

### Weak

- Only subset of dimensions measured
- Many implicit gaps
- Reproducibility unclear
- No acknowledgment of limits

### Missing

- No coverage honesty section
- Metrics presented without context
- Unknown unknowns not considered
- Theater metrics suspected

## Delta Correctness

### What to Check

For any before/after comparison:

1. **Base SHA verified** - Is the "before" state clearly identified?
2. **Head SHA verified** - Is the "after" state clearly identified?
3. **Merge base correct** - For PR comparisons, was merge base computed correctly?
4. **Context preserved** - Are environmental factors (flags, threading) constant?

### Common Delta Errors

- Using wrong branch as base (master vs actual merge base)
- Comparing HEAD to parent commit when PR has multiple commits
- Not accounting for concurrent changes to base branch
- Comparing different build configurations

## Calculation Verification

### Verifiable Calculations

When inputs and formula are clear:
- `(27 / 33) * 100 = 82%` (LSP coverage)
- `added: 15, removed: 3, net: +12` (test delta)
- `base: 145, head: 178, delta: +33` (mutation score change)

### Unverifiable Calculations

When inputs or formula unclear:
- "Improved by 15%" without showing base/head values
- Aggregates without components
- Weighted averages without weights stated
- Percentiles without distribution data

## Example Input

```
<pr_metadata>
PR Number: 259
Title: Name span for LSP navigation
</pr_metadata>

<dossier_draft>
## Cover Sheet

**Quality Deltas**:
| Surface | Delta | Notes |
|---------|-------|-------|
| Correctness | +2 | 11 comprehensive span tests added |

**Budget**:
| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 4h30m | Git log analysis |
| Tests Added | 11 | name_spans_special_test.rs |

### Verification (receipts)
- 11 tests in name_spans_special_test.rs: all passing
- Mutation testing: not run
</dossier_draft>

<tool_receipts>
$ cargo test -p perl-parser --test name_spans_special_test
running 11 tests
test test_begin_phase_name_span ... ok
test test_end_phase_name_span ... ok
[... 9 more tests ...]

test result: ok. 11 passed; 0 failed; 0 ignored

$ git log --oneline --date=short 90e47a4f^..90e47a4f
90e47a4f feat(parser): add name_span for precise LSP navigation in phase blocks
</tool_receipts>

<git_context>
Base SHA: a1b2c3d (master before PR)
Head SHA: 90e47a4f (PR merge commit)
Merge base: a1b2c3d (fast-forward)
</git_context>
```

## Example Output

```yaml
analyzer: measurement-auditor
pr: 259
timestamp: 2025-01-07T12:00:00Z
coverage: receipts_included

provenance_audit:
  metrics_with_source:
    - metric: "Tests Added"
      claimed_value: "11"
      source: "cargo test output"
      evidence_anchor: "tool_receipts section, line 4-15"
      verified: yes
      confidence: high
    - metric: "DevLT"
      claimed_value: "4h30m"
      source: "Git log analysis"
      evidence_anchor: "Not provided in receipts"
      verified: no
      confidence: low
  metrics_without_source:
    - metric: "Correctness Delta +2"
      claimed_value: "+2"
      severity: P3
      notes: "Qualitative assessment, not a measured metric"
  unanchored_claims:
    - claim: "11 comprehensive span tests"
      location: "Quality Deltas table"
      severity: P3

reproducibility:
  commands_provided: true
  receipts_present: true
  gaps:
    - metric: "DevLT"
      missing: receipt
      impact: reduces_confidence
      severity: P2
  reproducibility_section_quality: adequate

theater_detection:
  cherry_picked: []
  inflated: []
  misleading_deltas: []

coverage_honesty:
  explicit_not_measured:
    - metric: "Mutation testing"
      explicitly_stated: yes
      evidence: "Verification section: 'Mutation testing: not run'"
  implicit_gaps:
    - category: "Performance impact"
      should_be_measured: maybe
      severity: P3
      notes: "Added 11 tests but no timing impact reported"
  unknown_unknowns:
    - area: "Integration test coverage"
      confidence: medium

delta_correctness:
  base_sha_verified: true
  head_sha_verified: true
  merge_strategy_accounted: true
  issues: []

calculation_audit:
  - metric: "Tests Added"
    formula_stated: no
    inputs_traceable: yes
    calculation_verified: yes
    discrepancies: []

findings:
  - id: MA-001
    severity: P2
    category: missing_provenance
    summary: DevLT claim lacks measurement receipt
    evidence:
      - anchor: "dossier_draft Budget table"
        content: "DevLT | 4h30m | Git log analysis"
    recommendation: "Provide git log output or timestamp analysis showing 4h30m calculation"
    confidence: high

  - id: MA-002
    severity: P3
    category: unreproducible
    summary: Correctness delta calculation method not documented
    evidence:
      - anchor: "dossier_draft Quality Deltas"
        content: "Correctness | +2"
    recommendation: "Document rubric for +2 assessment or clarify this is qualitative"
    confidence: medium

  - id: MA-003
    severity: info
    category: coverage_gap
    summary: Performance impact not measured
    evidence:
      - anchor: "dossier_draft"
        content: "11 tests added but no timing delta reported"
    recommendation: "Consider measuring test suite timing before/after if relevant"
    confidence: low

summary:
  verdict: warn
  key_findings:
    - Test count metric properly anchored with cargo test receipt
    - DevLT claim lacks provenance (P2) - needs git log receipt
    - Mutation testing explicitly marked as not run (good honesty)
  measurement_integrity_assessment: honest

assumptions:
  - Git log analysis is valid method for DevLT if timestamps provided
  - Correctness delta is qualitative assessment, not measured metric
  - Test output provided is complete and unedited
```

## Trust Model

### Can Be Verified (High Confidence)

- Exact match between claimed test count and cargo test output
- Line counts from git diff --stat
- Commit SHAs from git log
- CI timing from linked run logs
- Calculation arithmetic (when inputs shown)

### Can Be Inferred (Medium Confidence)

- Whether metric is measurable or qualitative
- Whether omission is intentional or oversight
- Whether comparison baseline is appropriate
- Whether calculation method is sound

### Cannot Be Verified

- Quality of measurement tools themselves
- Whether receipts are edited or complete
- Historical data not provided in inputs
- Future claims ("will improve X")
- Subjective assessments ("comprehensive coverage")

### Red Flags to Note

- Metrics without any provenance statement
- Percentages without showing numerator/denominator
- Performance claims without benchmark receipts
- Deltas without base/head identification
- "Estimated" without estimation method
- Qualitative assessments presented as measurements
- Receipts that contradict claims

## Integration Notes

Measurement Auditor uses:
- **All analyzer outputs**: Validates metrics claimed in synthesized dossier
- **Verification Auditor output**: Cross-checks test count claims
- **Policy Auditor output**: Cross-checks metrics drift findings
- **Tool receipts**: Primary evidence for verification

Measurement Auditor feeds into:
- **Dossier publishing decision**: Block on P1 measurement integrity issues
- **Process improvements**: Identify gaps in receipt collection
- **Lessons learned**: Document provenance best practices

Run Measurement Auditor as final quality gate before dossier publication.
