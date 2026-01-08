# LLM Analyzer Prompt Pack

Structured prompts for specialist analyzers used in PR forensics. Each prompt defines inputs, outputs, and trust boundaries for an LLM-powered analysis pass.

## Core Principle

> **LLMs can synthesize freely; the repo constrains what gets published as truth.**

These prompts enable deep analysis while maintaining epistemic honesty. Each analyzer:
- Operates on bounded inputs
- Produces structured, diffable output
- Declares confidence levels and assumptions
- Anchors findings to evidence

## Available Prompts

| Prompt | Surface | Purpose |
|--------|---------|---------|
| [`diff-scout.md`](diff-scout.md) | Scope | Review map, hotspots, semantic ratio |
| [`design-auditor.md`](design-auditor.md) | Maintainability | Boundaries, coupling, API stability |
| [`verification-auditor.md`](verification-auditor.md) | Correctness | Test depth, mutation survival, regression coverage |
| [`docs-auditor.md`](docs-auditor.md) | Reproducibility | Gate clarity, snippet validity, drift risk |
| [`policy-auditor.md`](policy-auditor.md) | Governance | Catalog drift, metrics sync, schema compliance, guardrail effectiveness |
| [`measurement-auditor.md`](measurement-auditor.md) | Measurement Honesty | Metrics provenance, reproducibility, theater detection, delta correctness |
| [`chronologist.md`](chronologist.md) | Temporal | Convergence narrative, decision timeline |
| [`decision-extractor.md`](decision-extractor.md) | Budget | DevLT estimation from decision events |

## How to Use

### 1. Gather Context

Before invoking an analyzer, collect the required inputs specified in the prompt file.

**Common context sources:**

```bash
# Git diff
git diff <base>..HEAD > diff.txt

# Commit history
git log --oneline --date=short <base>..HEAD > commits.txt

# File histogram
git diff --stat <base>..HEAD > histogram.txt

# Full commit messages
git log --format="=== %h ===%n%B" <base>..HEAD > full_commits.txt
```

**For PR forensics:**
```bash
# PR description and comments
gh pr view <number> --json body,comments > pr_content.json

# CI runs
gh run list --limit 10 --json databaseId,status,conclusion > ci_runs.json
```

### 2. Format Input

Wrap each context source in XML tags as specified in the prompt:

```
<git_diff>
[contents of diff.txt]
</git_diff>

<commit_history>
[contents of commits.txt]
</commit_history>
```

### 3. Invoke Analyzer

Provide the prompt content followed by the formatted input to an LLM. Request output in the specified YAML schema.

**Example invocation pattern:**

```
I need you to act as the Diff Scout analyzer.

[Contents of diff-scout.md]

Here are the inputs:

<pr_metadata>
PR Number: 259
Title: Test harness hardening
</pr_metadata>

<git_diff>
[diff content]
</git_diff>

<commit_history>
[commit log]
</commit_history>

<file_histogram>
[diff --stat output]
</file_histogram>

Please analyze and produce output in the specified YAML schema.
```

### 4. Validate Output

Check that output conforms to schema:
- All required fields present
- Confidence levels declared
- Evidence anchors are specific (file:line, commit SHA)
- Assumptions listed

## Combining Outputs into a Dossier

### Recommended Pipeline

Run analyzers in this order for best results:

```
1. diff-scout        (provides review map for other analyzers)
       |
       v
2. design-auditor    (uses diff-scout hotspots)
   verification-auditor  (uses diff-scout hotspots)
   docs-auditor      (uses diff-scout file categories)
   policy-auditor    (uses diff-scout for governance-relevant changes)
       |
       v
3. chronologist      (uses all findings for temporal context)
       |
       v
4. decision-extractor (uses chronologist timeline)
       |
       v
5. measurement-auditor (validates synthesized dossier - FINAL GATE)
       |
       ├─→ comparable: proceed to publication
       └─→ not_comparable: BLOCK until contract issues fixed
```

### Measurement Auditor (Final Gate)

The measurement-auditor serves as the **final quality gate** before dossier publication. It validates:

1. **Measurement Contract**: Is the comparison valid? (stable/changed/unknown)
2. **Claims Found**: Are PR claims backed by evidence?
3. **Required Fixes**: What must be fixed before publication?

**Hard rule**: If the measurement contract is unstable or unknown, the auditor emits `contract_verdict: not_comparable` and the dossier **must not be published** until issues are resolved.

**Common blocks**:
- Performance claims without benchmark receipts
- Coverage claims without tool output
- Multiplier claims ("4x faster") without absolute numbers
- Delta comparisons using wrong baseline

### Synthesis Rules

When combining analyzer outputs into a dossier:

1. **Verdicts**: If any analyzer returns `fail`, overall is `fail`
2. **Key findings**: Pull P1/P2 findings from all analyzers, limit to 5
3. **Quality deltas**: Aggregate from surface-specific analyzers:
   - Maintainability from design-auditor
   - Correctness from verification-auditor
   - Reproducibility from docs-auditor
   - Governance from policy checks (if run)
4. **Budget**: Pull DevLT estimate from decision-extractor
5. **Convergence**: Pull narrative from chronologist

### Dossier Template

```markdown
## PR #NNN: [Title]

### Cover Sheet

**Verdict**: [pass|warn|fail]

**Quality Deltas**:
| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | [+2...-2] | [from design-auditor] |
| Correctness | [+2...-2] | [from verification-auditor] |
| Reproducibility | [+2...-2] | [from docs-auditor] |
| Governance | [+2...-2] | [if assessed] |

**Budget**:
| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | [X-Ym] | [from decision-extractor] |
| CI | [Zm] | [measured/estimated] |

**Key Findings**:
1. [P1/P2 finding from any analyzer]
2. [P1/P2 finding]
3. [etc.]

### Detailed Analysis

[Embed or link full analyzer outputs]

### Assumptions

[Aggregate assumptions from all analyzers]
```

## Trust Model

### What LLMs Do Well
- Pattern recognition across large diffs
- Classifying code changes by type
- Identifying potential risk areas
- Extracting decision events from text
- Synthesizing narratives from timelines

### What LLMs Cannot Do
- Execute code to verify it works
- Access historical data not provided
- Determine actual human attention time
- Verify claims without evidence
- Know repository-specific context not in input

### Coverage Levels

Every analysis declares its coverage:

| Level | Inputs Available | Confidence Impact |
|-------|------------------|-------------------|
| `receipts_included` | Agent logs, CI output, test results | Narrow bounds |
| `github_plus_agent_logs` | GitHub data + session logs | Moderate bounds |
| `github_only` | PR thread, commits, CI status | Wider bounds |

### Evidence Requirements

Findings are only as strong as their evidence:

| Evidence Type | Strength | Example |
|---------------|----------|---------|
| Code anchor | High | `file.rs:45-50` |
| Commit SHA | High | `a1b2c3d` |
| CI output | High | Link to run |
| Commit message | Medium | Inferred from message |
| Code pattern | Medium | "Multiple `unwrap()` calls" |
| Absence | Low | "No test found for X" |

### Publishing Protocol

1. **LLM synthesizes** - Analyzers produce structured findings
2. **Human reviews** - Operator validates critical findings
3. **Repo constrains** - Only evidence-anchored findings become truth
4. **Provenance preserved** - Coverage and assumptions documented

## Extending the Pack

To add a new analyzer:

1. **Define purpose** - What quality surface does it evaluate?
2. **Specify inputs** - What context does it need?
3. **Design output schema** - What structured data does it produce?
4. **Document key questions** - What does it answer?
5. **Establish trust model** - What can vs. can't be inferred?
6. **Add to pipeline** - Where does it fit in the analysis order?

Follow the format of existing prompts for consistency.

## See Also

- [`../ANALYZER_FRAMEWORK.md`](../../ANALYZER_FRAMEWORK.md) - Framework specification
- [`../QUALITY_SURFACES.md`](../../QUALITY_SURFACES.md) - Quality surface definitions
- [`../DEVLT_ESTIMATION.md`](../../DEVLT_ESTIMATION.md) - DevLT estimation rubric
- [`../FORENSICS_SCHEMA.md`](../../FORENSICS_SCHEMA.md) - Full dossier template
- [`../METRICS_PROVENANCE.md`](../../METRICS_PROVENANCE.md) - Provenance requirements
