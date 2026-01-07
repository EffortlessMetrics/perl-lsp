# Casebook

Exhibit PRs that demonstrate the development model and key capabilities.

## How to Read This

Each exhibit shows:
- **What it proves** (1 line)
- **Review map** (key files/surfaces touched)
- **Proof bundle** (receipts: test output, gate output, benchmarks)
- **What went wrong → fix → prevention** (if applicable)
- **DevLT band** (0-10m / 10-30m / 30m+)
- **Compute band** (unknown / low / med / high)

## Exhibits

### Exhibit 1: [Placeholder - Truth Pipeline Setup]

**What it proves:** TBD - Claims become mechanically bound to catalogs

**Review map:**
- `scripts/update-current-status.py`
- `features.toml`
- `docs/CURRENT_STATUS.md`

**Proof bundle:** `just status-check` fails if drift

**Scar story:** N/A - placeholder

**DevLT:** TBD | **Compute:** TBD

---

### Exhibit 2: [Placeholder - Semantic Analyzer Phase 1]

**What it proves:** TBD - Major capability addition with receipts

**Review map:**
- `crates/perl-parser/src/semantic/`
- LSP definition handler

**Proof bundle:** `just ci-lsp-def` (4/4 tests)

**Scar story:** N/A - placeholder

**DevLT:** TBD | **Compute:** TBD

---

### Exhibit 3: [Placeholder - Measurement Drift Fix]

**What it proves:** TBD - When claims drifted and how it was caught

**Review map:** TBD

**Proof bundle:** TBD

**Scar story:** N/A - placeholder

**DevLT:** TBD | **Compute:** TBD

---

## Adding Exhibits

To add an exhibit:
1. Use Level 2 archaeology from `docs/FORENSICS_SCHEMA.md`
2. Identify the "what it proves" in one line
3. Document the review map (key files)
4. Link to receipts (test output, gate output, benchmarks)
5. Record any wrongness discovered → fix → prevention
6. Estimate DevLT and compute bands

See [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md) for the full dossier template.
See [`forensics/INDEX.md`](forensics/INDEX.md) for the PR inventory.
