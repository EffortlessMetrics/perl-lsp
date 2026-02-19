# DevLT Estimation Rubric

Decision-weighted human attention time estimation for AI-native development.

## Core Principle

**DevLT is decision time, not commit time.**

In AI-native repos, LLMs do the typing. Humans do:
- Scope calls (what's in/out)
- Design calls (interfaces, invariants, stability)
- Verification calls (what evidence is sufficient)
- Acceptance calls (merge readiness)
- Prevention calls (what guardrail gets added after wrongness)

We estimate DevLT from **decision topology**, not wall clock or commit timestamps.

## The Three Clocks

| Clock | What it measures | Source |
|-------|-----------------|--------|
| **Wall clock** | PR created → merged | GitHub timestamps |
| **Machine work** | CI + LLM compute | Run durations, token receipts |
| **DevLT** | Human attention minutes | Decision events + friction events |

Wall clock and machine work are measured. DevLT is estimated.

## DevLT Buckets

Split DevLT into three components:

| Bucket | Description | Examples |
|--------|-------------|----------|
| **Control** | Scope/design/risk decisions | "Should we include X?", "What's the interface?" |
| **Audit** | Reading receipts, interpreting failures | Reviewing test output, gate failures, PR diffs |
| **Ops** | Mechanical publish work | Updating docs, cover sheets, housekeeping |

## Decision Events (weights in minutes)

Events that require human judgment:

| Event | Weight (min) | Notes |
|-------|-------------|-------|
| Scope boundary set/changed | 8–20 | Defining what's in/out |
| Architectural constraint decision | 10–30 | Interface design, module boundaries |
| Interface stability decision | 8–20 | API surface, breaking changes |
| Verification strategy decision | 8–20 | What tests/evidence is sufficient |
| Risk acceptance/mitigation call | 5–15 | Known limits, debt tracking |
| Design trade-off resolution | 10–25 | Choosing between approaches |
| Acceptance decision | 5–15 | "Is this ready to merge?" |

## Friction Events (weights in minutes)

Events that consume attention beyond expected:

| Event | Weight (min) | Notes |
|-------|-------------|-------|
| Gate fail requiring interpretation | 5–20 | Understanding why CI failed |
| Flaky/non-deterministic failure | 10–30 | Investigating intermittent issues |
| Measurement integrity incident | 15–45 | Baseline drift, metric confusion |
| Wrongness discovered | 10–40 | Bug found, prevention work needed |
| Scope expansion mid-PR | 10–30 | Unexpected dependency discovered |
| Reviewer feedback cycle | 5–20 | Per round of changes requested |

## Estimation Formula

```
DevLT = Σ(decision_events × weights) + Σ(friction_events × weights) × coverage_factor
```

**Coverage factors:**
- `receipts_included`: 1.0 (narrow bounds)
- `github_plus_agent_logs`: 1.2 (moderate bounds)
- `github_only`: 1.5 (wider bounds)

## Output Format

Never output "unknown." Always output:

```
DevLT: <low>–<high>m (<confidence>; <coverage>; <basis>)
```

### Examples

**Clean PR, good coverage:**
```
DevLT: 45–75m (high; github_plus_agent_logs; 3 decision events, 0 friction)
```

**Complex PR, limited coverage:**
```
DevLT: 90–180m (medium; github_only; 5 decision events, 3 friction loops)
```

**Mechanization PR with wrongness:**
```
DevLT: 120–200m (medium; receipts_included; 4 decisions, 2 friction + wrongness prevention)
```

## Worked Example

PR #251-253 (Test Harness Hardening):

**Decision events:**
- Scope boundary (harness vs. individual tests): 15m
- Verification strategy (what proves BrokenPipe fixed): 15m
- Interface stability (error code API): 12m
- Acceptance decision: 10m

**Friction events:**
- Gate failures requiring interpretation: 20m (2 × 10m)
- Wrongness discovered + prevention: 30m

**Total:**
- Events sum: 52m (decisions) + 50m (friction) = 102m
- Coverage: github_plus_agent_logs (×1.2)
- Range: 90–130m

**Output:**
```
DevLT: 90–130m (medium; github_plus_agent_logs; 4 decisions + 2 friction + wrongness)
```

## Calibration

To improve estimates over time:

1. **Collect reported DevLT** on ~10 PRs (even rough estimates)
2. **Compare to estimated DevLT** from this rubric
3. **Tune weights** until reported usually falls inside estimated range
4. **Document error band** and revision history

### Calibration Store

Calibration data is stored in [`forensics/calibration/devlt.csv`](forensics/calibration/devlt.csv).

**Fields**:
- `pr` - PR number
- `date` - Analysis date
- `est_lb_min`, `est_ub_min` - Estimated range (minutes)
- `reported_min` - Actual reported DevLT (when available)
- `coverage` - Data coverage level
- `method_id` - Estimation method version
- `notes` - Brief description

**Adding entries**: After each forensics pass, append to the CSV. See [`forensics/calibration/README.md`](forensics/calibration/README.md) for details.

### Calibration Data Collection

#### Per-PR Calibration Record

After each PR that goes through forensics, collect:

```yaml
pr: <number>
# Estimated (from decision-extractor)
estimated_devlt:
  range_min: <minutes>
  range_max: <minutes>
  confidence: low|med|high
  coverage: github_only|github_plus_agent_logs
  basis:
    - <event 1>
    - <event 2>

# Reported (from maintainer, if available)
reported_devlt:
  value: <minutes>
  confidence: low|med|high
  notes: <optional>

# Calibration result
calibration:
  in_range: true|false
  error_minutes: <if reported available>
  error_direction: under|over|accurate
```

**Note:** Calibration requires maintainer-reported values which should be collected during PR review or retrospectives. Reported values can be rough estimates (±15m acceptable).

#### Calibration Log Format

Track calibration over time:

| PR | Est. Range | Reported | In Range? | Error | Notes |
|----|------------|----------|-----------|-------|-------|
| 259 | 45–90 | 60 | ✓ | - | baseline |
| 260 | 30–60 | 75 | ✗ | +15 under | friction underweighted |

#### Weight Adjustment Protocol

When calibration shows systematic bias:

1. **Collect 5+ data points** showing same direction error
2. **Identify which event category** is mis-weighted (decision vs. friction, which type)
3. **Adjust weight by 20%** in correction direction
4. **Document in revision history** with rationale
5. **Re-run next 5 PRs** to validate adjustment

**Example adjustment:**
- If 5 PRs show consistent under-estimation (+20m outside high end)
- And all involve friction events
- Increase friction event weights by 20%
- Document as v2 in revision history

### Calibration Log

Initial calibration from casebook exhibits (v1 weights):

| PR | Type | Estimated | Reported | In Range? | Notes |
|----|------|-----------|----------|-----------|-------|
| #231/232/234 | feature | 60–90m | — | — | Semantic analyzer; 4 decisions, 0 friction |
| #260/264 | hardening | 60–90m | — | — | Mutation hardening; 3 decisions, 2 friction |
| #251-253 | mechanization | 90–130m | — | — | Harness hardening; 4 decisions, 3 friction + wrongness |
| #259 | feature | 45–75m | — | — | Name span; 3 decisions, 0 friction |
| #225/226/229 | feature | 60–90m | — | — | Statement tracker; 4 decisions, 0 friction |

**Status:** Awaiting reported values for calibration. Current estimates use v1 weights.

### Weight Revision History

| Version | Date | Changes | Rationale |
|---------|------|---------|-----------|
| v1 | 2025-01-07 | Initial weights | Heuristic baseline |
| v2 | TBD | Pending calibration data | - |

### Error Band

**Current documented error band:** ±30–50% typical (uncalibrated)

This will narrow after calibration with reported values. Target: ±20% for high-confidence estimates.

### Calibration Procedure

To calibrate from a new PR:

```bash
# 1. Run forensics harvest
scripts/forensics/pr-harvest.sh <PR_NUMBER> -o harvest.yaml

# 2. Run temporal analysis
scripts/forensics/temporal-analysis.sh <PR_NUMBER> -o temporal.yaml

# 3. Estimate DevLT using rubric (count decision + friction events)
# 4. Append to calibration store
echo "275,$(date -I),60,90,,github_plus_agent_logs,devlt_est_v1:decision_weighted,description" >> docs/forensics/calibration/devlt.csv

# 5. After merge, update reported_min column with actual DevLT
# 6. If outside range, adjust weights and document in revision history
```

### Calibration Quality Check

After collecting 5+ reported values:

```bash
# Check calibration accuracy
cd docs/forensics/calibration
grep -v '^#' devlt.csv | awk -F, '
NR>1 && $5!="" {
  total++
  if ($5 >= $3 && $5 <= $4) { in_range++ }
}
END {
  pct = (in_range/total) * 100
  print "In-range: " in_range "/" total " (" pct "%)"
  if (pct < 50) print "WARNING: Calibration needed - adjust weights"
}'
```

## Machine Work Estimation

Alongside DevLT, estimate machine work in stable units:

### CI Minutes (measured when available)
```
CI: <minutes>m (measured from workflow run)
CI: ~<minutes>m (estimated from local gate)
```

### LLM Work Units (estimated)
```
LLM: <low>–<high> work units (<basis>)
```

Work units are iteration-weighted:
- Simple iteration: 1 unit
- Complex iteration with exploration: 2–3 units
- Multi-file refactor iteration: 3–5 units

Convert to cost later when pricing is stable.

## Integration with Cover Sheets

Every PR cover sheet includes:

```markdown
### Budget

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 60–90m | estimated; github_plus_agent_logs; 4 decisions |
| CI | 12m | measured; workflow run #xyz |
| LLM | ~8 units | estimated; 4 iterations × complexity |
```

## See Also

- [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md) - Full dossier template
- [`METRICS_PROVENANCE.md`](METRICS_PROVENANCE.md) - Provenance schema
- [`CASEBOOK.md`](CASEBOOK.md) - Exhibit examples
