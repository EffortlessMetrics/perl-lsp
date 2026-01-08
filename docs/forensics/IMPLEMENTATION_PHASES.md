# Implementation Phases

Sequencing for the 20-issue swarm to reduce entropy and prevent duplicate tooling.

## Guiding Principle

**Quality-first, not velocity-first.** Each phase stabilizes a foundation before the next builds on it.

## Phase A — Trust Surface Stabilization

**Goal**: One authoritative merge gate posture, minimal contradictions, predictable receipts.

| Issue | Title | Why First |
|-------|-------|-----------|
| #211 | CI pipeline cleanup | Prerequisite for all CI-based tooling |
| #210 | Merge gates formalization | Defines what "passing" means for everything else |

**Exit criteria**:
- [ ] Single source of truth for gate configuration
- [ ] All gate outputs have receipt format
- [ ] Local-first development path documented

**Dependencies**: None (root phase)

---

## Phase B — Test Reliability and Correctness Hardening

**Goal**: Reduce churn and regressions by stabilizing test infrastructure.

| Issue | Title | Why Here |
|-------|-------|----------|
| #198 | Ignored tests | Reduces ignored count before measuring |
| #201 | Mutation tests failing | Establishes mutation baseline |
| #255 | Incremental parsing metrics | Defines parsing correctness semantics |
| #204 | Eliminate unreachable!() | Removes panic surface |
| #256 | TODO cleanup | Only after tests stable to avoid churn |

**Exit criteria**:
- [ ] Ignored test count ≤ N (define threshold)
- [ ] Mutation score ≥ M% for parser module
- [ ] Zero unreachable!() in production code
- [ ] TODO count tracked in `.todo-baseline`

**Dependencies**: Phase A (gates must be defined before hardening)

---

## Phase C — Measurement/Tooling Contracts

**Goal**: Single semantics layer for all measurement tools. Shared receipt format.

| Issue | Title | Measurement Contract |
|-------|-------|---------------------|
| #276 | Coverage | Test coverage % with defined scope |
| #277 | Semver checks | Breaking change detection |
| #278 | Perf regression alerts | Benchmark comparison |
| #280 | Changelog generation | Release note automation |
| #279 | Dependabot/Renovate | Dependency update flow |
| #284 | Dead code detection | Unused code reporting |

**Shared Infrastructure**:
- Common receipt format (YAML with provenance)
- Common tier system (always-on | exhibit-grade | research)
- Common threshold configuration in `tooling.toml` or similar
- Common PR comment format

**Exit criteria**:
- [ ] Each tool has documented measurement contract
- [ ] All tools share receipt format
- [ ] Tier classification documented for each tool
- [ ] `just quality-report` aggregates all tool outputs

**Dependencies**: Phase B (test baseline must be stable before measuring coverage)

---

## Phase D — Supply Chain + Security

**Goal**: Security tooling starts as research tier, promotes when stable.

| Issue | Title | Initial Tier |
|-------|-------|-------------|
| #281 | SBOM/SLSA | research |
| #282 | Security scanning | research |
| #285 | cargo-fuzz | research |

**Promotion criteria** (research → exhibit-grade → always-on):
1. Contract documented and stable for 5 PRs
2. False positive rate < 10%
3. CI time impact acceptable (< 2m added)
4. Maintainer sign-off on promotion

**Exit criteria**:
- [ ] Each tool has measurement contract (even if research tier)
- [ ] Promotion criteria documented per tool
- [ ] At least one tool promoted to exhibit-grade

**Dependencies**: Phase C (measurement contract system must exist)

---

## Phase E — "Batteries Included" and Product Polish

**Goal**: UX improvements on stable foundation.

| Issue | Title | Notes |
|-------|-------|-------|
| #208 | Batteries included | First-run experience |
| #212 | Parser foundation | Core parser improvements |
| #213 | LSP polish | Editor integration quality |
| #283 | mdBook docs | Documentation site |

**Can start earlier** (non-blocking):
- #283 mdBook can start in Phase A but should not gate other work

**Exit criteria**:
- [ ] First-run tutorial exists
- [ ] LSP passes all protocol compliance tests
- [ ] Documentation builds and deploys

**Dependencies**: Phases A-D (trust surfaces and tests must be stable)

---

## Batching Strategy

### Same-PR Batches (ship together)

Group issues that share implementation:

| Batch | Issues | Rationale |
|-------|--------|-----------|
| Gate consolidation | #210, #211 | Both touch CI/gate config |
| Test hardening | #198, #201 | Both stabilize test baseline |
| Coverage tooling | #276, #284 | Both measure code coverage/usage |
| Security baseline | #281, #282 | Both are supply chain security |

### Sequential Dependencies

```
Phase A ──► Phase B ──► Phase C ──► Phase D
                │                    │
                └──────► Phase E ◄───┘
```

Phase E can start partial items during B/C, but UX polish should not merge until trust surfaces are stable.

---

## Shared Measurement Contract Work

All tooling issues (#276-#285) must share:

### 1. Receipt Format

```yaml
tool: <tool-name>
version: <tool-version>
timestamp: <ISO8601>
tier: <always-on|exhibit-grade|research>
contract_hash: <sha256 of measurement contract>
inputs:
  baseline: <SHA or file>
  flags: <list>
  dataset: <description>
outputs:
  value: <measured value>
  unit: <unit>
  threshold: <pass/warn/fail thresholds>
  verdict: <pass|warn|fail>
comparability: <comparable|not_comparable>
```

### 2. Tier Configuration

Location: `tooling.toml` (or section in existing config)

```toml
[tools.coverage]
tier = "exhibit-grade"
threshold_warn = 70
threshold_fail = 50
contract_version = "v1"

[tools.semver]
tier = "always-on"
# No threshold - binary pass/fail
contract_version = "v1"

[tools.perf]
tier = "exhibit-grade"
threshold_warn_pct = 10
threshold_fail_pct = 25
contract_version = "v1"
```

### 3. PR Comment Template

All tooling outputs should use consistent comment format:

```markdown
## Quality Report

| Tool | Result | Delta | Notes |
|------|--------|-------|-------|
| coverage | 78% | +2% | vs baseline abc123 |
| semver | ✓ | — | No breaking changes |
| perf | ⚠️ | +12% | Exceeds 10% threshold |

<details>
<summary>Full Reports</summary>

### Coverage
[raw output]

### SemVer
[raw output]

### Performance
[raw output]
</details>
```

---

## Top 3 Prevention Actions (Repo-Wide)

From swarm analysis, these prevent the most friction:

### 1. Measurement Contract Required for New Tools

**Rule**: No new measurement tool merges without documented contract in `docs/TOOLING_CONTRACTS.md`.

**Enforcement**: Add checklist item to PR template.

### 2. Multiplier Claims Require Absolute Numbers

**Rule**: Claims like "4x faster" must include absolute before/after values, baseline SHA, and methodology.

**Enforcement**: Add to `measurement-auditor.md` checklist.

### 3. Threshold Changes Require Calibration Data

**Rule**: Changing pass/warn/fail thresholds requires ≥5 data points showing new threshold is appropriate.

**Enforcement**: Add to tier promotion criteria.

---

## Roll-Up Issue

Create a meta-issue or pinned discussion containing:

1. **This phase sequencing** (link to this doc)
2. **Issue dependency graph** (mermaid or ASCII)
3. **Shared measurement contract PRs** (track which issues contribute)
4. **Current phase status** (updated weekly)

Template:

```markdown
# Tooling Swarm Roll-Up

## Current Phase: A (Trust Surface)

### Phase A Status
- [ ] #210 - in progress
- [ ] #211 - ready for review

### Blocked Items
- #276 coverage - blocked on #210 (gate definition)

### Shared Infrastructure PRs
- PR #xxx - receipt format schema
- PR #xxx - tooling.toml configuration

### Next Phase Gate
Phase A → B requires:
- All Phase A issues merged
- Gate configuration documented
- Receipt format tested in 2+ tools
```

---

## See Also

- [WORK_ORDER_FORMAT.md](WORK_ORDER_FORMAT.md) - Per-issue contract format
- [DEVLT_ESTIMATION.md](../DEVLT_ESTIMATION.md) - Budget estimation
- [FORENSICS_SCHEMA.md](../FORENSICS_SCHEMA.md) - PR dossier format
