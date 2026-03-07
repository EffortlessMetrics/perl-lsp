# Metrics Provenance Schema

Every metric we present carries provenance. This document defines the schema for truthful metric reporting in AI-native development.

## Core Principle

> **"Show your work."**

A cold reader can validate any claim by following the provenance chain. This is the "sell without selling" posture.

## Provenance Fields

Every metric includes:

| Field | Required | Description |
|-------|----------|-------------|
| `value` | Yes | The metric value or range |
| `kind` | Yes | `measured` \| `derived` \| `estimated` \| `reported` |
| `basis` | Yes | List of inputs used to produce this value |
| `coverage` | Yes | What data sources were consulted |
| `confidence` | Yes | `high` \| `medium` \| `low` |
| `method` | If derived/estimated | How the value was computed |
| `assumptions` | If estimated | What assumptions were made |

## Kind Definitions

### `measured`
Direct observation from a reproducible source.

```yaml
value: 12m
kind: measured
basis: [workflow_run_abc123]
coverage: github_actions
confidence: high
```

### `derived`
Algorithmically computed from measured inputs.

```yaml
value: 87%
kind: derived
basis: [cargo_mutants_output, test_run_xyz]
method: "killed_mutants / total_mutants"
coverage: local_gate
confidence: high
```

### `estimated`
Bounded inference from available signals.

```yaml
value: 60-90m
kind: estimated
basis: [pr_thread, commit_topology, gate_failures]
method: decision_weighted_v1
assumptions: [decision_weights_from_rubric, no_agent_logs_available]
coverage: github_only
confidence: medium
```

### `reported`
Human-attested value.

```yaml
value: 45m
kind: reported
basis: [developer_self_report]
coverage: self_attested
confidence: medium
```

## Coverage Levels

| Level | Sources Consulted | Typical Confidence |
|-------|-------------------|-------------------|
| `receipts_included` | Agent logs, token receipts, full audit trail | high |
| `github_plus_agent_logs` | GitHub + Claude Code session logs | high-medium |
| `github_only` | PR thread, commits, CI checks only | medium |
| `self_attested` | Human report without corroboration | medium-low |

## Standard Metrics

### Budget Metrics

| Metric | Kind | Typical Basis |
|--------|------|---------------|
| DevLT | estimated | decision events, friction events |
| CI minutes | measured | workflow run duration |
| LLM work units | estimated | iteration count, complexity |
| Wall clock | measured | PR timestamps |

### Quality Metrics

| Metric | Kind | Typical Basis |
|--------|------|---------------|
| Test count delta | derived | git diff on test files |
| Mutation score | measured | cargo-mutants output |
| Clippy warnings | measured | clippy output |
| Coverage delta | derived | coverage tool diff |

### Scope Metrics

| Metric | Kind | Typical Basis |
|--------|------|---------------|
| Files changed | measured | git diff |
| Lines delta | measured | git diff --stat |
| Crates touched | derived | file paths |
| Hotspot list | derived | churn analysis |

## JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["value", "kind", "basis", "coverage", "confidence"],
  "properties": {
    "value": {
      "oneOf": [
        { "type": "number" },
        { "type": "string" },
        { "type": "object", "properties": { "low": { "type": "number" }, "high": { "type": "number" } } }
      ]
    },
    "kind": {
      "enum": ["measured", "derived", "estimated", "reported"]
    },
    "basis": {
      "type": "array",
      "items": { "type": "string" }
    },
    "coverage": {
      "enum": ["receipts_included", "github_plus_agent_logs", "github_only", "self_attested"]
    },
    "confidence": {
      "enum": ["high", "medium", "low"]
    },
    "method": { "type": "string" },
    "assumptions": {
      "type": "array",
      "items": { "type": "string" }
    }
  }
}
```

## Markdown Format

For cover sheets and dossiers, use this format:

### Compact (inline)
```
DevLT: 60–90m (estimated; github_only; medium; 4 decisions + 2 friction)
```

### Expanded (table)
```markdown
| Metric | Value | Kind | Confidence | Basis |
|--------|-------|------|------------|-------|
| DevLT | 60–90m | estimated | medium | 4 decision events, 2 friction loops |
| CI | 12m | measured | high | workflow run #abc123 |
| Tests added | +15 | derived | high | git diff --stat |
```

### Full (for dossiers)
```markdown
#### DevLT
- **Value:** 60–90m
- **Kind:** estimated
- **Basis:** PR thread (3 design comments), commit topology (feat/fix/test pattern), 2 gate failures
- **Method:** decision_weighted_v1 (see DEVLT_ESTIMATION.md)
- **Coverage:** github_only
- **Confidence:** medium
- **Assumptions:** No agent logs available; weights from calibration v1
```

## Anti-Patterns

### Don't Do This

❌ `DevLT: unknown`
❌ `Compute: some`
❌ `Time: ~2 hours` (no provenance)
❌ `This took about a day` (vague, no basis)

### Do This Instead

✅ `DevLT: 45–90m (estimated; github_only; low; sparse thread, 2 visible decisions)`
✅ `CI: ~15m (estimated; local gate elapsed; no workflow run available)`
✅ `DevLT: 120–180m (estimated; github_plus_agent_logs; medium; 6 decisions, 3 friction loops)`

## Validation Rules

When reviewing metrics:

1. **Kind must match basis**: Can't claim "measured" without a measurement source
2. **Range must reflect confidence**: Low confidence → wider range
3. **Coverage must be declared**: Reader needs to know what was consulted
4. **Assumptions must be explicit**: For estimates, state what you assumed

## Evolution

This schema will evolve:

1. **v1 (current)**: Manual provenance annotation
2. **v2 (planned)**: Tooling extracts provenance from receipts
3. **v3 (future)**: Automated validation of provenance chains

## See Also

- [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md) - Decision-weighted DevLT method
- [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md) - Full dossier template
- [`QUALITY_SURFACES.md`](QUALITY_SURFACES.md) - Quality metric definitions
