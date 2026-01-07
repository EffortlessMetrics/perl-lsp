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
| TBD | Placeholder | feature | parser/lsp | - | - | 0 | - | - |

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
