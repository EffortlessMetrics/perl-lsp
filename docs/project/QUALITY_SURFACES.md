# Quality Surfaces

Quality in AI-native development isn't a feeling. It's a set of deltas across falsifiable surfaces.

## The Four Surfaces

Every PR is evaluated against four quality surfaces:

| Surface | Question | Key Metrics |
|---------|----------|-------------|
| **Maintainability** | Is the code easier to work with? | Boundaries, coupling, complexity |
| **Correctness** | Does it do what it claims? | Tests, error handling, regressions |
| **Governance** | Does it follow project rules? | Schema alignment, anti-drift, receipts |
| **Reproducibility** | Can someone else verify it? | Gate commands, receipts, limits |

## A. Maintainability Surface

Measures how the PR affects long-term code health.

### Dimensions

| Dimension | Positive Delta | Negative Delta |
|-----------|---------------|----------------|
| **Boundary clarity** | Module responsibilities clearer | Responsibilities blurred |
| **Coupling** | Dependencies reduced/explicit | Hidden cross-module calls |
| **Complexity** | Large files split, logic simplified | Big files bigger, nested logic |
| **Public surface** | API stable or explicitly versioned | Breaking changes undocumented |
| **Debt tracking** | Known issues linked/filed | Hidden debt introduced |

### Assessment Questions

- Did module boundaries get clearer or blurrier?
- Are new dependencies explicit and justified?
- Did complexity concentrate or distribute?
- Is the public API stable or explicitly changed?
- Is any debt introduced explicitly tracked?

### Evidence Types

- File/module change maps
- Dependency diffs (`cargo tree` delta)
- Complexity metrics (cyclomatic, nesting depth)
- API diff (public functions/types changed)

## B. Correctness Surface

Measures whether the code does what it claims.

### Dimensions

| Dimension | Positive Delta | Negative Delta |
|-----------|---------------|----------------|
| **Behavior tests** | Tests verify behavior, not shape | Tests are shallow/tautological |
| **Error handling** | Error paths covered | Happy path only |
| **Mutation survival** | Mutants killed | Mutants survive |
| **Regression coverage** | Bug fixes have regression tests | Fixes without tests |
| **Property tests** | Invariants checked generatively | Manual cases only |

### Assessment Questions

- Do tests verify actual behavior or just structure?
- Are error paths exercised?
- Would mutations in this code be caught?
- Do bug fixes include regression tests?
- Are there property/fuzz tests where appropriate?

### Evidence Types

- Test count delta by type (unit, integration, property)
- Mutation testing score
- Coverage delta (if available)
- Regression test commits

## C. Governance Surface

Measures alignment with project policies and anti-drift mechanisms.

### Dimensions

| Dimension | Positive Delta | Negative Delta |
|-----------|---------------|----------------|
| **Schema alignment** | Matches `features.toml`, docs | Diverges from schema |
| **Anti-drift** | Adds status-check, snapshots | No anti-drift mechanism |
| **Wrongness recording** | Scars documented | Failures unrecorded |
| **Receipt linkage** | Evidence linked | Claims without proof |
| **Commit hygiene** | Conventional commits, atomic | Vague messages, mixed scope |

### Assessment Questions

- Does the PR align with capability schemas?
- What anti-drift mechanisms were added or used?
- Is any wrongness discovered documented?
- Are all claims backed by linked evidence?
- Are commits atomic and well-messaged?

### Evidence Types

- Schema diffs (`features.toml`, `STABILITY.md`)
- Gate additions (new checks added)
- Scar stories in dossier
- Receipt links in PR body

## D. Reproducibility Surface

Measures whether a third party can verify the work.

### Dimensions

| Dimension | Positive Delta | Negative Delta |
|-----------|---------------|----------------|
| **Gate clarity** | One command to verify | Multi-step, undocumented |
| **Receipt availability** | Outputs linked/committed | "Trust me, it worked" |
| **Limit declaration** | Known limits explicit | Hidden assumptions |
| **Environment stability** | Pinned deps, Nix flake | "Works on my machine" |

### Assessment Questions

- Can someone verify this with a single command?
- Are gate outputs available as receipts?
- Are known limits and caveats explicit?
- Is the build environment reproducible?

### Evidence Types

- Gate command in cover sheet
- Linked CI runs or local output
- `known_limits` section in cover sheet
- Lockfile stability

## Cover Sheet Integration

Every PR cover sheet includes a **Quality Deltas** section:

```markdown
### Quality Deltas

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +1 | Split `large_module.rs` into 3 focused modules |
| Correctness | +2 | Added 15 behavior tests, killed 4 mutants |
| Governance | 0 | No schema changes; existing gates pass |
| Reproducibility | +1 | Added `known_limits` section |
```

Delta scale:
- **+2**: Significant improvement
- **+1**: Minor improvement
- **0**: No change
- **-1**: Minor regression (justified)
- **-2**: Significant regression (requires justification)

## Analyzer Integration

Each surface maps to a specialist analyzer:

| Surface | Analyzer | Key Outputs |
|---------|----------|-------------|
| Maintainability | Design/Contract Auditor | Boundary map, coupling delta |
| Correctness | Verification Depth Auditor | Test depth score, mutation survival |
| Governance | Policy Auditor | Schema alignment, anti-drift inventory |
| Reproducibility | Docs Correctness Auditor | Gate clarity score, receipt inventory |

See [`ANALYZER_FRAMEWORK.md`](ANALYZER_FRAMEWORK.md) for analyzer specifications.

## Quality vs. Efficiency

Quality is the primary output. Efficiency is a tuning signal.

```
Quality comes first.
Efficiency tells you how much it cost to achieve that quality.
```

A PR with excellent quality and high DevLT is preferable to a PR with poor quality and low DevLT.

## See Also

- [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md) - Budget estimation
- [`METRICS_PROVENANCE.md`](METRICS_PROVENANCE.md) - Provenance schema
- [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md) - Full dossier template
- [`ANALYZER_FRAMEWORK.md`](ANALYZER_FRAMEWORK.md) - Specialist analyzers
