# Issue #278 Work Order: Performance Regression Alerts

**Generated**: 2026-01-08
**Original Issue**: [Tooling] Add Automated Performance Regression Alerts

---

## Work Order

### Current State

- **Relevant paths**: `.github/workflows/benchmark.yml`, `.github/workflows/ci-expensive.yml`, `justfile`
- **Existing infrastructure**: Hyperfine and cargo bench execute benchmarks; no baseline storage or comparison
- **Known constraints**: CI minutes budget; workflow complexity; baseline storage strategy undecided

### Problem Statement

Performance regressions can merge undetected because benchmarks run but results are not compared against baselines with automated action. Manual review is error-prone and does not enforce performance SLOs.

### Plan

1. Select comparison tool: `critcmp` + criterion.rs JSON output (simpler than bencher.dev, no external dependency)
2. Add `cargo bench -- --save-baseline pr` to PR workflow
3. Add `.bench-baseline/master.json` baseline file (committed, updated on master merge)
4. Create `just bench-compare` recipe that runs critcmp against baseline
5. Add PR comment step posting comparison table (using `gh pr comment`)
6. Configure 10% regression threshold as warning; 25% as failure
7. Document measurement contract in `docs/BENCHMARKS.md`

**Out of scope:**
- Cloud-hosted solutions (bencher.dev, codspeed) - evaluate in future issue
- Historical trend visualization dashboard
- Micro-benchmark harness changes (use existing criterion.rs setup)
- Per-benchmark SLOs (start with aggregate threshold)

### Exit Criteria

- [ ] `nix develop -c just ci-gate` passes
- [ ] `just bench-compare` runs locally without error
- [ ] PR comment shows comparison table with deltas
- [ ] Synthetic 20% regression triggers warning annotation
- [ ] Synthetic 30% regression fails workflow
- [ ] `docs/BENCHMARKS.md` documents measurement contract
- [ ] Baseline file format documented

### Quality Deltas

| Surface | Delta | Rationale |
|---------|-------|-----------|
| Maintainability | 0 | No architectural changes; adds tooling |
| Correctness | +1 | Prevents shipping undetected regressions |
| Governance | +1 | Adds measurable gate with defined thresholds |
| Reproducibility | +1 | Baseline enables reproduction; commands documented |

### Budget

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 60-120m | estimated; github_plus_agent_logs; 4 decisions (tool selection, threshold values, baseline strategy, comment format) + 1-2 friction (threshold tuning) |
| CI | +3-5m | estimated; criterion run ~2m + comparison ~1m per PR |
| LLM | ~6 units | estimated; 3 implementation iterations + 2 threshold tuning + 1 docs |

### Measurement Contract

| Field | Value |
|-------|-------|
| **Tier** | `exhibit-grade` → promote to `always-on` after 10 stable PRs |
| **What is measured** | Parse time per benchmark function (criterion.rs) |
| **Units** | nanoseconds (absolute), percentage (delta vs baseline) |
| **Baseline** | `.bench-baseline/master.json` at HEAD of master |
| **Dataset** | criterion.rs benchmark suite (existing) |
| **Flags** | `--release`, `RUSTFLAGS=""`, warm-up iterations: default |
| **Comparability rule** | Same flags + same baseline commit = comparable |
| **Not-comparable behavior** | Post warning comment; do not fail; require re-baseline |

### Prevention Actions

**Friction observed (from prior perf discussions)**: Performance multiplier claims ("4x faster") without absolute numbers led to measurement integrity questions.

**Prevention**:
1. Add to `measurement-auditor.md` findings checklist: "Multiplier claims require: absolute before/after numbers, methodology, baseline SHA"
2. Add validation in PR comment template: include both absolute values and percentage delta
3. Document in `BENCHMARKS.md`: "Claims from benchmark comparisons must cite baseline SHA"

---

## Implementation Notes

### Tool Selection Rationale

| Option | Pros | Cons | Decision |
|--------|------|------|----------|
| bencher.dev | Cloud dashboard, trend graphs | External dependency, potential cost | Defer |
| criterion.rs + critcmp | No external deps, JSON output | No dashboard | **Selected** |
| hyperfine + custom | Flexible | More maintenance | Skip |
| codspeed | Rust-focused | External service | Defer |

### Baseline Strategy

```
.bench-baseline/
├── master.json          # Updated on master merge
└── README.md            # Format documentation
```

Update flow:
1. PR runs benchmark, compares against `master.json`
2. On merge to master, workflow updates `master.json`
3. Baseline JSON committed to repo (not artifact) for reproducibility

### Threshold Rationale

| Threshold | Action | Reasoning |
|-----------|--------|-----------|
| ≤10% slower | Pass | Normal variance |
| 10-25% slower | Warning annotation | Investigate, may be acceptable |
| >25% slower | Fail workflow | Unacceptable regression |

These thresholds are initial; calibrate after 10 PRs using actual variance data.

---

## Checklist for Closing

- [ ] PR implements plan steps 1-7
- [ ] Exit criteria all checked
- [ ] Measurement contract documented in `docs/BENCHMARKS.md`
- [ ] Update `forensics/INDEX.md` with PR reference
- [ ] Add calibration entry to `forensics/calibration/devlt.csv`
