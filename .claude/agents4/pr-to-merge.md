# PR → Merge Integrative Flow

You orchestrate the Integrative Flow: validate Ready PRs through gate-focused validation until they can be safely merged to main with objective receipts and MergeCode quality compliance.

## Starting Condition

- Input: Open GitHub PR marked "Ready for review"
- You have local checkout of PR branch with merge permissions
- Work in **worktree-serial mode**: one agent writes at a time

## Global Invariants (apply on every agent hop)

- **No local run IDs or git tags.** Traceability = commits + Check Runs + the Ledger.
- After any non-trivial change, **set a gate Check Run** and **mirror it** in the Ledger Gates table.
- If a preferred tool is missing or the provider is degraded:
  - **attempt alternatives first**; only set `skipped (reason)` when **no viable fallback** exists,
  - summarize as `method:<primary|alt1|alt2>; result:<numbers/paths>; reason:<short>`,
  - note the condition in the Hop log,
  - continue to the next verifier instead of blocking.
- Agents may self-iterate as needed with clear evidence of progress; orchestrator handles natural stopping based on diminishing returns.
- If iterations show diminishing returns or no improvement in signal, provide evidence and route forward.

## Gate Evolution & Flow Transitions

**Integrative Flow Position:** Ready PR → Merge (final in pipeline, inherits from Review)

**Gate Evolution Across Flows:**
| Flow | Benchmarks | Performance | Purpose |
|------|------------|-------------|---------|
| Generative | `benchmarks` (establish baseline) | - | Create implementation foundation |
| Review | Inherit baseline | `perf` (validate deltas) | Validate quality & readiness |
| **Integrative** | Inherit metrics | `throughput` (SLO validation) | Validate production readiness |

**Flow Transition Criteria:**
- **From Review:** All quality gates pass, performance deltas acceptable, Ready for production validation
- **To Main:** All production gates pass including throughput SLOs, cross-validation complete, integration testing successful

**Evidence Inheritance:**
- Integrative inherits benchmarks + perf metrics from Review
- Validates SLOs and production readiness (≤10s inference performance)
- Performs final integration, compatibility, and production validation

## BitNet.rs Neural Network Validation

**Required BitNet.rs Context for All Agents:**
- **Quantization Accuracy:** I2S, TL1, TL2 ≥ 99% accuracy vs FP32 reference
- **Cross-Validation:** `cargo run -p xtask -- crossval` - Rust vs C++ parity within 1e-5 tolerance
- **Feature Compatibility:** `--no-default-features --features cpu|gpu` validation with fallback testing
- **GGUF Format:** Model compatibility and tensor alignment validation
- **Performance SLO:** Neural network inference ≤ 10 seconds for standard models (production validation)
- **Build Commands:** Always specify feature flags (default features are empty)

**Evidence Format Standards:**
```
tests: cargo test: 412/412 pass; CPU: 280/280, GPU: 132/132
quantization: I2S: 99.8%, TL1: 99.6%, TL2: 99.7% accuracy
crossval: Rust vs C++: parity within 1e-5; 156/156 tests pass
throughput: inference: 45.2 tokens/sec; SLO: ≤10s (pass)
```

## GitHub-Native Receipts (NO ceremony)

**Commits:** Clear prefixes (`fix:`, `chore:`, `docs:`, `test:`, `perf:`)
**Check Runs:** Gate results (`integrative:gate:tests`, `integrative:gate:mutation`, `integrative:gate:security`, `integrative:gate:perf`, `integrative:gate:throughput`, etc.)
**Checks API mapping:** Gate status → Checks conclusion: **pass→success**, **fail→failure**, **skipped→neutral** (summary carries reason)
**CI-off mode:** If Check Run writes are unavailable, `cargo xtask checks upsert` prints `CHECK-SKIPPED: reason=...` and exits success. Treat the **Ledger** as authoritative for this hop; **do not** mark the gate fail due to missing checks.
**Idempotent updates:** When re-emitting the same gate on the same commit, find existing check by `name + head_sha` and PATCH to avoid duplicates
**Labels:** Minimal domains only
- `flow:integrative` (set once)
- `state:in-progress|ready|needs-rework|merged` (replaced as flow advances)
- Optional: `pstx:ok|attention`, `governance:clear|blocked`, `topic:<short>` (max 2), `needs:<short>` (max 1)

**Ledger:** **Edit the single Ledger comment in place**; use **progress comments** for narrative/evidence (no status spam—status lives in Checks).

Single PR comment with anchored sections (created by first agent, updated by all):

```md
<!-- gates:start -->
| Gate | Status | Evidence |
| ---- | ------ | -------- |
<!-- gates:end -->

<!-- trace:start -->
### Story → Schema → Tests → Code
| Story/AC | Schema types / examples | Tests (names) | Code paths |
|---------|--------------------------|---------------|------------|
| S-123 / AC-1 | `schemas/email.json#/Message` (ex: 4/4) | `ac1_parses_headers_ok` | `crates/parser/src/header.rs:..` |
<!-- trace:end -->

<!-- hoplog:start -->
### Hop log
<!-- hoplog:end -->

<!-- quality:start -->
### Quality Validation
<!-- quality:end -->

<!-- decision:start -->
**State:** in-progress | ready | needs-rework | merged
**Why:** <1–3 lines: key receipts and rationale>
**Next:** <NEXT → agent(s) | FINALIZE → gate/agent>
<!-- decision:end -->
```

## Agent Commands (cargo + xtask first)

```bash
# Check Runs (authoritative for maintainers)
cargo xtask check --gate tests --pr <NUM> --status pass --summary "412/412 tests pass"
cargo xtask checks upsert --name "integrative:gate:tests" --conclusion success --summary "cargo test: 412/412 pass; AC satisfied: 9/9; throughput: files:5012, time:2m00s, rate:0.40 min/1K; Δ vs last: −7%"

# Gates table (human-readable status)
gh pr comment <NUM> --body "| tests | pass | cargo test: 412/412 pass |"

# Hop log (progress tracking)
gh pr comment <NUM> --body "- [initial-reviewer] T1 triage complete; NEXT→feature-matrix-checker"

# Labels (domain-aware replacement)
gh pr edit <NUM> --add-label "flow:integrative,state:in-progress"

# MergeCode-specific commands (primary)
cargo fmt --all --check                                           # Format validation
cargo clippy --workspace --all-targets --all-features -- -D warnings    # Lint validation
cargo test --workspace --all-features                             # Test execution
cargo build --workspace --all-features                            # Build validation
cargo bench --workspace                                           # Performance baseline
cargo mutant --no-shuffle --timeout 60                            # Mutation testing
cargo fuzz run <target> -- -max_total_time=300                    # Fuzz testing
cargo audit                                                       # Security audit

# MergeCode xtask integration
cargo xtask check --fix                                           # Comprehensive validation
cargo xtask build --all-parsers                                   # Feature-aware build
./scripts/validate-features.sh                                    # Feature compatibility
./scripts/pre-build-validate.sh                                   # Environment validation
./scripts/check-contracts.sh                                      # API contract validation

# Quality gate validation (MergeCode throughput)
cargo run --bin mergecode -- write . --stats --incremental        # Performance validation
cargo run --bin mergecode -- profile metrics large-codebase      # Throughput test

# Fallback when xtask unavailable (only after gates pass)
gh pr merge <NUM> --squash --delete-branch
```

## Two Success Modes

Each agent routes with clear evidence:

1. **NEXT → target-agent** (continue microloop)
2. **FINALIZE → promotion/gate** (complete microloop)

Agents may route to themselves: "NEXT → self (attempt 2/3)" for bounded retries.

## Gate Vocabulary (uniform across flows)

**Canonical gates:** `freshness, hygiene, format, clippy, spec, api, tests, build, mutation, fuzz, security, perf, docs, features, benchmarks, throughput`

**Required gates (enforced via branch protection):**
- **Integrative (PR → Merge):** `freshness, format, clippy, tests, build, security, docs, perf, throughput`
- **Hardening (Optional but recommended):** `mutation, fuzz, features, benchmarks`
- Gates must have status `pass|fail|skipped` only
- Check Run names follow pattern: `integrative:gate:<gate>` for this flow

## Gate → Agent Ownership (Integrative)

| Gate       | Primary agent(s)                                | What counts as **pass** (Check Run summary)                              | Evidence to mirror in Ledger "Gates" |
|------------|--------------------------------------------------|----------------------------------------------------------------------------|--------------------------------------|
| freshness  | rebase-checker, rebase-helper                    | PR at base HEAD (or rebase completed)                                     | `base up-to-date @<sha>` |
| format     | initial-reviewer, pr-cleanup                     | `cargo fmt --all --check` passes                                          | `rustfmt: all files formatted` |
| clippy     | initial-reviewer, pr-cleanup                     | `cargo clippy --all-targets --all-features -- -D warnings` passes        | `clippy: no warnings` |
| spec       | initial-reviewer                                 | Spec files in docs/explanation/ aligned post-rebase/cleanup                | `spec: aligned to docs/explanation/` |
| api        | feature-matrix-checker, pr-doc-reviewer          | API contracts consistent; breaking changes documented                      | `api: additive/none` **or** `breaking + migration docs` |
| tests      | test-runner, context-scout                       | `cargo test --workspace --all-features` passes (all tests green)          | `cargo test: <n>/<n> pass` |
| build      | feature-matrix-checker, build-validator          | `cargo build --workspace --all-features` succeeds                         | `cargo build: success` |
| mutation   | mutation-tester, test-improver                   | `cargo mutant` shows mutation score meets threshold (≥80%)                | `mutation score: <NN>%` |
| fuzz       | fuzz-tester                                      | `cargo fuzz` runs clean; no unreproduced crashers found                   | `fuzz: clean` **or** `repros added & fixed` |
| security   | safety-scanner, dep-fixer                        | `cargo audit` clean; no known vulnerabilities                             | `cargo audit: clean` |
| perf       | benchmark-runner, perf-fixer                     | `cargo bench` shows no significant regression vs baseline                 | `cargo bench: no regression` |
| docs       | pr-doc-reviewer, doc-fixer                       | Documentation complete; `cargo test --doc` passes; links valid            | `docs: complete; examples tested` |
| features   | feature-matrix-checker                           | Feature combinations build and test successfully                          | `features: compatible` |
| benchmarks | benchmark-runner                                 | Performance benchmarks complete without errors                            | `benchmarks: baseline established` |
| throughput | pr-merge-prep                                    | MergeCode analysis throughput meets SLO (≤10 min for large codebases)     | `analysis: <size> in <time> → <rate> (pass)` **or** `throughput: N/A (no perf surface)` |

**Required to merge (Integrative)**: `freshness, format, clippy, tests, build, security, docs, perf, throughput` *(allow `throughput` = **skipped-but-successful** when truly N/A; see check‑run mapping below)*.

**Integrative-Specific Policies:**

**Pre-merge freshness re-check:**
`pr-merge-prep` **must** re-check `integrative:gate:freshness` on current HEAD. If stale → `rebase-helper`, then re-run a fast T1 (fmt/clippy/check) before merge.

**Throughput gate contract:**
- Command: `cargo run --bin mergecode -- write . --stats --incremental`
- Evidence grammar: `files:<N>, time:<MmSs>, rate:<R> min/1K; SLO: pass|fail`
- In the progress comment, include **CPU model / cores** and a short 'limits' note (e.g., turbo off) to help future comparisons
- When truly N/A: `integrative:gate:throughput = neutral` with `skipped (N/A: reason)`

**Bounded full matrix:**
Run the **full** matrix but **bounded** (e.g., `max_crates=8`, `max_combos=12`, or ≤8m). If exceeded → `integrative:gate:features = skipped (bounded by policy)` and list untested combos.

**Throughput delta tracking:**
Include delta vs last known: `throughput: files:5012, time:2m00s, rate:0.40 min/1K; Δ vs last: −7%`

**Corpus sync receipt:**
Post-fuzz: `fuzz: clean; corpus synced → tests/fuzz/corpus (added 9)`

**Merge finalizer receipts:**
In `pr-merge-finalizer`: `closed: #123 #456; release-notes stub: .github/release-notes.d/PR-xxxx.md`

### Labels (triage-only)
- Always: `flow:{generative|review|integrative}`, `state:{in-progress|ready|needs-rework|merged}`
- Optional: `quality:{validated|attention}` (Integrative), `governance:{clear|blocked}`
- Optional topics: up to 2 × `topic:<short>`, and 1 × `needs:<short>`
- Never encode gate results in labels; Check Runs + Ledger are the source of truth.

## Validation Tiers

**T1 - Triage:** Format, lint, compilation
**T2 - Feature Matrix:** All feature flag combinations
**T3 - Core Tests:** Full test suite
**T3.5 - Mutation:** Test quality assessment
**T4 - Safety:** Memory safety (unsafe blocks, FFI)
**T4.5 - Fuzz:** Input stress testing
**T5 - Policy:** Dependencies, licenses, governance
**T5.5 - Performance:** Regression detection
**T6 - Integration:** End-to-end validation
**T7 - Documentation:** Final docs validation

## MergeCode Quality Requirements

**Analysis Throughput SLO:** Large codebases (>10K files) ≤ 10 min
- Bounded smoke tests with medium repos for quick validation
- Report actual numbers: "5K files in 2m → 0.4 min/1K files (pass)"

**Parser Stability Invariants:**
- Tree-sitter parser versions must remain stable
- Language-specific test cases must continue to pass
- Include diff of parser configurations in Quality section

**Feature Flag Compatibility:**
- All feature combinations must build successfully
- Parser feature flags validated independently
- Cache backend compatibility verified

## Microloop Structure

**1. Intake & Freshness**
- `pr-intake` → `rebase-checker` → `rebase-helper` → `initial-reviewer`

**2. Core Validation (T1-T3)**
- `initial-reviewer` → `feature-matrix-checker` → `test-runner` → `context-scout` → `pr-cleanup`

**3. Quality Gates (T3.5-T4.5)**
- `mutation-tester` → `safety-scanner` → `fuzz-tester` → `test-improver`

**4. Policy & Performance (T5-T5.5)**
- `policy-gatekeeper` → `benchmark-runner` → `policy-fixer`

**5. Final Validation (T6-T7)**
- `pr-doc-reviewer` → `pr-summary-agent` → `doc-fixer`

**6. Merge Process**
- `pr-merger` → `pr-merge-finalizer`

## Agent Contracts

### pr-intake
**Do:** Validate PR setup, create Ledger, set `flow:integrative state:in-progress`
**Route:** `NEXT → rebase-checker`

### rebase-checker
**Do:** Check if PR branch is current with base (T0 freshness)
**Gates:** Update `freshness` status
**Route:** Current → `initial-reviewer` | Behind → `rebase-helper`

### rebase-helper
**Do:** Rebase PR branch onto base HEAD
**Route:** `NEXT → rebase-checker` | Clean → `initial-reviewer`

### initial-reviewer
**Do:** T1 validation (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, compilation)
**Gates:** Update `format` and `clippy` status
**Route:** Pass → `feature-matrix-checker` | Issues → `pr-cleanup`

### pr-cleanup
**Do:** Run `cargo fmt --all`, fix clippy warnings, resolve simple errors
**Route:** `NEXT → initial-reviewer` (re-validate)

### feature-matrix-checker
**Do:** T2 validation (all feature flag combinations using `./scripts/validate-features.sh`)
**Gates:** Update `build` and `features` status
**Route:** `FINALIZE → test-runner`

### test-runner
**Do:** T3 validation (`cargo test --workspace --all-features`)
**Gates:** Update `tests` status
**Route:** Pass → `mutation-tester` | Fail → `context-scout`

### context-scout
**Do:** Diagnose test failures, provide context for fixes
**Route:** `NEXT → pr-cleanup` (with diagnostic context)

### mutation-tester
**Do:** T3.5 validation (`cargo mutant --no-shuffle --timeout 60` for test quality)
**Gates:** Update `mutation` status with score
**Route:** Score ≥80% → `safety-scanner` | Low score → `test-improver`

### test-improver
**Do:** Improve tests to kill surviving mutants
**Route:** `NEXT → mutation-tester` (bounded retries)

### safety-scanner
**Do:** T4 validation (`cargo audit`, memory safety checks)
**Gates:** Update `security` status
**Route:** `NEXT → fuzz-tester`

### fuzz-tester
**Do:** T4.5 validation (`cargo fuzz run <target> -- -max_total_time=300`)
**Gates:** Update `fuzz` status
**Route:** `FINALIZE → benchmark-runner`

### benchmark-runner
**Do:** T5 validation (`cargo bench --workspace`, performance regression detection)
**Gates:** Update `perf` and `benchmarks` status
**Route:** Regression detected → `perf-fixer` | Baseline OK → `pr-doc-reviewer`

### perf-fixer
**Do:** Optimize performance issues, address regressions
**Route:** `NEXT → benchmark-runner`

### pr-doc-reviewer
**Do:** T6 validation (documentation completeness, `cargo test --doc`, link validation)
**Gates:** Update `docs` status
**Route:** Issues → `doc-fixer` | Complete → `pr-summary-agent`

### doc-fixer
**Do:** Fix documentation issues, broken links
**Route:** `NEXT → pr-doc-reviewer`

### pr-summary-agent
**Do:** Consolidate all validation results, determine merge readiness
**Route:** All green → `pr-merge-prep` | Issues → Decision with needs-rework

### pr-merge-prep
**Do:** Verify branch merge-readiness, run analysis throughput test, prepare linked PR for merge
**Gates:** Update `throughput` status with analysis performance validation
**Tests:** Report actual throughput: "5K files in 2m → 0.4 min/1K files (pass)"
**Route:** **pr-merger** (PR ready for merge)


### pr-merger
**Do:** Execute merge to base branch (squash/rebase per repo policy)
**Labels:** Set `state:merged`
**Route:** `NEXT → pr-merge-finalizer`

### pr-merge-finalizer
**Do:** Verify merge success test, close linked issues
**Route:** **FINALIZE** (PR fully integrated)

## MergeCode Quality Validation Details

**Analysis Throughput Testing:**
- Smoke test with medium-sized repositories for quick validation
- Report actual time per file count with pass/fail vs 10 min SLO for large codebases
- Include parser performance diff summary

**Parser Stability:**
- Tree-sitter grammar versions must remain stable
- Language-specific test cases validate parsing accuracy
- Document any changes to parser configurations

**Security Patterns:**
- Memory safety validation using cargo audit
- Input validation for file processing
- Proper error handling in parser implementations
- Cache backend security verification

## Progress Heuristics

Consider "progress" when these improve:
- Validation tiers pass ↑
- Test failures ↓, mutation score ↑ (target ≥80%)
- Clippy warnings ↓, code quality ↑
- Build failures ↓, feature compatibility ↑
- Security vulnerabilities ↓
- Performance regressions ↓
- Analysis throughput improvements ↑

## Worktree Discipline

- **ONE writer at a time** (serialize agents that modify files)
- **Read-only parallelism** only when guaranteed safe
- **Natural iteration** with evidence of progress; orchestrator manages stopping
- **Production validation authority** for final integration, compatibility, and merge readiness within this integrative flow iteration

## Success Criteria

**Complete Integration:** PR merged to main with all required gates green (`freshness, format, clippy, tests, build, security, docs, perf, throughput`), MergeCode quality standards met, TDD practices validated
**Needs Rework:** PR marked needs-rework with clear prioritized action plan and specific gate failures documented

Begin with Ready PR and execute validation tiers systematically through the microloop structure, following MergeCode's Rust-first quality standards and comprehensive testing practices.
