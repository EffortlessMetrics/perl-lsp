# PR Forensics Schema

This document defines the schema for PR archaeology dossiers. Use this when mining merged PRs to extract lessons and evidence.

## Purpose

PR archaeology extracts actionable patterns from past work:

- What went wrong (and was caught vs. slipped through)
- What the combined budget actually was (DevLT + compute)
- Which guardrails improved because of the PR
- How claims drifted from implementation

## Dossier Schema

Each PR dossier should capture:

### 1. Intent

| Field | Description |
| ----- | ----------- |
| PR number | GitHub PR reference |
| Issue/AC | Linked issue(s) and acceptance criteria |
| Stated goal | What the PR description claimed it would do |
| Actual scope | What files/modules it actually touched |

### 2. Scope Map

| Directory | Files Changed | Lines Delta | Notes |
| --------- | ------------- | ----------- | ----- |
| `crates/perl-parser/src/` | N | +X/-Y | (e.g., "parser core changes") |
| `crates/perl-lsp/src/` | N | +X/-Y | |
| `tests/` | N | +X/-Y | |
| `docs/` | N | +X/-Y | |

### 3. Evidence Pointers

Links to verification artifacts:

- CI run URL (if available)
- Gate output excerpt
- Test results (pass/fail counts)
- Benchmark delta (if applicable)

### 4. Findings

Categorized discoveries:

| Category | Finding | Severity | Evidence |
| -------- | ------- | -------- | -------- |
| Claim drift | PR said X, code does Y | P1/P2/P3 | Link/line |
| Stub left behind | Function declared but not implemented | P1/P2/P3 | File:line |
| Perf regression | Metric worsened | P1/P2/P3 | Benchmark |
| Test gap | Claimed tested, no test exists | P1/P2/P3 | Search |
| Scope creep | Changed files outside stated scope | P2/P3 | Diff |
| Dead code | Added code never exercised | P3 | Coverage |

Severity levels:

- **P1**: Blocks correctness or stability
- **P2**: Causes confusion or maintenance burden
- **P3**: Cleanup opportunity

### 5. Budget Estimates

| Metric | Value | Notes |
| ------ | ----- | ----- |
| DevLT (human attention) | X min | Band: <30 / 30-120 / 120+ |
| Compute (tokens/CI) | Y | Band: cheap / moderate / expensive |
| Efficiency | quality / budget | Subjective assessment |

Bands for DevLT:

- **<30 min**: Quick fix, no exploration needed
- **30-120 min**: Standard feature, some investigation
- **120+ min**: Complex, multi-session work

Bands for compute:

- **Cheap**: <10K tokens, <5 CI minutes
- **Moderate**: 10-100K tokens, 5-30 CI minutes
- **Expensive**: >100K tokens, >30 CI minutes

### 6. Factory Delta

What systemic improvement resulted from this PR:

| Guardrail | Before | After | Notes |
| --------- | ------ | ----- | ----- |
| (e.g., status-check) | Did not exist | Enforced in ci-gate | |
| (e.g., features.toml) | Manual tracking | Computed catalog | |

### 7. Exhibit Score

Overall quality assessment:

| Dimension | Score (1-5) | Notes |
| --------- | ----------- | ----- |
| Clarity of intent | | Was the goal clear? |
| Scope discipline | | Did it stay in scope? |
| Evidence quality | | Were claims backed? |
| Test coverage | | Did tests match claims? |
| DevLT efficiency | | Human time well spent? |

## Example Dossier

```markdown
## PR #153: Mutation Testing + UTF-16 Security Fixes

### Intent
- Issue: #148, #150
- Stated goal: Add mutation testing, fix UTF-16 boundary vulnerabilities
- Actual scope: perl-parser, perl-lsp, test infrastructure

### Scope Map
| Directory | Files | Delta |
| --------- | ----- | ----- |
| crates/perl-parser/src/ | 8 | +450/-120 |
| crates/perl-lsp/src/ | 3 | +80/-20 |
| tests/ | 12 | +600/-50 |

### Evidence
- CI: [run #xyz](link)
- Mutation score: 87% (target: 87%)
- UTF-16 symmetric conversion: verified

### Findings
| Category | Finding | Severity |
| -------- | ------- | -------- |
| None | Clean PR | - |

### Budget
- DevLT: 90 min (30-120 band)
- Compute: moderate (50K tokens, 15 CI min)

### Factory Delta
- Added: cargo-mutants to CI
- Added: UTF-16 boundary test suite

### Exhibit Score
- Clarity: 5 - Well-documented intent
- Scope: 4 - Minor scope expansion to related security
- Evidence: 5 - Mutation score receipts
- Tests: 5 - Comprehensive coverage
- Efficiency: 4 - Reasonable for scope
```

## How to Use

1. Pick a merged PR to analyze
2. Create `docs/forensics/pr-NNN.md`
3. Fill in each section of the schema
4. Add findings to `docs/LESSONS.md` if systemic
5. Update guardrails if a gap is found

## See Also

- [`LESSONS.md`](LESSONS.md) - Aggregated wrongness log
- [`AGENTIC_DEV.md`](AGENTIC_DEV.md) - Budget definitions and workflow
