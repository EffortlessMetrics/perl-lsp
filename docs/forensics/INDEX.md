# PR Archaeology Index

Inventory of PRs analyzed for the casebook and lessons ledger.

## Triage Levels

| Level | Depth | Output | When to use |
|-------|-------|--------|-------------|
| 0 | Inventory | One row here | All PRs |
| 1 | Dossier | `pr-NNN.md` file | Interesting or risky PRs |
| 2 | Exhibit | Entry in `CASEBOOK.md` | Best PRs for external legibility |

## Inventory

| PR # | Title | Type | Surfaces | Evidence? | Wrongness? | Level | DevLT | Compute |
|------|-------|------|----------|-----------|------------|-------|-------|---------|
| 181/259 | Call Hierarchy and Name Span Improvements | feature | parser/lsp | Y (11 tests) | None | 1 | 60-90 | moderate |
| 188 | Semantic Analyzer Phase 1 | feature | parser/lsp/docs | Y (receipts) | None | 1 | 60-90 | moderate |
| 225/226/229 | Statement Tracker + Heredoc Block-Aware Integration | feature | parser/lexer | Y (F1-F6 fixtures) | None | 1 | 60-90 | moderate |
| 260 | Substitution Operator Correctness (MUT_002, MUT_005) | hardening | parser/lexer | Y (mutant IDs) | Y (mutation bugs) | 1 | 60-90 | moderate |
| 264 | Mixed-Delimiter Substitution Replacement | hardening | lexer/tests | Y (regression) | Y (follow-up) | 1 | 30-60 | cheap |
| 251/252/253 | Test Harness Hardening + BrokenPipe Sweep | mechanization | lsp/tests | Y (baseline) | Y (protocol) | 1 | 90-120 | moderate |

**Type codes:** feature | hardening | mechanization | docs | perf | refactor

**Surfaces:** parser | lexer | lsp | dap | ci | docs | catalog | bench

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

Add an entry to [`../CASEBOOK.md`](../CASEBOOK.md).

## See Also

- [`FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md) - Dossier template
- [`CASEBOOK.md`](../CASEBOOK.md) - Exhibit entries
- [`LESSONS.md`](../LESSONS.md) - Wrongness log
- [`README.md`](README.md) - This directory's purpose
