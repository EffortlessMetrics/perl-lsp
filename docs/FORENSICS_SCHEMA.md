# PR Forensics Schema

This document defines the schema for PR archaeology dossiers. Use this when mining merged PRs to extract lessons and evidence.

## Core Principle

> **The product isn't code. It's decisions + proof.**

AI does the typing. Humans do scope calls, design calls, verification calls, acceptance calls, and prevention calls. The artifacts we publish must center those decisions.

## Purpose

PR archaeology extracts actionable patterns from past work:

- What went wrong (and was caught vs. slipped through)
- What the combined budget actually was (DevLT + compute)
- Which guardrails improved because of the PR
- How claims drifted from implementation

## Quality-First Approach

Quality is the primary output. Budget metrics are secondary.

See [`QUALITY_SURFACES.md`](QUALITY_SURFACES.md) for the four quality surfaces:
- **Maintainability**: boundaries, coupling, complexity
- **Correctness**: tests, error paths, mutation survival
- **Governance**: schema alignment, anti-drift, receipts
- **Reproducibility**: gate clarity, limits, environment

## Dossier Structure: Four-Panel Model

Every PR dossier uses a **four-panel structure** to surface measured facts, then derives insights:

| Panel | Type | Question | Key Outputs |
| ----- | ---- | -------- | ----------- |
| 1. Change Surface | Measured | Where did this PR land? What got riskier? | Review map (3-8 priority files), integration points |
| 2. Verification Depth | Measured | What is actually proven now? | Proof bundle, "Not measured" explicit list |
| 3. Governance Integrity | Measured | Did we keep truth surfaces honest? | Claim drift list, schema compliance |
| 4. Temporal Topology | Derived | How did it converge? | 3-line convergence story, friction zones |

After the four panels come:
- **Panel 5**: Budget Estimates (with provenance)
- **Panel 6**: Quality Deltas (impact on four quality surfaces)
- **Panel 7**: Factory Delta (systemic improvements)
- **Panel 8**: Exhibit Score (overall assessment)
- **Panel 9**: Next Prevention Actions (actionable items to prevent recurrence)
- **Findings**: Exceptional issues only (optional)

---

## Panel 1: Change Surface (Measured)

**Goal**: Map the blast radius. Show what got touched, what hotspots emerged, what surface areas changed.

### Intent

| Field | Description |
| ----- | ----------- |
| PR number | GitHub PR reference |
| Issue/AC | Linked issue(s) and acceptance criteria |
| Stated goal | What the PR description claimed it would do |

### Directory Histogram (Top 10)

| Directory | Files Changed | Lines Delta | Notes |
| --------- | ------------- | ----------- | ----- |
| `crates/perl-parser/src/` | N | +X/-Y | (e.g., "parser core changes") |
| `crates/perl-lsp/src/` | N | +X/-Y | |
| `tests/` | N | +X/-Y | |
| `docs/` | N | +X/-Y | |

### Hotspots by Churn (Top 10 Files)

Files touched most frequently or with highest line churn in this PR.

| File Path | Commits | Delta | Notes |
| --------- | ------- | ----- | ----- |
| `crates/perl-parser/src/parser.rs` | 8 | +250/-120 | Core parsing logic |
| `crates/perl-lsp/src/server.rs` | 3 | +80/-40 | LSP server impl |

### Dependency Delta

| Change Type | Before | After | Risk Level |
| ----------- | ------ | ----- | ---------- |
| Added crate | - | `foo v1.2.3` | Low/Med/High |
| Version bump | `bar v2.0.0` | `bar v2.1.0` | Low/Med/High |
| Removed crate | `baz v0.5.0` | - | Low |

### Public API Delta

Functions/types/traits added, removed, or changed in public crate APIs.

| Change | Item | Crate | Breaking? |
| ------ | ---- | ----- | --------- |
| Added | `pub fn parse_extended()` | perl-parser | No |
| Changed | `IndexProvider::index()` signature | perl-parser | Yes |
| Removed | `deprecated_fn()` | perl-lsp | Yes |

### Unsafe/Concurrency Surface Deltas

| Metric | Before | After | Delta | Notes |
| ------ | ------ | ----- | ----- | ----- |
| Unsafe blocks | 3 | 5 | +2 | UTF-16 boundary ops |
| Arc/Mutex uses | 8 | 10 | +2 | Workspace indexing |
| Thread spawns | 2 | 2 | 0 | |

### Output: Review Map

**Priority files** (3-8 files requiring human review):
1. `crates/perl-parser/src/parser.rs` - Core logic changes
2. `crates/perl-lsp/src/index.rs` - New unsafe blocks

**Integration points** (external surface changes):
- LSP protocol: Added `textDocument/semanticTokens/full`
- File I/O: New workspace scanning logic

---

## Panel 2: Verification Depth (Measured)

**Goal**: Show what is actually proven by automated gates. Make gaps explicit.

### Canonical Gate Outcome

| Gate | Status | Duration | Link |
| ---- | ------ | -------- | ---- |
| `just ci-gate` | Pass/Fail | Xm Ys | CI run link |
| Formatting | Pass/Fail | | |
| Clippy | Pass/Fail | | |
| Tests | Pass/Fail | X tests | |
| Policy checks | Pass/Fail | | |

### Test Deltas

| Change Type | Count | Examples |
| ----------- | ----- | -------- |
| Tests added | 15 | `test_utf16_boundary_safety()` |
| Tests removed | 2 | Obsolete parser v2 tests |
| Tests ignored | 3 | `#[ignore]` with reason |
| Test LOC delta | +450 | |

### Mutation Testing Deltas

| Metric | Before | After | Delta | Notes |
| ------ | ------ | ----- | ----- | ----- |
| Mutation score | 85% | 87% | +2% | Target: 87% |
| Survivors | 45 | 38 | -7 | Improved kill rate |
| Timeouts | 2 | 1 | -1 | |

**Survivor list** (if applicable):
- `crates/perl-parser/src/lexer.rs:234` - Boundary condition
- `crates/perl-lsp/src/index.rs:567` - Error path

### Corpus/Fixtures Added

| Type | Count | Examples |
| ---- | ----- | -------- |
| Test corpus files | 8 | `test_corpus/unicode/` |
| Integration fixtures | 3 | `tests/fixtures/workspace_multi/` |
| Property test generators | 2 | UTF-16 quickcheck |

### Output: Proof Bundle

**What is proven**:
- UTF-16 boundary safety: Property tests + mutation coverage
- LSP compliance: Integration test suite
- Performance: Benchmark baseline established

**What is NOT measured** (explicit gaps):
- Real-world workspace performance beyond fixtures
- Client compatibility beyond VS Code
- Memory usage under sustained load

---

## Panel 3: Governance Integrity (Measured)

**Goal**: Show whether documentation, catalogs, and schemas stayed honest.

### Catalog Diffs (features.toml)

| Change Type | Feature | Before | After | Notes |
| ----------- | ------- | ------ | ----- | ----- |
| Added | `semanticTokens` | - | Full | New capability |
| Status change | `completion` | Partial | Full | Improved coverage |
| Removed | `deprecated_feature` | Partial | - | Cleaned up |

### Doc Drift Checks

| Check | Status | Notes |
| ----- | ------ | ----- |
| `just status-check` | Pass/Fail | Computed markers current? |
| CURRENT_STATUS.md markers | Pass/Fail | `BEGIN_COMPUTED` blocks updated? |
| Schema validation | Pass/Fail | Dossier follows FORENSICS_SCHEMA? |
| Receipt schema | Pass/Fail | Gate receipts valid? |

### Claim vs. Reality

| Claim Location | Claimed | Actual | Drift? |
| -------------- | ------- | ------ | ------ |
| PR description | "Adds semantic tokens" | Semantic tokens added | No |
| features.toml | `completion: Full` | 87% mutation score | No |
| ROADMAP.md | Milestone 0.9.0 | Delivered in 0.8.8 | Yes (early) |

### Output: Drift Report

**Corrected drifts**:
- Updated features.toml to reflect new capabilities
- Refreshed CURRENT_STATUS.md computed blocks

**Remaining gaps**:
- None

**Schema compliance**:
- Dossier: Valid
- Receipts: Valid
- Gate ledger: Valid

---

## Panel 4: Temporal Topology (Derived)

**Goal**: Show how the PR converged. Identify patterns of iteration, oscillation, stabilization.

### Burst Sessions

Periods of rapid commit activity (within 2-hour windows).

| Session | Timeframe | Commits | Focus |
| ------- | --------- | ------- | ----- |
| Session 1 | 2026-01-05 10:00-12:00 | 8 | Initial parser implementation |
| Session 2 | 2026-01-05 14:00-16:00 | 5 | Test refinement |

### Oscillation Patterns

Files/modules that were rewritten multiple times, dependencies added/removed/re-added, reverts.

| Pattern | Occurrences | Files/Items | Reason |
| ------- | ----------- | ----------- | ------ |
| Same hotspot rewrite | 3 | `parser.rs` | Edge case discovery |
| Dependency churn | 2 | `serde_json` add/remove/add | API design iteration |
| Reverts | 1 | Commit abc123 reverted | Performance regression |

### Stabilization Inflection

The point where logic changes stop and only tests/docs remain.

| Phase | Timeframe | Activity |
| ----- | --------- | -------- |
| Logic churn | Commits 1-15 | Parser implementation |
| Stabilization | Commits 16-20 | Test coverage, doc updates |
| Polish | Commits 21-23 | Clippy fixes, formatting |

**Inflection point**: Commit 15 (2026-01-05 16:30) - Last logic change before stabilization

### Friction Heatmap

Files touched across many commits, indicating uncertainty or complexity.

| File | Touches | Span (commits) | Friction Type |
| ---- | ------- | -------------- | ------------- |
| `parser.rs` | 12 | 1-20 | High complexity |
| `index.rs` | 8 | 5-18 | API uncertainty |
| `tests/integration.rs` | 6 | 10-23 | Evolving coverage |

### Output: Convergence Story

**3-line summary**:
1. Initial burst (commits 1-8): Core implementation with 3 rewrites of parser.rs
2. Oscillation phase (commits 9-15): API design iteration, dependency churn
3. Stabilization (commits 16-23): Test coverage and polish, no logic changes after commit 15

**Friction zones**:
- `parser.rs`: High complexity drove 12 touches across full PR lifespan
- `index.rs`: API uncertainty required 8 iterations

**Next prevention**:
- Pre-commit design review for parser.rs-level complexity
- API contract validation before implementation for index.rs-level uncertainty

---

## Panel 9: Next Prevention Actions

**Purpose**: Turn forensics into "do better next time" outputs. Every dossier must produce actionable items to prevent recurrence.

### Policy Constraints (Required)

These constraints ensure prevention actions are **deterministic and hard to forget**:

1. **Quality Delta Rule**: If *any* quality delta is `0`, `-1`, or `-2`, at least one prevention action **must** target that surface.
   - Example: If Correctness delta is `-1`, include at least one verification action (9.2).

2. **Friction Rule**: If friction existed (temporal topology shows high-churn files, oscillations, or gate failures), at least one prevention action **must** be a **mechanized guardrail** (gate/receipt/schema), not just "write better docs".
   - Example: If `parser.rs` had 12 touches, add a complexity check gate, not just a design guideline.

3. **Wrongness Rule**: If wrongness was discovered (claim drift, bug, stub), at least one prevention action **must** mechanize detection of that wrongness class.
   - Example: If claim drift occurred, add a drift detection script to ci-gate.

### Enforcement

Dossiers that violate these constraints should not be published until prevention actions are added. Reviewers should verify:

- [ ] Quality deltas at 0/-1/-2 have corresponding prevention actions
- [ ] Friction zones have at least one mechanized guardrail action
- [ ] Discovered wrongness has mechanized detection action

Three categories of actions, ranked by priority:

### 9.1 Code Health Actions (Maintainability)

Actions to improve maintainability and reduce future friction:

- **Split hotspots**: Files too frequently touched across commits
- **Reduce coupling**: Modules too interdependent, changes cascade
- **Clarify boundaries**: Unclear responsibility separation between modules
- **Shrink/document public surface**: Public API needs refinement or documentation

Each action item should include:

| Field | Description |
| ----- | ----------- |
| **What** | Specific change needed |
| **Where** | File path or component |
| **Why** | What problem it prevents |
| **Evidence** | How you know it's done |

**Example**:

| What | Where | Why | Evidence |
| ---- | ----- | --- | -------- |
| Split `parser.rs` into `parser/core.rs` and `parser/expressions.rs` | `crates/perl-parser/src/parser.rs` (800+ LOC) | Reduce churn frequency - file touched in 12/23 commits | File split complete, both <400 LOC, clippy passes |
| Extract `IndexProvider` trait to separate module | `crates/perl-lsp/src/index.rs` | Clarify LSP provider boundaries | Trait in `lsp/providers/index.rs`, tests pass |

### 9.2 Verification Actions (Correctness)

Actions to strengthen test coverage and proof quality:

- **Missing unhappy path tests**: Error conditions not exercised
- **Mutation survivors → targeted tests needed**: Mutation testing reveals gaps
- **New fixtures for edge cases discovered**: Integration test scenarios missing
- **Coverage gaps in changed code**: Changed code lacks test coverage

Each action item should include:

| Field | Description |
| ----- | ----------- |
| **What** | Specific test or fixture needed |
| **Where** | Test file or coverage gap location |
| **Why** | What correctness issue it prevents |
| **Evidence** | How you know it's done |

**Example**:

| What | Where | Why | Evidence |
| ---- | ----- | --- | -------- |
| Add property test for surrogate pair edge cases | `tests/property_utf16.rs` | Current tests missed 2-rewrite friction in utf16.rs | Quickcheck test added, utf16.rs mutation score >90% |
| Add integration fixture for multi-file workspace navigation | `tests/fixtures/workspace_multi/` | No test for dual indexing across files | Fixture exists, LSP navigation test passes |
| Test error recovery path in `index.rs:567` | `tests/lsp_index_test.rs` | Mutation survivor indicates untested error path | Mutation killed, test fails when error handling removed |

### 9.3 Factory Actions (Process)

Actions to improve the development factory itself:

- **New preflight gate to prevent this friction class**: Automated check missing
- **Tighten schema/contracts**: Documentation or API contract unclear
- **Add/adjust lane rules**: Workflow or routing logic needs refinement
- **New receipt requirement**: Quality evidence missing from gate

Each action item should include:

| Field | Description |
| ----- | ----------- |
| **What** | Specific process change |
| **Where** | Justfile, CI config, or documentation |
| **Why** | What friction class it prevents |
| **Evidence** | How you know it's done |

**Example**:

| What | Where | Why | Evidence |
| ---- | ----- | --- | -------- |
| Add `just check-complexity` gate (max 400 LOC/file) | `justfile`, `scripts/complexity-check.sh` | Prevent parser.rs-level hotspots (800+ LOC) | Gate fails on >400 LOC, documented in COMMANDS_REFERENCE.md |
| Require mutation score receipt in PR template | `.github/pull_request_template.md` | Ensure mutation testing runs before merge | Template updated, PR checklist includes mutation score |
| Add pre-implementation design template for >500 LOC changes | `docs/DESIGN_TEMPLATE.md` | Prevent API oscillation (8 touches to index.rs) | Template exists, used in next large PR |

### Output: Prevention Plan

**Summary**: Select 2-3 actions per category (6-9 total) that best address the PR's friction patterns.

**Priority ranking**: Order by expected impact on preventing similar friction in future PRs.

**Tracking**: Link to issues created for each action if implementation is deferred beyond current PR.

---

## Panel 5: Budget Estimates (with Provenance)

Every metric includes provenance. See [`METRICS_PROVENANCE.md`](METRICS_PROVENANCE.md).

| Metric | Value | Kind | Confidence | Basis |
| ------ | ----- | ---- | ---------- | ----- |
| DevLT | X–Ym | estimated | high/med/low | decision events, friction events |
| CI | Zm | measured | high | workflow run ID |
| LLM units | ~N | estimated | med | iteration count |

**DevLT estimation** uses decision-weighted method. See [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md).

Bands for DevLT (reference):
- **<30 min**: Quick fix, no exploration needed
- **30-120 min**: Standard feature, some investigation
- **120+ min**: Complex, multi-session work

Bands for compute (reference):
- **Cheap**: <10K tokens, <5 CI minutes
- **Moderate**: 10-100K tokens, 5-30 CI minutes
- **Expensive**: >100K tokens, >30 CI minutes

**Coverage declaration** (required):
- `receipts_included`: Agent logs, token receipts available
- `github_plus_agent_logs`: GitHub + session logs
- `github_only`: PR thread, commits, CI only

---

## Panel 6: Quality Deltas

Rate the PR's impact on each quality surface:

| Surface | Delta | Notes |
| ------- | ----- | ----- |
| Maintainability | +1/0/-1 | Boundary clarity, coupling, complexity |
| Correctness | +1/0/-1 | Test depth, error handling, mutation survival |
| Governance | +1/0/-1 | Schema alignment, anti-drift, receipts |
| Reproducibility | +1/0/-1 | Gate clarity, limits declared |

Delta scale:
- **+2**: Significant improvement
- **+1**: Minor improvement
- **0**: No change
- **-1**: Minor regression (justified)
- **-2**: Significant regression (requires justification)

---

## Panel 7: Factory Delta

What systemic improvement resulted from this PR:

| Guardrail | Before | After | Notes |
| --------- | ------ | ----- | ----- |
| (e.g., status-check) | Did not exist | Enforced in ci-gate | |
| (e.g., features.toml) | Manual tracking | Computed catalog | |

---

## Panel 8: Exhibit Score

Overall quality assessment:

| Dimension | Score (1-5) | Notes |
| --------- | ----------- | ----- |
| Clarity of intent | | Was the goal clear? |
| Scope discipline | | Did it stay in scope? |
| Evidence quality | | Were claims backed? |
| Test coverage | | Did tests match claims? |
| DevLT efficiency | | Human time well spent? |

---

## Findings (Optional)

Categorized discoveries for exceptional cases:

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

## Example Dossier (Four-Panel Structure)

```markdown
## PR #153: Mutation Testing + UTF-16 Security Fixes

---

## Panel 1: Change Surface (Measured)

### Intent
- **PR number**: #153
- **Issue/AC**: #148 (mutation testing), #150 (UTF-16 security)
- **Stated goal**: Add mutation testing, fix UTF-16 boundary vulnerabilities

### Directory Histogram (Top 10)
| Directory | Files Changed | Lines Delta | Notes |
| --------- | ------------- | ----------- | ----- |
| crates/perl-parser/src/ | 8 | +450/-120 | UTF-16 safety, mutation-tested paths |
| crates/perl-lsp/src/ | 3 | +80/-20 | LSP UTF-16 integration |
| tests/ | 12 | +600/-50 | Property tests, mutation fixtures |
| docs/ | 2 | +120/-10 | Security documentation |

### Hotspots by Churn (Top 5 Files)
| File Path | Commits | Delta | Notes |
| --------- | ------- | ----- | ----- |
| crates/perl-parser/src/utf16.rs | 6 | +280/-80 | Core UTF-16 conversion logic |
| crates/perl-parser/src/lsp/position.rs | 4 | +150/-30 | Position mapping |
| tests/property_utf16.rs | 5 | +320/-10 | Quickcheck generators |

### Dependency Delta
| Change Type | Before | After | Risk Level |
| ----------- | ------ | ----- | ---------- |
| Added crate | - | `cargo-mutants v24.11.0` | Low (dev-dep) |
| Added crate | - | `quickcheck v0.9.x` | Low (test-dep) |

### Public API Delta
| Change | Item | Crate | Breaking? |
| ------ | ---- | ----- | --------- |
| Added | `pub fn utf16_to_utf8_safe()` | perl-parser | No |
| Added | `pub fn utf8_to_utf16_safe()` | perl-parser | No |

### Unsafe/Concurrency Surface Deltas
| Metric | Before | After | Delta | Notes |
| ------ | ------ | ----- | ----- | ----- |
| Unsafe blocks | 3 | 5 | +2 | UTF-16 boundary validation |
| Arc/Mutex uses | 8 | 8 | 0 | |

### Output: Review Map
**Priority files** (3):
1. crates/perl-parser/src/utf16.rs - New unsafe blocks, boundary validation
2. crates/perl-parser/src/lsp/position.rs - LSP position mapping changes
3. tests/property_utf16.rs - Property test coverage

**Integration points**:
- LSP protocol: UTF-16 position conversions
- Parser: Safe string boundary handling

---

## Panel 2: Verification Depth (Measured)

### Canonical Gate Outcome
| Gate | Status | Duration | Link |
| ---- | ------ | -------- | ---- |
| `just ci-gate` | Pass | 8m 45s | [Run #xyz](link) |
| Formatting | Pass | 12s | |
| Clippy | Pass | 2m 10s | |
| Tests | Pass | 4m 20s (423 tests) | |
| cargo-mutants | Pass | 12m 15s | 87% score |

### Test Deltas
| Change Type | Count | Examples |
| ----------- | ----- | -------- |
| Tests added | 18 | `test_utf16_boundary_safety()`, `prop_utf16_roundtrip()` |
| Tests removed | 0 | |
| Tests ignored | 0 | |
| Test LOC delta | +600 | Property tests, edge cases |

### Mutation Testing Deltas
| Metric | Before | After | Delta | Notes |
| ------ | ------ | ----- | ----- | ----- |
| Mutation score | N/A | 87% | New | Initial baseline established |
| Survivors | N/A | 38 | New | Documented in mutation.txt |
| Timeouts | N/A | 1 | New | Long-running integration test |

**Survivor list**:
- crates/perl-parser/src/lexer.rs:234 - Comment parsing boundary
- crates/perl-lsp/src/index.rs:567 - Error recovery path

### Corpus/Fixtures Added
| Type | Count | Examples |
| ---- | ----- | -------- |
| Property test generators | 3 | UTF-16 quickcheck, unicode boundary |
| Integration fixtures | 2 | test_corpus/unicode/ |

### Output: Proof Bundle
**What is proven**:
- UTF-16 boundary safety: Property tests with 1000 iterations each
- Symmetric conversion: utf8→utf16→utf8 verified
- Mutation coverage: 87% of UTF-16 code paths exercised

**What is NOT measured** (explicit gaps):
- Real-world LSP client compatibility (beyond VS Code)
- Performance impact of safe conversion overhead
- Unicode edge cases beyond BMP (surrogate pairs partially covered)

---

## Panel 3: Governance Integrity (Measured)

### Catalog Diffs (features.toml)
| Change Type | Feature | Before | After | Notes |
| ----------- | ------- | ------ | ----- | ----- |
| Status change | `utf16_safety` | - | Full | New capability tracked |

### Doc Drift Checks
| Check | Status | Notes |
| ----- | ------ | ----- |
| `just status-check` | Pass | Computed blocks updated |
| CURRENT_STATUS.md markers | Pass | Security section added |
| Schema validation | Pass | Dossier follows FORENSICS_SCHEMA v2 |
| Receipt schema | Pass | Gate receipts valid |

### Claim vs. Reality
| Claim Location | Claimed | Actual | Drift? |
| -------------- | ------- | ------ | ------ |
| PR description | "Adds mutation testing" | cargo-mutants added to CI | No |
| PR description | "Fixes UTF-16 vulnerabilities" | Property tests + safe APIs added | No |
| Issue #150 | "87% mutation target" | 87% achieved | No |

### Output: Drift Report
**Corrected drifts**: None

**Remaining gaps**: None

**Schema compliance**:
- Dossier: Valid (FORENSICS_SCHEMA v2)
- Receipts: Valid (gate ledger + mutation report)
- Gate ledger: Valid

---

## Panel 4: Temporal Topology (Derived)

### Burst Sessions
| Session | Timeframe | Commits | Focus |
| ------- | --------- | ------- | ----- |
| Session 1 | 2026-01-05 09:00-11:30 | 8 | UTF-16 implementation |
| Session 2 | 2026-01-05 14:00-16:00 | 6 | Property test coverage |
| Session 3 | 2026-01-06 10:00-11:00 | 4 | Mutation testing integration |

### Oscillation Patterns
| Pattern | Occurrences | Files/Items | Reason |
| ------- | ----------- | ----------- | ------ |
| Same hotspot rewrite | 2 | utf16.rs | Edge case discovery (surrogate pairs) |
| Dependency churn | 0 | - | - |
| Reverts | 0 | - | - |

### Stabilization Inflection
| Phase | Timeframe | Activity |
| ----- | --------- | -------- |
| Logic churn | Commits 1-10 | UTF-16 implementation |
| Stabilization | Commits 11-15 | Property test refinement |
| Polish | Commits 16-18 | Documentation, mutation fixes |

**Inflection point**: Commit 10 (2026-01-05 15:30) - Last UTF-16 logic change

### Friction Heatmap
| File | Touches | Span (commits) | Friction Type |
| ---- | ------- | -------------- | ------------- |
| utf16.rs | 6 | 1-12 | Edge case complexity |
| property_utf16.rs | 5 | 7-15 | Test coverage iteration |

### Output: Convergence Story
**3-line summary**:
1. Initial implementation (commits 1-8): UTF-16 conversion with 2 rewrites for surrogate pairs
2. Test coverage phase (commits 9-15): Property tests reveal edge cases, final logic fix
3. Integration phase (commits 16-18): Mutation testing added, documentation polished

**Friction zones**:
- utf16.rs: Surrogate pair handling required 2 rewrites across commits 1-12
- property_utf16.rs: Test generator refinement across 5 commits

**Next prevention**:
- Pre-implementation Unicode research for surrogate pair handling
- Quickcheck integration earlier in development cycle

---

## Panel 5: Budget Estimates (with Provenance)

| Metric | Value | Kind | Confidence | Basis |
| ------ | ----- | ---- | ---------- | ----- |
| DevLT | 75-105m | estimated | medium | 4 decision events, 1 friction (utf16.rs rewrites) |
| CI | 15m | measured | high | workflow run #xyz |
| LLM units | ~6 | estimated | medium | 3 iteration cycles (impl, tests, integration) |

Coverage: `github_plus_agent_logs`

---

## Panel 6: Quality Deltas

| Surface | Delta | Notes |
| ------- | ----- | ----- |
| Maintainability | +1 | Security boundary now explicit, unsafe blocks documented |
| Correctness | +2 | Mutation testing (87%) + UTF-16 property tests |
| Governance | +1 | Added cargo-mutants gate, schema compliance |
| Reproducibility | 0 | No change to gate structure |

---

## Panel 7: Factory Delta

| Guardrail | Before | After | Notes |
| --------- | ------ | ----- | ----- |
| cargo-mutants | Not enforced | Enforced in ci-gate | 87% target |
| UTF-16 property tests | Did not exist | Quickcheck test suite | 1000 iterations/test |
| Security documentation | Minimal | docs/SECURITY.md updated | UTF-16 boundary guidance |

---

## Panel 8: Exhibit Score

| Dimension | Score (1-5) | Notes |
| --------- | ----------- | ----- |
| Clarity of intent | 5 | Well-documented intent, clear AC |
| Scope discipline | 4 | Minor scope expansion to related security docs |
| Evidence quality | 5 | Mutation score receipts, property test logs |
| Test coverage | 5 | Comprehensive: property tests + mutation testing |
| DevLT efficiency | 4 | Reasonable for scope, minimal friction |

---

## Panel 9: Next Prevention Actions

### 9.1 Code Health Actions (Maintainability)

| What | Where | Why | Evidence |
| ---- | ----- | --- | -------- |
| Document unsafe block preconditions in utf16.rs | `crates/perl-parser/src/utf16.rs` (5 unsafe blocks) | 2 rewrites for surrogate pairs indicate complexity | Doc comments added for all unsafe blocks, reviewed |
| Extract boundary validation to separate module | `crates/perl-parser/src/utf16/validation.rs` | Separate concerns: conversion vs. validation | Module exists, tests pass, <200 LOC |

### 9.2 Verification Actions (Correctness)

| What | Where | Why | Evidence |
| ---- | ----- | --- | -------- |
| Add property test for BMP edge cases | `tests/property_utf16.rs` | Current coverage partial for non-BMP unicode | Test added, 1000 iterations with BMP boundaries |
| Test mutation survivor in lexer.rs:234 | `tests/lexer_test.rs` | Comment parsing boundary untested | New test kills mutation, lexer.rs score >90% |
| Add integration fixture for LSP client compatibility | `tests/fixtures/lsp_clients/` | Only VS Code tested, gap acknowledged | Emacs, Neovim fixtures added |

### 9.3 Factory Actions (Process)

| What | Where | Why | Evidence |
| ---- | ----- | --- | -------- |
| Add cargo-mutants to pre-push hook | `scripts/install-githooks.sh` | Prevent mutation score regression | Hook runs mutants on changed files only |
| Create Unicode research checklist | `docs/UNICODE_CHECKLIST.md` | Prevent surrogate pair discovery during implementation | Checklist exists, includes surrogate pair section |

**Priority ranking**:
1. Document unsafe blocks (immediate maintainability win)
2. Add mutation survivor tests (close correctness gap)
3. Add cargo-mutants to pre-push (prevent regression)
4. Create Unicode checklist (prevent similar friction)
5. Extract validation module (reduce future churn)
6. Add BMP property tests (expand correctness coverage)

**Tracking**: Issues #160-#165 created for deferred actions

---

## Findings

None - clean PR.
```

## How to Use

1. Pick a merged PR to analyze
2. Create `docs/forensics/pr-NNN.md`
3. Fill in the **four measurement panels first** (Change Surface, Verification Depth, Governance Integrity, Temporal Topology)
4. Derive budget estimates, quality deltas, factory improvements from measured data
5. Complete exhibit scoring
6. Add systemic findings to `docs/LESSONS.md` if patterns emerge
7. Update guardrails if gaps are found

### Panel Completion Order

**Phase 1: Measured Facts** (automation-friendly)
1. Panel 1 (Change Surface) - Use git tooling to extract file deltas, churn, dependencies
2. Panel 2 (Verification Depth) - Parse CI logs, test output, mutation reports
3. Panel 3 (Governance Integrity) - Run schema validators, diff catalog files

**Phase 2: Derived Insights** (human analysis)
4. Panel 4 (Temporal Topology) - Analyze commit timeline for patterns
5. Panel 5 (Budget Estimates) - Apply DevLT estimation method
6. Panel 6 (Quality Deltas) - Assess impact on quality surfaces

**Phase 3: Synthesis**
7. Panel 7 (Factory Delta) - Identify systemic improvements
8. Panel 8 (Exhibit Score) - Overall assessment
9. Panel 9 (Next Prevention Actions) - Actionable items to prevent recurrence
10. Findings (optional) - Document exceptional issues

### Backward Compatibility

Existing dossiers following the 8-section structure remain valid. The four-panel model is a reorganization and enhancement of the existing schema, not a breaking change. Tools reading dossiers should support both formats.

## See Also

- [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md) - Decision-weighted DevLT method
- [`METRICS_PROVENANCE.md`](METRICS_PROVENANCE.md) - Provenance schema for all metrics
- [`QUALITY_SURFACES.md`](QUALITY_SURFACES.md) - The four quality surfaces
- [`ANALYZER_FRAMEWORK.md`](ANALYZER_FRAMEWORK.md) - Specialist analyzer specs
- [`LESSONS.md`](LESSONS.md) - Aggregated wrongness log
- [`AGENTIC_DEV.md`](AGENTIC_DEV.md) - Budget definitions and workflow
