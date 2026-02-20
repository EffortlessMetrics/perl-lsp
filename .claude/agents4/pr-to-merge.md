# PR → Merge Integrative Flow

You orchestrate the Integrative Flow: validate Ready PRs through gate-focused validation until they can be safely merged to main with objective receipts and Perl LSP production readiness compliance.

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
| **Integrative** | Inherit metrics | `parsing` (SLO validation) | Validate production readiness |

**Flow Transition Criteria:**
- **From Review:** All quality gates pass, performance deltas acceptable, Ready for production validation
- **To Main:** All production gates pass including parsing SLOs, LSP server stability complete, workspace integration successful

**Evidence Inheritance:**
- Integrative inherits benchmarks + perf metrics from Review
- Validates SLOs and production readiness (≤1ms parsing performance)
- Performs final integration, compatibility, and LSP server validation

## Perl LSP Production Validation

**Required Perl LSP Context for All Agents:**
- **Parsing Performance:** ≤1ms for incremental updates with 70-99% node reuse efficiency
- **LSP Protocol Compliance:** ~89% LSP features functional with comprehensive workspace support
- **Cross-File Navigation:** 98% reference coverage with dual indexing (qualified/unqualified patterns)
- **Package Testing:** `cargo test -p perl-parser`, `cargo test -p perl-lsp`, `cargo test -p perl-lexer` validation
- **Performance SLO:** Perl parsing and LSP operations ≤1ms for production validation
- **Threading Configuration:** Adaptive threading with `RUST_TEST_THREADS=2` for LSP tests

**Evidence Format Standards:**
```
tests: cargo test: 295/295 pass; parser: 180/180, lsp: 85/85, lexer: 30/30
parsing: 1-150μs per file, incremental: <1ms updates; SLO: pass
lsp: ~89% features functional; workspace navigation: 98% coverage
threading: RUST_TEST_THREADS=2; behavioral tests: 0.31s
```

## GitHub-Native Receipts (NO ceremony)

**Commits:** Clear prefixes (`fix:`, `chore:`, `docs:`, `test:`, `perf:`)
**Check Runs:** Gate results (`integrative:gate:tests`, `integrative:gate:mutation`, `integrative:gate:security`, `integrative:gate:perf`, `integrative:gate:parsing`, etc.)
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
gh api -X POST repos/:owner/:repo/check-runs -f name="integrative:gate:tests" -f head_sha="$(git rev-parse HEAD)" -f status=completed -f conclusion=success -f output[summary]="cargo test: 295/295 pass; parser: 180/180, lsp: 85/85, lexer: 30/30"
gh api -X POST repos/:owner/:repo/check-runs -f name="integrative:gate:parsing" -f head_sha="$(git rev-parse HEAD)" -f status=completed -f conclusion=success -f output[summary]="parsing: 1-150μs per file, incremental: <1ms updates; SLO: pass"

# Gates table (human-readable status)
gh pr comment <NUM> --body "| tests | pass | cargo test: 295/295 pass; parser: 180/180, lsp: 85/85, lexer: 30/30 |"

# Hop log (progress tracking)
gh pr comment <NUM> --body "- [initial-reviewer] T1 triage complete; NEXT→feature-matrix-checker"

# Labels (domain-aware replacement)
gh pr edit <NUM> --add-label "flow:integrative,state:in-progress"

# Perl LSP-specific commands (primary)
cargo fmt --workspace --check                                     # Format validation
cargo clippy --workspace                                          # Lint validation
cargo test                                                        # Test execution (adaptive threading)
cargo test -p perl-parser                                         # Parser library tests
cargo test -p perl-lsp                                           # LSP server integration tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp                       # Adaptive threading for LSP tests
cargo build -p perl-lsp --release                                # LSP server build validation
cargo build -p perl-parser --release                             # Parser library build validation
cargo bench                                                       # Performance baseline and benchmarking
cargo mutant --no-shuffle --timeout 60                           # Mutation testing
cargo fuzz run <target> -- -max_total_time=300                   # Fuzz testing
cargo audit                                                       # Security audit

# Perl LSP xtask integration
cd xtask && cargo run highlight                                   # Tree-sitter highlight integration testing
cd xtask && cargo run dev --watch                                 # Development server with hot-reload

# Quality gate validation (Perl LSP parsing performance)
cargo bench                                                       # Parsing performance validation
cargo test -p perl-parser --test comprehensive_parsing_tests     # Full parsing validation

# Fallback when preferred tools unavailable (only after gates pass)
gh pr merge <NUM> --squash --delete-branch
```

## Two Success Modes

Each agent routes with clear evidence:

1. **NEXT → target-agent** (continue microloop)
2. **FINALIZE → promotion/gate** (complete microloop)

Agents may route to themselves: "NEXT → self (attempt 2/3)" for bounded retries.

## Gate Vocabulary (uniform across flows)

**Canonical gates:** `freshness, format, clippy, spec, api, tests, build, features, mutation, fuzz, security, benchmarks, perf, docs, parsing`

**Required gates (enforced via branch protection):**
- **Integrative (PR → Merge):** `freshness, format, clippy, tests, build, security, docs, perf, parsing`
- **Hardening (Optional but recommended):** `mutation, fuzz, features, benchmarks`
- Gates must have status `pass|fail|skipped` only
- Check Run names follow pattern: `integrative:gate:<gate>` for this flow

## Gate → Agent Ownership (Integrative)

| Gate       | Primary agent(s)                                | What counts as **pass** (Check Run summary)                              | Evidence to mirror in Ledger "Gates" |
|------------|--------------------------------------------------|----------------------------------------------------------------------------|--------------------------------------|
| freshness  | rebase-checker, rebase-helper                    | PR at base HEAD (or rebase completed)                                     | `base up-to-date @<sha>` |
| format     | initial-reviewer, pr-cleanup                     | `cargo fmt --workspace --check` passes                                    | `rustfmt: all files formatted` |
| clippy     | initial-reviewer, pr-cleanup                     | `cargo clippy --workspace` passes with zero warnings                      | `clippy: 0 warnings (workspace)` |
| spec       | initial-reviewer                                 | Spec files in docs/ aligned post-rebase/cleanup (Diátaxis framework)      | `spec: aligned to docs/` |
| api        | feature-matrix-checker, pr-doc-reviewer          | API contracts consistent; breaking changes documented                      | `api: additive/none` **or** `breaking + migration docs` |
| tests      | test-runner, context-scout                       | `cargo test` passes (295+ tests including parser/lsp/lexer)               | `cargo test: 295/295 pass; parser: 180/180, lsp: 85/85, lexer: 30/30` |
| build      | feature-matrix-checker, build-validator          | `cargo build -p perl-lsp --release` and `cargo build -p perl-parser --release` succeed | `build: workspace ok; parser: ok, lsp: ok, lexer: ok` |
| mutation   | mutation-tester, test-improver                   | `cargo mutant` shows mutation score meets threshold (≥80%)                | `mutation score: <NN>%` |
| fuzz       | fuzz-tester                                      | `cargo fuzz` runs clean; no unreproduced crashers found                   | `fuzz: clean` **or** `repros added & fixed` |
| security   | safety-scanner, dep-fixer                        | `cargo audit` clean; no known vulnerabilities                             | `cargo audit: clean` |
| perf       | benchmark-runner, perf-fixer                     | `cargo bench` shows no significant regression vs baseline                 | `cargo bench: no regression` |
| docs       | pr-doc-reviewer, doc-fixer                       | Documentation complete; `cargo test --doc` passes; links valid            | `docs: complete; examples tested` |
| features   | feature-matrix-checker                           | Feature combinations build and test successfully                          | `features: compatible` |
| benchmarks | benchmark-runner                                 | Performance benchmarks complete without errors                            | `benchmarks: baseline established` |
| parsing    | pr-merge-prep                                    | Perl parsing performance meets SLO (≤1ms for incremental updates)        | `parsing: 1-150μs per file, incremental: <1ms updates; SLO: pass` **or** `skipped (N/A)` |

**Required to merge (Integrative)**: `freshness, format, clippy, tests, build, security, docs, perf, parsing` *(allow `parsing` = **skipped (N/A)** when truly N/A; see check‑run mapping below)*.

**Integrative-Specific Policies:**

**Pre-merge freshness re-check:**
`pr-merge-prep` **must** re-check `integrative:gate:freshness` on current HEAD. If stale → `rebase-helper`, then re-run a fast T1 (fmt/clippy/check) before merge.

**Parsing gate contract:**
- Command: `cargo bench` or `cargo test -p perl-parser --test comprehensive_parsing_tests`
- Evidence grammar: `parsing:<files/sec>, completion:<ms/request>, navigation:<references/sec>; SLO: ≤1ms/update => <pass|fail>`
- In the progress comment, include **parsing performance metrics** and LSP feature coverage to help future diagnosis
- When truly N/A: `integrative:gate:parsing = neutral` with `skipped (N/A: no parsing surface)`

**Bounded full matrix:**
Run the **full** matrix but **bounded** (e.g., `max_crates=8`, `max_combos=12`, or ≤8m). If exceeded → `integrative:gate:features = skipped (bounded by policy)` and list untested combos.

**Parsing delta tracking:**
Include delta vs baseline: `parsing: 1-150μs per file, completion: <100ms, navigation: 1000+ refs/sec; Δ vs baseline: +12%`

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

## Perl LSP Quality Requirements

**Parsing Performance SLO:** Perl parsing and LSP operations ≤ 1ms for incremental updates
- Bounded smoke tests with real Perl codebases for quick validation
- Report actual numbers: "Parsing: 1-150μs per file (pass)"
- Route to integrative-benchmark-runner for full validation if needed

**LSP Protocol Compliance Invariants:**
- ~89% LSP features must be functional with comprehensive workspace support
- Cross-file navigation must achieve 98% reference coverage with dual indexing
- Include LSP feature coverage metrics in Quality section

**Security Patterns:**
- Memory safety validation using cargo audit for parser libraries
- Input validation for Perl source file processing
- Proper error handling in parsing and LSP protocol implementations
- UTF-16/UTF-8 position mapping safety verification and boundary checks
- Package-specific testing validation (`perl-parser`, `perl-lsp`, `perl-lexer`)

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
**Do:** T1 validation (`cargo fmt --workspace --check`, `cargo clippy --workspace`, compilation)
**Gates:** Update `format` and `clippy` status
**Route:** Pass → `feature-matrix-checker` | Issues → `pr-cleanup`

### pr-cleanup
**Do:** Run `cargo fmt --workspace`, fix clippy warnings, resolve simple errors
**Route:** `NEXT → initial-reviewer` (re-validate)

### feature-matrix-checker
**Do:** T2 validation (`cargo build -p perl-lsp --release`, `cargo build -p perl-parser --release`, feature combinations)
**Gates:** Update `build` and `features` status
**Route:** `FINALIZE → test-runner`

### test-runner
**Do:** T3 validation (`cargo test`, `RUST_TEST_THREADS=2 cargo test -p perl-lsp`)
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
**Do:** T5 validation (`cargo bench`, Perl parsing performance regression detection)
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
**Do:** Verify branch merge-readiness, run Perl parsing performance validation, prepare linked PR for merge
**Gates:** Update `parsing` status with LSP performance validation
**Tests:** Report actual parsing performance: "Parsing: 1-150μs per file, incremental: <1ms updates; SLO: pass"
**Route:** **pr-merger** (PR ready for merge)


### pr-merger
**Do:** Execute merge to base branch (squash/rebase per repo policy)
**Labels:** Set `state:merged`
**Route:** `NEXT → pr-merge-finalizer`

### pr-merge-finalizer
**Do:** Verify merge success test, close linked issues
**Route:** **FINALIZE** (PR fully integrated)

## Perl LSP Quality Validation Details

**Parsing Performance Testing:**
- Smoke test with real Perl codebases for quick validation
- Report actual parsing times with pass/fail vs ≤1ms SLO for incremental updates
- Include LSP feature coverage and workspace navigation metrics

**LSP Server Stability:**
- Tree-sitter highlight integration must pass (4/4 tests via `cd xtask && cargo run highlight`)
- Perl parsing accuracy validation with comprehensive test coverage
- Document any changes to parsing or LSP configurations

**Security Patterns:**
- Memory safety validation using cargo audit for parser libraries
- Input validation for Perl source file processing
- Proper error handling in parsing and LSP protocol implementations
- UTF-16/UTF-8 position mapping safety and boundary checks

## Progress Heuristics

Consider "progress" when these improve:
- Validation tiers pass ↑
- Test failures ↓, mutation score ↑ (target ≥80%)
- Clippy warnings ↓, code quality ↑
- Build failures ↓, package compatibility ↑ (perl-parser, perl-lsp, perl-lexer)
- Security vulnerabilities ↓
- Performance regressions ↓
- Perl parsing performance improvements ↑ (<1ms incremental updates)
- LSP feature coverage improvements ↑ (~89% functional)

## Worktree Discipline

- **ONE writer at a time** (serialize agents that modify files)
- **Read-only parallelism** only when guaranteed safe
- **Natural iteration** with evidence of progress; orchestrator manages stopping
- **LSP production validation authority** for final integration, parsing performance, and merge readiness within this integrative flow iteration

## Success Criteria

**Complete Integration:** PR merged to main with all required gates green (`freshness, format, clippy, tests, build, security, docs, perf, parsing`), Perl LSP production quality standards met, LSP server stability validated
**Needs Rework:** PR marked needs-rework with clear prioritized action plan and specific gate failures documented

Begin with Ready PR and execute validation tiers systematically through the microloop structure, following Perl LSP's cargo-first quality standards and comprehensive LSP server testing practices.
