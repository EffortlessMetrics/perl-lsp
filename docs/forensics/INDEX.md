# PR Archaeology Index

Inventory of PRs analyzed for the casebook and lessons ledger.

## Triage Levels

| Level | Depth | Output | When to use |
|-------|-------|--------|-------------|
| 0 | Inventory | One row here | All PRs |
| 1 | Dossier | `pr-NNN.md` file | Interesting or risky PRs |
| 2 | Exhibit | Entry in `CASEBOOK.md` | Best PRs for external legibility |

## Inventory

| Issue | PR(s) | Title | Type | Surfaces | Evidence? | Wrongness? | Level | Cover Sheet | Exhibit ID |
|-------|-------|-------|------|----------|-----------|------------|-------|-------------|------------|
| #181 | #259 | Name Span for LSP Navigation | feature | parser/lsp | Y (11 tests) | None | 2 | ✅ Pasted | `name-span` |
| #188 | #231/232/234 | Semantic Analyzer Phase 1 | feature | parser/lsp/docs | Y (receipts) | None | 2 | ✅ Pasted | `semantic-phase1` |
| #182 | #225/226/229 | Statement Tracker + Heredoc Block-Aware | feature | parser/lexer | Y (F1-F6 fixtures) | None | 2 | ✅ Pasted | `statement-tracker` |
| #144/147 | #260/264 | Substitution Operator Correctness | hardening | parser/lexer | Y (mutant IDs) | Y (mutation bugs) | 2 | ✅ Pasted | `mutation-subst` |
| — | #251/252/253 | Test Harness Hardening + BrokenPipe Sweep | mechanization | lsp/tests | Y (baseline) | Y (protocol) | 2 | ✅ Pasted | `harness-hardening` |
| #182c | #271 | Statement Tracker Block Depth v2 | feature | parser | Y | None | 0 | — | — |
| — | #261 | BUG=0 Burn-down | hardening | parser/lexer | Y | Y (multi-bug) | 1 | ⏳ Pending | — |
| — | #250 | Test Harness Hardening (29 unignored) | mechanization | lsp/tests | Y (baseline) | Y (protocol) | 0 | — | — |
| — | #236 | LSP Modularization + Version Guard | refactor | lsp/parser | Y | None | 1 | ⏳ Pending | — |
| #182b | #223 | Thread Statement Tracker | feature | parser | Y | None | 0 | — | — |
| #182a | #222 | Heredoc/Statement Tracker Data Structures | feature | parser | Y | None | 0 | — | — |
| — | #214 | Heredoc Day2 Lean CI | feature | ci/parser | Y | None | 0 | — | — |
| #147 | #158 | Complete Substitution Operator Parsing | feature | parser | Y | None | 0 | — | — |

**Type codes:** feature | hardening | mechanization | docs | perf | refactor

**Surfaces:** parser | lexer | lsp | dap | ci | docs | catalog | bench

**Cover Sheet status:** ✅ Pasted (applied to GitHub PR) | ✅ Ready (drafted, ready to paste) | ⏳ Pending | — (not needed)

### Dossier Files

| Exhibit ID | Dossier |
|------------|---------|
| `name-span` | [`pr-259.md`](pr-259.md) |
| `semantic-phase1` | [`pr-231-232-234.md`](pr-231-232-234.md) |
| `statement-tracker` | [`pr-225-226-229.md`](pr-225-226-229.md) |
| `mutation-subst` | [`pr-260-264.md`](pr-260-264.md) |
| `harness-hardening` | [`pr-251-252-253.md`](pr-251-252-253.md) |

## How to Use

### Level 0: Inventory (for all PRs)

Run a quick scan:
- Directory histogram (where did the diff land?)
- Commit prefix distribution (feat/fix/test/docs/chore)
- Check-run truth surface
- Receipts present? (Y/N)

Add one row to the table above.

### Level 1: Dossier (for interesting PRs)

Create `pr-NNN.md` in this directory using the template in [`../FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md).

### Level 2: Exhibit (for best PRs)

1. Create dossier (Level 1)
2. Add entry to [`../CASEBOOK.md`](../CASEBOOK.md)
3. Draft cover sheet using format below
4. Paste cover sheet into GitHub PR body with "Addendum (YYYY-MM-DD)" header

## Cover Sheet Format (Quality-First)

```markdown
## Cover sheet (added YYYY-MM-DD; original notes below)

- **Issue(s):** #NNN
- **PR:** #NNN
- **Exhibit ID:** `slug-name`

### What changed
### Why
### Review map
### Verification (receipts)
### Known limits / follow-ups
### How to reproduce trust

### Quality Deltas

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +1/0/-1 | boundary clarity, coupling |
| Correctness | +1/0/-1 | test depth, mutation survival |
| Governance | +1/0/-1 | schema alignment, anti-drift |
| Reproducibility | +1/0/-1 | gate clarity, limits |

### Budget (with Provenance)

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | X–Ym | estimated; coverage; confidence; basis |
| CI | Zm | measured/estimated; source |
| LLM | ~N units | estimated; basis |

---

### Forensics addendum (optional)
- Diff shape
- Complexity/risk notes
- Friction events (wrong → caught → fix → prevention)
```

### Directory Structure

```
docs/forensics/
├── INDEX.md                    # This file - PR inventory
├── README.md                   # Directory purpose and methodology
├── pr-*.md                     # Individual PR dossiers
├── prompts/                    # LLM analyzer prompt specifications
│   ├── README.md               # Analyzer framework overview
│   ├── diff-scout.md           # Scope surface analyzer
│   ├── design-auditor.md       # Maintainability analyzer
│   ├── verification-auditor.md # Correctness analyzer
│   ├── docs-auditor.md         # Reproducibility analyzer
│   ├── policy-auditor.md       # Governance analyzer
│   ├── measurement-auditor.md  # Measurement integrity (final gate)
│   ├── chronologist.md         # Temporal topology analyzer
│   └── decision-extractor.md   # DevLT estimation analyzer
└── calibration/                # DevLT calibration data
    ├── README.md               # Calibration methodology
    └── devlt.csv               # Per-PR calibration records
```

See methodology docs:
- [`../DEVLT_ESTIMATION.md`](../DEVLT_ESTIMATION.md) - DevLT estimation method
- [`../METRICS_PROVENANCE.md`](../METRICS_PROVENANCE.md) - Provenance schema
- [`../QUALITY_SURFACES.md`](../QUALITY_SURFACES.md) - The four quality surfaces
- [`../FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md) - Full dossier template

## See Also

- [`FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md) - Dossier template
- [`CASEBOOK.md`](../CASEBOOK.md) - Exhibit entries
- [`LESSONS.md`](../LESSONS.md) - Wrongness log
- [`README.md`](README.md) - This directory's purpose
