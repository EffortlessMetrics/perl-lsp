# Issue Work Order Format

Standard contract block for converting agent research into mergeable PRs.

## Purpose

When agents investigate issues, they produce analysis. To make that analysis *actionable*, every issue update must conclude with a contract block that:

1. Is convertible into a PR scope
2. Has testable exit criteria
3. Quantifies quality impact
4. Anchors budget to decisions
5. Identifies prevention actions (if wrongness was discovered)

## Required Headings

Every issue work order comment must end with these 7 sections:

### 1. Current State (facts + anchors)

**What exists today:**
- Paths touched / relevant modules
- What already exists (tests, gates, docs)
- Any known constraints or blockers

```markdown
### Current State

- **Relevant paths**: `crates/perl-parser/src/lsp/`, `justfile`
- **Existing infrastructure**: `just ci-gate` runs format+clippy+tests
- **Known constraints**: CI minutes budget ~15m/run
```

### 2. Problem Statement (why this exists)

**One paragraph, no adjectives.** State the problem, not the solution.

```markdown
### Problem Statement

Performance regression alerts fire after-the-fact when users report slowness.
There is no automated detection of parse speed regressions between PRs, which
means perf degradations ship silently until noticed in production.
```

### 3. Plan (what we will do)

**3-10 ordered steps.** Explicitly call out what will NOT be done in this issue.

```markdown
### Plan

1. Add `cargo bench` baseline capture to CI gate
2. Store baseline in `.perf-baseline.json` (format TBD)
3. Add comparison step that fails if >10% regression
4. Document measurement contract in `docs/BENCHMARKS.md`
5. Add `just perf-check` recipe for local validation

**Out of scope:**
- Micro-benchmark harness selection (use cargo bench defaults)
- Historical trend visualization (future issue)
```

### 4. Exit Criteria (proof, not vibes)

**Always include `nix develop -c just ci-gate` plus 1-3 targeted commands.**

```markdown
### Exit Criteria

- [ ] `nix develop -c just ci-gate` passes
- [ ] `cargo bench` runs without error
- [ ] Regression detection triggers on synthetic 20% slowdown
- [ ] `docs/BENCHMARKS.md` documents measurement contract
```

### 5. Quality Deltas (primary output)

**Quantify impact on the four quality surfaces.** Range: +2 to -2.

| Surface | Delta | Rationale |
|---------|-------|-----------|
| Maintainability | 0 | No architectural changes |
| Correctness | +1 | Prevents shipping regressions |
| Governance | +1 | Adds measurable gate |
| Reproducibility | +1 | Baseline enables reproduction |

### 6. Budget (secondary, never "unknown")

**Anchored to decision events and friction.** Use [DEVLT_ESTIMATION.md](DEVLT_ESTIMATION.md) rubric.

```markdown
### Budget

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 45-90m | estimated; github_plus_agent_logs; 3 decisions (scope/verification/acceptance), 1 friction (baseline format) |
| CI | ~5m | estimated; cargo bench ~2m + existing gate ~3m |
| LLM | ~4 units | estimated; 2 iterations implementation + 2 refinement |
```

**Budget fields:**
- **DevLT**: `<low>-<high>m` (`confidence`; `coverage`; `basis`)
- **CI**: `<minutes>m` (`measured` or `estimated`; `source`)
- **LLM**: `~<units>` (`estimated`; `basis`)

### 7. Prevention Actions (only if needed)

**If friction was observed, at least one guardrail/gate/receipt change.**

```markdown
### Prevention Actions

**Friction observed**: Performance multiplier claims without absolute numbers led to confusion in PR #178.

**Prevention**:
1. Add to `measurement-auditor.md`: "Multiplier claims (Nx faster) require absolute numbers and methodology"
2. Add CI check: perf claims in PR body must include baseline SHA
```

If no friction was observed, omit this section or write:

```markdown
### Prevention Actions

None required. Clean investigation path.
```

## Measurement Contract (for tooling issues)

Issues that introduce new measurement tools (#276-#285 cluster) must include:

```markdown
### Measurement Contract

| Field | Value |
|-------|-------|
| **Tier** | `always-on` / `exhibit-grade` / `research` |
| **What is measured** | Parse time per-file, aggregate across corpus |
| **Units** | milliseconds (absolute), percentage (delta) |
| **Baseline** | SHA of comparison commit |
| **Dataset** | `test_corpus/` (100 files, 50KLOC) |
| **Flags** | `--release`, `RUST_TEST_THREADS=1` |
| **Comparability rule** | Same dataset + same flags = comparable |
| **Not-comparable behavior** | Block publication; require re-baseline |
```

### Tier Definitions

| Tier | When to run | Stability requirement |
|------|-------------|----------------------|
| **always-on** | Every CI run | Contract frozen; breaking change = migration |
| **exhibit-grade** | On-demand for forensics | Contract documented; changes require re-analysis |
| **research** | Manual invocation | No contract; results labeled "exploratory" |

## Template

Copy this for new issue work orders:

```markdown
## Work Order (YYYY-MM-DD)

### Current State

- **Relevant paths**:
- **Existing infrastructure**:
- **Known constraints**:

### Problem Statement

[One paragraph, no adjectives]

### Plan

1.
2.
3.

**Out of scope:**
-

### Exit Criteria

- [ ] `nix develop -c just ci-gate` passes
- [ ]
- [ ]

### Quality Deltas

| Surface | Delta | Rationale |
|---------|-------|-----------|
| Maintainability | 0 | |
| Correctness | 0 | |
| Governance | 0 | |
| Reproducibility | 0 | |

### Budget

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | -m | estimated; ; |
| CI | ~m | estimated; |
| LLM | ~ units | estimated; |

### Prevention Actions

[If friction observed, describe prevention. Otherwise: "None required."]
```

## Integration with Forensics Pipeline

```
Issue Created
    │
    ▼
Agent Research (analysis, exploration)
    │
    ▼
Work Order Comment (this format) ◄── Contract for PR scope
    │
    ▼
PR Created (work order becomes PR description basis)
    │
    ▼
PR Merged
    │
    ▼
Forensics Dossier (docs/forensics/pr-NNN.md) ◄── Post-merge analysis
    │
    ▼
Calibration Update (devlt.csv)
```

## See Also

- [DEVLT_ESTIMATION.md](DEVLT_ESTIMATION.md) - Budget estimation rubric
- [FORENSICS_SCHEMA.md](FORENSICS_SCHEMA.md) - PR dossier format (post-merge)
- [forensics/prompts/measurement-auditor.md](forensics/prompts/measurement-auditor.md) - Measurement integrity validation
- [forensics/INDEX.md](forensics/INDEX.md) - PR archaeology index
