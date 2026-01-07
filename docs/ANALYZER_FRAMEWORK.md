# Analyzer Framework

Specialist analyzers for deep PR forensics. Use bounded analyzers instead of one summarizer to get depth without fiction.

## Core Principle

> **LLMs can synthesize freely; the repo constrains what gets published as truth.**

Each analyzer outputs structured findings with explicit evidence anchors. The cover sheet pulls 1–2 lines per section.

## Analyzer Types

| Analyzer | Surface | Key Outputs |
|----------|---------|-------------|
| Diff Scout | Scope | Hotspots, semantic vs generated, review map |
| Design Auditor | Maintainability | Boundary map, coupling delta, API changes |
| Verification Auditor | Correctness | Test depth, mutation survival, invariants |
| Docs Auditor | Reproducibility | Gate clarity, executable snippets, drift |
| Measurement Auditor | Governance | Perf semantics, baseline stability |
| Policy Auditor | Governance | Schema alignment, anti-drift inventory |

## Common Output Schema

Every analyzer produces:

```yaml
analyzer: <name>
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

findings:
  - id: <unique_id>
    severity: <P1|P2|P3|info>
    category: <category>
    summary: <one line>
    evidence:
      - anchor: <file:line or commit or link>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings: <1-3 bullet points>

assumptions:
  - <what was assumed>
```

## 1. Diff Scout

Produces review map and identifies hotspots.

### Inputs
- Git diff
- Commit history
- File change histogram

### Outputs

```yaml
review_map:
  - path: <file path>
    delta: <+X/-Y>
    category: <logic|test|config|docs|generated>
    risk: <high|medium|low>

hotspots:
  - path: <file path>
    reason: <why this is a hotspot>
    lines: <specific line ranges>

commit_topology:
  feat: <count>
  fix: <count>
  test: <count>
  docs: <count>
  chore: <count>

semantic_ratio: <percentage of semantic vs generated/config changes>
```

### Questions Answered
- Where did the diff land?
- What are the riskiest changes?
- Is this mostly semantic work or boilerplate?

## 2. Design/Contract Auditor

Assesses maintainability impact.

### Inputs
- Diff
- `features.toml`
- Public API surface (pub functions/types)
- Dependency changes

### Outputs

```yaml
boundary_changes:
  - module: <path>
    change: <added|removed|modified>
    responsibilities: <before → after>

coupling_delta:
  - from: <module>
    to: <module>
    change: <new|removed|strengthened|weakened>

api_surface:
  public_functions:
    added: [<list>]
    removed: [<list>]
    modified: [<list>]
  breaking_changes: [<list with justification>]

dependency_delta:
  added: [<crate versions>]
  removed: [<crate versions>]
  updated: [<crate: old → new>]
```

### Questions Answered
- Did module boundaries get clearer or blurrier?
- Are there new coupling patterns?
- Is the public API stable?

## 3. Verification Depth Auditor

Assesses correctness evidence.

### Inputs
- Test files in diff
- Test output/receipts
- Mutation testing results (if available)

### Outputs

```yaml
test_inventory:
  added: <count>
  modified: <count>
  removed: <count>

test_depth:
  behavior_tests: <count>  # Tests that verify behavior
  shape_tests: <count>     # Tests that verify structure only
  error_path_coverage: <yes|partial|no>
  property_tests: <count>

mutation_survival:
  score: <percentage>
  surviving_mutants: [<list with locations>]

invariants_added:
  - invariant: <description>
    enforced_by: <test or gate>

regression_coverage:
  bugs_found: <count>
  bugs_with_regression_test: <count>
```

### Questions Answered
- Are tests shallow or deep?
- Would mutations be caught?
- Are error paths exercised?

## 4. Docs Correctness Auditor

Assesses reproducibility and documentation.

### Inputs
- Doc files in diff
- README changes
- Gate commands

### Outputs

```yaml
gate_clarity:
  single_command: <yes|no>
  command: <the command>
  documented_in: <file path>

executable_snippets:
  total: <count>
  verified: <count>
  broken: [<list with locations>]

drift_risk:
  - doc: <file>
    code: <file>
    risk: <description>

receipt_availability:
  ci_runs: [<links>]
  local_output: <yes|no>

known_limits:
  declared: <yes|no>
  location: <file path>
```

### Questions Answered
- Can someone verify with one command?
- Are code snippets executable?
- Are limits explicit?

## 5. Measurement Integrity Auditor

Assesses metric stability.

### Inputs
- Benchmark changes
- Status files (`CURRENT_STATUS.md`)
- Baseline definitions

### Outputs

```yaml
baseline_changes:
  - metric: <name>
    before: <value>
    after: <value>
    justified: <yes|no>
    justification: <if provided>

semantic_drift:
  - metric: <name>
    risk: <description>

measurement_contracts:
  added: [<list>]
  removed: [<list>]

reproducibility:
  pinned_deps: <yes|no>
  deterministic: <yes|no|partial>
```

### Questions Answered
- Did baselines change without justification?
- Are measurements reproducible?
- Is there semantic drift?

## 6. Policy/Governance Auditor

Assesses alignment with project policies.

### Inputs
- Schema files (`features.toml`, `STABILITY.md`)
- Gate changes
- Commit messages

### Outputs

```yaml
schema_alignment:
  features_toml: <aligned|diverged|not_applicable>
  stability_md: <aligned|diverged|not_applicable>
  divergences: [<list>]

anti_drift_mechanisms:
  added: [<list>]
  existing_used: [<list>]
  missing: [<list>]

receipt_linkage:
  claims: <count>
  claims_with_evidence: <count>
  unlinked: [<list>]

commit_hygiene:
  conventional: <yes|partial|no>
  atomic: <yes|partial|no>
  issues: [<list>]
```

### Questions Answered
- Does the PR align with schemas?
- What anti-drift mechanisms exist?
- Are all claims backed?

## Running Analyzers

### Manual Invocation

Each analyzer can be run independently:

```bash
# Conceptual - actual implementation varies
analyze-pr --analyzer=diff-scout --pr=123
analyze-pr --analyzer=verification-depth --pr=123
```

### Full Analysis

Run all analyzers and synthesize:

```bash
analyze-pr --full --pr=123 --output=dossier
```

### Output Locations

- Individual: `docs/forensics/pr-NNN/analyzer-name.yaml`
- Synthesis: `docs/forensics/pr-NNN.md`
- Cover sheet: Extracted to PR body

## Synthesis Rules

When combining analyzer outputs into a cover sheet:

1. **Verdicts**: If any analyzer returns `fail`, overall is `fail`
2. **Key findings**: Pull P1/P2 findings, limit to 5
3. **Quality deltas**: Derive from analyzer outputs
4. **Budget**: Pull from basis of decision/friction events
5. **Recommendations**: Prioritize by severity

## Trust Model

- Analyzer findings are tagged with confidence
- Synthesis is explicit about assumptions
- Nothing is published as "fact" without evidence anchor
- Reanalysis is cheap and diffable

## Extension

To add a new analyzer:

1. Define inputs, outputs, and questions answered
2. Add to the analyzer table above
3. Implement the analysis logic
4. Add synthesis rules for cover sheet integration

## See Also

- [`QUALITY_SURFACES.md`](QUALITY_SURFACES.md) - The four quality surfaces
- [`METRICS_PROVENANCE.md`](METRICS_PROVENANCE.md) - Provenance schema
- [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md) - Full dossier template
