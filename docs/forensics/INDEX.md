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
| #181 | #259 | Name Span for LSP Navigation | feature | parser/lsp | Y (11 tests) | None | 2 | ✅ Ready | `name-span` |
| #188 | #231/232/234 | Semantic Analyzer Phase 1 | feature | parser/lsp/docs | Y (receipts) | None | 2 | ✅ Ready | `semantic-phase1` |
| — | #225/226/229 | Statement Tracker + Heredoc Block-Aware | feature | parser/lexer | Y (F1-F6 fixtures) | None | 2 | ✅ Ready | `statement-tracker` |
| — | #260/264 | Substitution Operator Correctness | hardening | parser/lexer | Y (mutant IDs) | Y (mutation bugs) | 2 | ✅ Ready | `mutation-subst` |
| — | #251/252/253 | Test Harness Hardening + BrokenPipe Sweep | mechanization | lsp/tests | Y (baseline) | Y (protocol) | 2 | ✅ Ready | `harness-hardening` |

**Type codes:** feature | hardening | mechanization | docs | perf | refactor

**Surfaces:** parser | lexer | lsp | dap | ci | docs | catalog | bench

**Cover Sheet status:** ✅ Ready (drafted, paste into GitHub) | ⏳ Pending | — (not needed)

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

## Cover Sheet Format

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

---

### Forensics addendum (optional)
- Diff shape
- Complexity/risk notes
- Friction events (wrong → caught → fix → prevention)
- Budget (wall clock, DevLT band, compute band)
```

See [`../FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md) for full template.

## See Also

- [`FORENSICS_SCHEMA.md`](../FORENSICS_SCHEMA.md) - Dossier template
- [`CASEBOOK.md`](../CASEBOOK.md) - Exhibit entries
- [`LESSONS.md`](../LESSONS.md) - Wrongness log
- [`README.md`](README.md) - This directory's purpose
