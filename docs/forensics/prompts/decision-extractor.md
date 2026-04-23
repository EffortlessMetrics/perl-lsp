# Decision Extractor Prompt

## Purpose

The Decision Extractor analyzer extracts **decision events** and **friction events** from PR artifacts to enable **DevLT estimation**. It identifies scope calls, design calls, verification calls, acceptance calls, and prevention calls that required human judgment.

**Quality Surface**: Budget estimation (cross-cutting)

## Core Concept

> **DevLT is decision time, not commit time.**

In AI-native repos, LLMs do the typing. Humans do:
- **Scope calls**: What's in/out
- **Design calls**: Interfaces, invariants, stability
- **Verification calls**: What evidence is sufficient
- **Acceptance calls**: Merge readiness
- **Prevention calls**: Guardrails after wrongness

This analyzer extracts these decision events to estimate human attention time.

## Required Inputs

Provide the following context to the analyzer:

### 1. PR Description and Comments
```
<pr_content>
[PR description, review comments, discussion]
</pr_content>
```

### 2. Commit Messages
```
<commits>
[Full commit messages with context]
</commits>
```

### 3. Chronologist Output (recommended)
```
<chronologist>
[Output from chronologist analyzer for temporal context]
</chronologist>
```

### 4. CI/Gate History
```
<ci_history>
[CI runs with pass/fail status, especially failures]
</ci_history>
```

### 5. Related Issues/Discussions
```
<issues>
[Linked issues, design discussions]
</issues>
```

### 6. Agent Logs (if available)
```
<agent_logs>
[LLM interaction logs showing decision points]
</agent_logs>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: decision-extractor
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

decision_events:
  - id: <unique_id, e.g., "DEC-001">
    type: <scope|design|verification|acceptance|prevention>
    description: <what was decided>
    evidence:
      - anchor: <commit, comment, or PR section>
        content: <excerpt showing the decision>
    weight_min: <minutes, lower bound>
    weight_max: <minutes, upper bound>
    confidence: <high|medium|low>
    notes: <context or reasoning>

friction_events:
  - id: <unique_id, e.g., "FRC-001">
    type: <gate_fail|flaky_test|measurement_incident|wrongness|scope_expansion|review_feedback>
    description: <what happened>
    evidence:
      - anchor: <CI run, commit, or comment>
        content: <excerpt>
    weight_min: <minutes, lower bound>
    weight_max: <minutes, upper bound>
    resolution: <how it was resolved>
    confidence: <high|medium|low>

budget_calculation:
  decision_events_sum:
    min: <total minutes>
    max: <total minutes>
    count: <number of events>
  friction_events_sum:
    min: <total minutes>
    max: <total minutes>
    count: <number of events>
  raw_total:
    min: <sum of mins>
    max: <sum of maxes>
  coverage_factor: <1.0|1.2|1.5>
  adjusted_total:
    min: <raw_min * coverage_factor>
    max: <raw_max * coverage_factor>

devlt_estimate:
  value: "<min>-<max>m"
  confidence: <high|medium|low>
  coverage: <github_only|github_plus_agent_logs|receipts_included>
  basis: "<N> decision events, <M> friction events"

machine_work_estimate:
  ci_minutes: <measured or estimated>
  llm_work_units: <estimated>
  basis: <how this was determined>

findings:
  - id: <unique_id, e.g., "DE-001">
    severity: <P1|P2|P3|info>
    category: <high_friction|decision_reversal|missing_decision|unusual_pattern>
    summary: <one line>
    evidence:
      - anchor: <location>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings:
    - <bullet 1>
    - <bullet 2>

assumptions:
  - <what was assumed>
```

## Decision Event Weights (Reference)

From `DEVLT_ESTIMATION.md`:

| Event | Weight (min) | Notes |
|-------|-------------|-------|
| Scope boundary set/changed | 8-20 | Defining what's in/out |
| Architectural constraint decision | 10-30 | Interface design, module boundaries |
| Interface stability decision | 8-20 | API surface, breaking changes |
| Verification strategy decision | 8-20 | What tests/evidence is sufficient |
| Risk acceptance/mitigation call | 5-15 | Known limits, debt tracking |
| Design trade-off resolution | 10-25 | Choosing between approaches |
| Acceptance decision | 5-15 | "Is this ready to merge?" |

## Friction Event Weights (Reference)

| Event | Weight (min) | Notes |
|-------|-------------|-------|
| Gate fail requiring interpretation | 5-20 | Understanding why CI failed |
| Flaky/non-deterministic failure | 10-30 | Investigating intermittent issues |
| Measurement integrity incident | 15-45 | Baseline drift, metric confusion |
| Wrongness discovered | 10-40 | Bug found, prevention work needed |
| Scope expansion mid-PR | 10-30 | Unexpected dependency discovered |
| Reviewer feedback cycle | 5-20 | Per round of changes requested |

## Coverage Factors

| Coverage | Factor | When to Use |
|----------|--------|-------------|
| `receipts_included` | 1.0 | Agent logs, token receipts available |
| `github_plus_agent_logs` | 1.2 | GitHub + session logs, no full receipts |
| `github_only` | 1.5 | PR thread, commits, CI only |

## Key Questions Answered

1. **What decisions required human judgment?** - What were the scope, design, verification decisions?
2. **What friction consumed attention?** - Where did progress stall?
3. **What's the DevLT estimate?** - How much human attention time?
4. **What's the machine work?** - CI minutes and LLM work units?
5. **Are there unusual patterns?** - High friction, reversals, missing decisions?

## Example Input

```
<pr_metadata>
PR Number: 259
Title: Test harness hardening
</pr_metadata>

<pr_content>
## Summary
Fix BrokenPipe issues in LSP test harness

## Changes
- Add connection state tracking
- Handle BrokenPipe gracefully
- Add resilience tests
</pr_content>

<commits>
a1b2c3d feat(lsp): add connection state tracking
  - Decided to track connection state explicitly
  - Interface: ConnectionState enum

e4f5g6h fix(lsp): handle BrokenPipe in shutdown
  - Handle BrokenPipe as recoverable
  - Error code 0 for clean shutdown

i7j8k9l test(lsp): add harness resilience tests
  - Tests for graceful shutdown
  - Tests for connection drop recovery
</commits>

<ci_history>
Run #1: FAILED - BrokenPipe in test
Run #2: PASSED - after fix
</ci_history>
```

## Example Output

```yaml
analyzer: decision-extractor
pr: 259
timestamp: 2025-01-07T12:00:00Z
coverage: github_only

decision_events:
  - id: DEC-001
    type: scope
    description: "Scope defined as BrokenPipe handling, not full error overhaul"
    evidence:
      - anchor: "PR description"
        content: "Fix BrokenPipe issues in LSP test harness"
    weight_min: 8
    weight_max: 15
    confidence: high
    notes: "Clear scope boundary in PR title and description"

  - id: DEC-002
    type: design
    description: "Decision to use ConnectionState enum for explicit state tracking"
    evidence:
      - anchor: "commit a1b2c3d"
        content: "Decided to track connection state explicitly"
    weight_min: 10
    weight_max: 20
    confidence: medium
    notes: "Interface design decision visible in commit message"

  - id: DEC-003
    type: design
    description: "Decision to treat BrokenPipe as recoverable error"
    evidence:
      - anchor: "commit e4f5g6h"
        content: "Handle BrokenPipe as recoverable"
    weight_min: 8
    weight_max: 15
    confidence: high
    notes: "Explicit design choice about error handling"

  - id: DEC-004
    type: verification
    description: "Decision on what tests prove the fix works"
    evidence:
      - anchor: "commit i7j8k9l"
        content: "Tests for graceful shutdown, connection drop recovery"
    weight_min: 8
    weight_max: 15
    confidence: medium
    notes: "Verification strategy: behavior tests for both scenarios"

  - id: DEC-005
    type: acceptance
    description: "Acceptance that PR is ready to merge after CI passes"
    evidence:
      - anchor: "CI Run #2"
        content: "PASSED"
    weight_min: 5
    weight_max: 10
    confidence: high
    notes: "Standard acceptance after green CI"

friction_events:
  - id: FRC-001
    type: gate_fail
    description: "Initial CI failure on BrokenPipe"
    evidence:
      - anchor: "CI Run #1"
        content: "FAILED - BrokenPipe in test"
    weight_min: 10
    weight_max: 20
    resolution: "Fixed in subsequent commit"
    confidence: high

budget_calculation:
  decision_events_sum:
    min: 39
    max: 75
    count: 5
  friction_events_sum:
    min: 10
    max: 20
    count: 1
  raw_total:
    min: 49
    max: 95
  coverage_factor: 1.5
  adjusted_total:
    min: 74
    max: 143

devlt_estimate:
  value: "75-145m"
  confidence: medium
  coverage: github_only
  basis: "5 decision events, 1 friction event"

machine_work_estimate:
  ci_minutes: "~12m"
  llm_work_units: "~4 units"
  basis: "2 CI runs, estimated 2 iterations"

findings:
  - id: DE-001
    severity: info
    category: unusual_pattern
    summary: CI failure discovered issue that expanded scope slightly
    evidence:
      - anchor: "CI Run #1"
        content: "Failure revealed need for explicit state tracking"
    recommendation: None - normal friction resolution
    confidence: high

summary:
  verdict: pass
  key_findings:
    - 5 clear decision events identified from PR artifacts
    - 1 friction event (CI failure) resolved promptly
    - DevLT estimate has wide bounds due to github_only coverage

assumptions:
  - No offline discussions affected decisions
  - Commit messages accurately reflect decision points
  - CI failure required investigation time
```

## Trust Model

### Can Be Inferred (High Confidence)
- Decision events from explicit commit messages
- CI failures as friction events
- Acceptance decisions from merge events
- Scope from PR title/description

### Can Be Inferred (Medium Confidence)
- Design decisions from code structure
- Verification decisions from test additions
- Friction duration from timestamp gaps

### Cannot Be Inferred
- Offline or synchronous discussions
- Actual time spent on each decision
- Whether decisions were easy or hard
- LLM interaction quality without agent logs

### Red Flags to Note
- Multiple scope changes (decision churn)
- Design reversals mid-PR
- High friction-to-decision ratio
- Missing verification decisions for critical changes

## Integration Notes

Decision Extractor uses:
- **Chronologist output**: Temporal context and friction timeline
- **All other analyzers**: To understand what decisions were about

Decision Extractor feeds into:
- **Dossier synthesis**: Budget section with provenance
- **Calibration data**: For improving estimation accuracy

For best estimates, provide agent logs when available to reduce coverage factor.
