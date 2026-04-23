# Chronologist Prompt

## Purpose

The Chronologist analyzer constructs a **temporal topology** of a PR or PR sequence, producing a convergence narrative that explains how work evolved from initial scope through final merge. It identifies decision points, scope changes, friction events, and the overall development rhythm.

**Quality Surface**: Cross-cutting (supports all surfaces through temporal context)

## Required Inputs

Provide the following context to the analyzer:

### 1. Commit History with Timestamps
```
<commit_history>
[Git log with dates and messages]
[e.g., git log --format="%h %ad %s" --date=short <base>..HEAD]
</commit_history>
```

### 2. PR Timeline Events
```
<pr_events>
[PR creation, comments, reviews, status checks, merge]
[Include timestamps for each event]
</pr_events>
```

### 3. Issue/Discussion Context (if available)
```
<issue_context>
[Linked issues, discussion threads, design decisions]
</issue_context>
```

### 4. CI/Gate Runs
```
<ci_timeline>
[CI run timestamps, pass/fail sequence]
</ci_timeline>
```

### 5. Related PRs (for PR sequences)
```
<related_prs>
[Other PRs in the sequence with their timelines]
</related_prs>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: chronologist
pr: <number or "NNN-NNN" for sequences>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

temporal_topology:
  start_date: <ISO8601>
  end_date: <ISO8601>
  wall_clock_duration: <e.g., "3 days" or "4 hours">
  active_periods:
    - start: <ISO8601>
      end: <ISO8601>
      activity: <description>

convergence_narrative:
  phases:
    - phase: <number>
      name: <e.g., "Initial implementation", "Bug fix", "Polish">
      start: <ISO8601>
      end: <ISO8601>
      commits: [<list of commit SHAs>]
      summary: <what happened in this phase>
      decision_points: [<list of decisions made>]

scope_evolution:
  initial_scope: <what PR originally claimed to do>
  final_scope: <what PR actually did>
  scope_changes:
    - change: <description of scope change>
      when: <ISO8601 or commit SHA>
      reason: <why scope changed, if known>
      direction: <expanded|contracted|shifted>

friction_timeline:
  - event: <description>
    when: <ISO8601>
    type: <gate_fail|review_feedback|scope_expansion|wrongness_discovered|flaky_test>
    resolution: <how it was resolved>
    duration: <time spent on this friction>

decision_sequence:
  - decision: <what was decided>
    when: <ISO8601 or commit SHA>
    evidence: <commit message, comment, or code change>
    type: <scope|design|verification|acceptance|prevention>

commit_rhythm:
  total_commits: <count>
  commits_per_day: <average>
  longest_gap: <duration between commits>
  busiest_period: <time range>
  pattern: <burst|steady|sporadic>

dependency_sequence:
  - pr: <PR number>
    relationship: <blocks|blocked_by|related|supersedes>
    timing: <before|concurrent|after>

findings:
  - id: <unique_id, e.g., "CHR-001">
    severity: <P1|P2|P3|info>
    category: <scope_creep|stalled_progress|friction_loop|decision_reversal|timing_anomaly>
    summary: <one line>
    evidence:
      - anchor: <commit SHA or timestamp>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings:
    - <bullet 1>
    - <bullet 2>
  convergence_quality: <clean|bumpy|troubled>

assumptions:
  - <what was assumed>
```

## Key Questions Answered

1. **How did the PR evolve?** - What was the journey from start to merge?
2. **What decisions were made and when?** - What's the decision sequence?
3. **What friction occurred?** - Where did progress stall or loop?
4. **Did scope change?** - How does final scope compare to initial intent?
5. **What's the commit rhythm?** - Burst vs. steady vs. sporadic work?

## Phase Identification

### Common Phase Patterns

| Phase Name | Characteristics |
|------------|-----------------|
| **Initial implementation** | First commits, core functionality |
| **Test addition** | Adding tests after initial code |
| **Review response** | Changes responding to feedback |
| **Bug fix** | Fixing issues discovered during review |
| **Polish** | Final cleanup, docs, formatting |
| **Scope expansion** | Adding features not in original scope |
| **Rollback** | Reverting or undoing previous changes |

## Friction Event Types

| Type | Description | Typical Duration |
|------|-------------|------------------|
| `gate_fail` | CI or local gate failure | 5-30 min investigation |
| `review_feedback` | Changes requested by reviewer | 15-60 min response |
| `scope_expansion` | Unexpected dependency discovered | 30-120 min new work |
| `wrongness_discovered` | Bug found during development | 20-60 min fix + prevention |
| `flaky_test` | Non-deterministic failure | 15-45 min investigation |

## Convergence Quality

| Quality | Criteria |
|---------|----------|
| **Clean** | Linear progress, minimal friction, scope stable |
| **Bumpy** | Some friction events, but resolved promptly |
| **Troubled** | Multiple friction loops, scope changes, reversals |

## Example Input

```
<pr_metadata>
PR Numbers: 251-252-253
Title: Test harness hardening sequence
</pr_metadata>

<commit_history>
a1b2c3d 2025-01-05 feat(lsp): initial harness changes
e4f5g6h 2025-01-05 test(lsp): add resilience tests
i7j8k9l 2025-01-06 fix(lsp): handle BrokenPipe (CI failed)
m0n1o2p 2025-01-06 fix(lsp): correct error code handling
q3r4s5t 2025-01-06 test(lsp): add regression test
u6v7w8x 2025-01-07 docs: update threading guidance
</commit_history>

<ci_timeline>
2025-01-05 14:00 - PR #251 opened
2025-01-05 14:15 - CI run #1 - FAILED (BrokenPipe)
2025-01-06 09:00 - PR #252 opened (fix)
2025-01-06 09:20 - CI run #2 - PASSED
2025-01-06 11:00 - PR #253 opened (docs)
2025-01-06 11:10 - CI run #3 - PASSED
2025-01-07 10:00 - All PRs merged
</ci_timeline>
```

## Example Output

```yaml
analyzer: chronologist
pr: "251-252-253"
timestamp: 2025-01-07T12:00:00Z
coverage: github_only

temporal_topology:
  start_date: 2025-01-05T14:00:00Z
  end_date: 2025-01-07T10:00:00Z
  wall_clock_duration: "2 days"
  active_periods:
    - start: 2025-01-05T14:00:00Z
      end: 2025-01-05T15:00:00Z
      activity: "Initial implementation and test addition"
    - start: 2025-01-06T09:00:00Z
      end: 2025-01-06T12:00:00Z
      activity: "Bug fix and documentation"
    - start: 2025-01-07T10:00:00Z
      end: 2025-01-07T10:30:00Z
      activity: "Final merge"

convergence_narrative:
  phases:
    - phase: 1
      name: "Initial implementation"
      start: 2025-01-05T14:00:00Z
      end: 2025-01-05T14:30:00Z
      commits: ["a1b2c3d", "e4f5g6h"]
      summary: "Core harness changes with tests, but CI revealed BrokenPipe issue"
      decision_points:
        - "Decision to add resilience tests upfront"
    - phase: 2
      name: "Friction resolution"
      start: 2025-01-06T09:00:00Z
      end: 2025-01-06T10:00:00Z
      commits: ["i7j8k9l", "m0n1o2p", "q3r4s5t"]
      summary: "Fixed BrokenPipe handling, added regression test"
      decision_points:
        - "Decision to treat BrokenPipe as recoverable"
        - "Decision to add regression test for the fix"
    - phase: 3
      name: "Documentation"
      start: 2025-01-06T11:00:00Z
      end: 2025-01-06T11:30:00Z
      commits: ["u6v7w8x"]
      summary: "Added threading guidance to prevent future issues"
      decision_points:
        - "Decision to document threading constraints"

scope_evolution:
  initial_scope: "Improve test harness resilience"
  final_scope: "Improve test harness resilience + BrokenPipe handling + threading docs"
  scope_changes:
    - change: "Added BrokenPipe error handling"
      when: 2025-01-06T09:00:00Z
      reason: "CI failure revealed gap"
      direction: expanded
    - change: "Added threading documentation"
      when: 2025-01-06T11:00:00Z
      reason: "Prevention mechanism for discovered issue"
      direction: expanded

friction_timeline:
  - event: "CI failure on BrokenPipe"
    when: 2025-01-05T14:15:00Z
    type: gate_fail
    resolution: "Fixed in PR #252"
    duration: "~18 hours (overnight)"

decision_sequence:
  - decision: "Add resilience tests alongside harness changes"
    when: "a1b2c3d"
    evidence: "Tests added in same commit sequence as implementation"
    type: verification
  - decision: "Treat BrokenPipe as recoverable error"
    when: "i7j8k9l"
    evidence: "Commit message: fix(lsp): handle BrokenPipe"
    type: design
  - decision: "Add threading documentation as prevention"
    when: "u6v7w8x"
    evidence: "docs: update threading guidance"
    type: prevention

commit_rhythm:
  total_commits: 6
  commits_per_day: 3
  longest_gap: "~18 hours (overnight between day 1 and 2)"
  busiest_period: "2025-01-06 09:00-12:00"
  pattern: burst

dependency_sequence:
  - pr: 251
    relationship: blocks
    timing: before
  - pr: 252
    relationship: blocked_by
    timing: after
  - pr: 253
    relationship: related
    timing: after

findings:
  - id: CHR-001
    severity: info
    category: scope_creep
    summary: Scope expanded from resilience to include error handling and docs
    evidence:
      - anchor: 2025-01-06T09:00:00Z
        content: "New PR opened to address CI failure discovered in #251"
    recommendation: Consider whether scope changes indicate planning gap
    confidence: high

summary:
  verdict: pass
  key_findings:
    - Clean convergence despite friction event (CI failure)
    - Scope expansion was reactive but appropriate
    - Prevention mechanism added (documentation)
  convergence_quality: bumpy

assumptions:
  - Commit timestamps are accurate
  - CI timeline reflects actual run times
  - Overnight gap was intentional (not blocked)
```

## Trust Model

### Can Be Inferred (High Confidence)
- Commit sequence and timestamps
- PR creation and merge times
- CI pass/fail sequence
- Scope from commit messages and diff

### Can Be Inferred (Medium Confidence)
- Phase boundaries (heuristic from commit patterns)
- Friction duration (gap between failure and resolution)
- Decision types from commit message prefixes

### Cannot Be Inferred
- Actual human attention during gaps
- Off-GitHub discussions affecting decisions
- Whether overnight gaps were intentional
- Real-time collaboration patterns

### Red Flags to Note
- Large scope expansion without explicit decision
- Multiple friction loops on same issue
- Decision reversals mid-PR
- Long gaps with no visible activity
- Commits that undo previous commits

## Integration Notes

Chronologist uses:
- **Git history**: Primary source for temporal data
- **GitHub events**: PR timeline and CI runs

Chronologist feeds into:
- **Decision Extractor**: Decision sequence for DevLT estimation
- **Dossier synthesis**: Convergence narrative and friction summary
- **Lessons learned**: Patterns to avoid in future

For PR sequences, combine timelines from all related PRs.
