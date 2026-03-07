# Lessons Learned (Factory Log)

This file records what we got wrong and what changed because of it.
Each entry follows the format: **wrong → evidence → fix → prevention**.

---

## Wrongness Categories

- **Claim drift**: docs/PR say X, catalog/tests say Y
- **Measurement drift**: denominator/units/input corpus changed; ratios became meaningless
- **Harness drift**: tooling behavior changed (timeouts, concurrency, env, OS paths)
- **Scope drift**: PR widened beyond ACs/issue intent
- **Non-determinism / flake**: intermittent failures, timing dependence
- **Coverage illusion**: "we have corpus coverage" but NodeKind/feature never exercised
- **Packaging drift**: install path, bin names, feature flags differ from docs

---

## Entries

### 2026-01-07 — Claim drift: LSP coverage overstated

**Type:** Claim drift

**Where it showed up:** ROADMAP.md, CLAUDE.md (claimed "~91% functional")

**Ground truth:** `features.toml` + `update-current-status.py` formula (82% advertised GA / trackable)

**Impact:** Credibility + planning error (wrong priority assumptions)

**Fix:**
- Created `update-current-status.py` to compute metrics from sources
- Added `just status-check` to `ci-gate` so drift fails locally
- Rewrote ROADMAP.md and CLAUDE.md to link to computed sources

**Prevention:**
- All numeric claims must be sourced from `CURRENT_STATUS.md` or receipts
- `just status-check` runs in `ci-gate`; fails if docs don't match computed values

**DevLT:** ~60 min (doc audit + script creation)
**Machine:** N/A

**Links:** This PR, `scripts/update-current-status.py`

---

### 2026-01-07 — Claim drift: Performance claims unverified

**Type:** Claim drift

**Where it showed up:** CLAUDE.md, ROADMAP.md (claimed "4-19x faster", "5000x performance improvements")

**Ground truth:** Benchmark framework exists but results are not published under `benchmarks/results/`

**Impact:** Claims cannot be verified by third parties

**Fix:**
- Removed all multiplier claims from ROADMAP.md
- Added note: "Framework exists; results are not yet published as canonical numbers"
- CLAUDE.md now links to CURRENT_STATUS for metrics instead of stating them

**Prevention:**
- No performance claims until benchmark results are committed to `benchmarks/results/`
- ROADMAP includes specific publication steps

**DevLT:** ~20 min
**Machine:** N/A

**Links:** This PR

---

### 2026-01-07 — Claim drift: Superlatives in documentation

**Type:** Claim drift

**Where it showed up:** CLAUDE.md, ROADMAP.md ("Revolutionary", "Enterprise-grade", "Game-changing")

**Ground truth:** Marketing language, not backed by receipts

**Impact:** Undermines credibility; confuses aspirational with factual

**Fix:**
- Rewrote CLAUDE.md as operator manual (no superlatives)
- Rewrote ROADMAP.md to focus on deliverables and exit criteria

**Prevention:**
- Rule: No adjectives without receipts
- Review checklist: "Would a skeptic accept this claim?"

**DevLT:** ~40 min
**Machine:** N/A

**Links:** This PR

---

### 2026-01-07 — Claim drift: Issue IDs treated as PR IDs

**Type:** Claim drift

**Where it showed up:** Dossiers and CASEBOOK.md referenced "#188" and "#181" as if they were PRs

**Ground truth:** `gh issue view 188` shows issue; `gh pr view 188` returns "no PR". PRs are #231/#232/#234 (for issue #188) and #259 (for issue #181)

**Impact:** Confusion in documentation; incorrect file names initially considered; readers could not verify claims

**Fix:**
- Verified all number references with `gh issue view` and `gh pr view`
- Updated forensics/INDEX.md to have separate `Issue` and `PR(s)` columns
- Renamed dossiers to use actual PR numbers (e.g., `pr-231-232-234.md` not `pr-188.md`)

**Prevention:**
- Schema rule: always store Issue and PR(s) as separate columns
- Verification command in workflow: `gh pr view $n` before creating dossier
- INDEX.md template enforces Issue/PR(s) column structure

**DevLT:** ~20 min
**Machine:** N/A

**Links:** forensics/INDEX.md, CASEBOOK.md

---

## How to Add Entries

When something is discovered to be wrong:

1. Identify the wrongness category
2. Document where it showed up (file + line if possible)
3. State the ground truth (what was actually correct)
4. Describe the impact (what went wrong because of it)
5. Record the fix (what changed in the codebase)
6. Define prevention (what systemic change prevents recurrence)
7. Log DevLT (human minutes) and machine cost (if tracked)
8. Link to relevant PRs/commits

Keep entries short and falsifiable. The point is system improvement, not blame.

## See Also

- [`AGENTIC_DEV.md`](AGENTIC_DEV.md) - Development model and budget definitions
- [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md) - PR archaeology dossier template
- [`INDEX.md`](INDEX.md) - Documentation front door
