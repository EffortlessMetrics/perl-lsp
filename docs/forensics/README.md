# PR Forensics

This directory contains PR dossiers created during archaeology, plus coordination infrastructure for parallel agent investigations.

## Directory Structure

```
forensics/
├── README.md                   # This file
├── INDEX.md                    # PR archaeology inventory
├── WORK_ORDER_FORMAT.md        # Pre-PR work order template
├── IMPLEMENTATION_PHASES.md    # Swarm coordination phases
├── pr-*.md                     # Individual PR dossiers
├── prompts/                    # LLM analyzer specifications
│   ├── README.md               # Analyzer framework overview
│   ├── diff-scout.md           # Scope surface analyzer
│   ├── design-auditor.md       # Maintainability analyzer
│   ├── verification-auditor.md # Correctness analyzer
│   ├── docs-auditor.md         # Reproducibility analyzer
│   ├── policy-auditor.md       # Governance analyzer
│   ├── measurement-auditor.md  # Measurement integrity (final gate)
│   ├── chronologist.md         # Temporal topology analyzer
│   └── decision-extractor.md   # DevLT estimation analyzer
├── calibration/                # DevLT calibration data
│   ├── README.md               # Calibration methodology
│   └── devlt.csv               # Per-PR calibration records
└── examples/                   # Work order examples
    └── issue-278-work-order.md # Demonstration template
```

## Two Workflows

### Pre-PR: Issue Work Orders

When agents investigate issues, use [WORK_ORDER_FORMAT.md](WORK_ORDER_FORMAT.md) to:
- Convert analysis into actionable scope
- Define testable exit criteria
- Anchor budget to decision events
- Declare measurement contracts (for tooling issues)

### Post-PR: Forensics Dossiers

After PRs merge, create `pr-NNN.md` using [`../FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md) to:
- Extract lessons from merged PRs
- Measure actual DevLT vs estimated
- Identify guardrail improvements
- Track claim drift

## Coordination

For multi-issue swarms, see [IMPLEMENTATION_PHASES.md](IMPLEMENTATION_PHASES.md):
- Phase sequencing (A→E)
- Dependency ordering
- Shared tooling contracts
- Prevention actions

## See Also

- [`INDEX.md`](INDEX.md) - PR archaeology inventory
- [`../FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md) - PR dossier template
- [`../DEVLT_ESTIMATION.md`](../DEVLT_ESTIMATION.md) - Budget estimation rubric
- [`../LESSONS.md`](../LESSONS.md) - Aggregate findings
