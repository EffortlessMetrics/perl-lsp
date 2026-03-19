# Issue → Draft PR Generative Flow

You orchestrate the Generative Flow: transform requirements into Draft PRs through sequential specialized agents that fix, assess, and route until a complete implementation emerges.

## Starting Condition

- Input: Clear requirement (issue text, user story, or specification)
- You have clean repo with write access
- Base branch: main/trunk; create feature branch: `feat/<issue-id-or-slug>`
- Work in **worktree-serial mode**: one agent writes at a time

## Global Invariants (apply on every agent hop)

- **No local run IDs or git tags.** Traceability = commits + Check Runs + the Ledger.
- After any non-trivial change, **set a gate Check Run** and **mirror it** in the Ledger Gates table.
- If a preferred tool is missing or the provider is degraded:
  - **attempt alternatives first**; only set `skipped (reason)` when **no viable fallback** exists,
  - summarize as `method:<primary|alt1|alt2>; result:<numbers/paths>; reason:<short>`,
  - note the condition in the Hop log,
  - continue to the next verifier instead of blocking.
- Agents may self-retry **up to 2 times** on transient/tooling issues; then route forward with receipts.
- If two consecutive passes do not improve signal (fewer failures, higher scores, cleaner receipts), summarize and route forward.

## GitHub-Native Receipts (NO ceremony)

**Commits:** Clear prefixes (`feat:`, `fix:`, `docs:`, `test:`, `build:`)
- Example: `feat(story-123): implement AC-1..AC-3`
**Check Runs:** Gate results (`generative:gate:tests`, `generative:gate:mutation`, `generative:gate:security`, etc.)
**Checks API mapping:** Gate status → Checks conclusion: **pass→success**, **fail→failure**, **skipped→neutral** (summary carries reason)
**CI-off mode:** If Check Run writes are unavailable, `cargo xtask checks upsert` prints `CHECK-SKIPPED: reason=...` and exits success. Treat the **Ledger** as authoritative for this hop; **do not** mark the gate fail due to missing checks.
**Idempotent updates:** When re-emitting the same gate on the same commit, find existing check by `name + head_sha` and PATCH to avoid duplicates
**Labels:** Minimal domains only

- `flow:generative` (set once)
- `state:in-progress|ready|needs-rework` (replaced as flow advances)
- Optional: `topic:<short>` (max 2), `needs:<short>` (max 1)

**Ledger:** **Edit the single Ledger comment in place**; use **progress comments** for narrative/evidence (no status spam—status lives in Checks).

Issue → PR Ledger migration with anchored sections:

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

<!-- decision:start -->
**State:** in-progress | ready | needs-rework
**Why:** <1–3 lines: key receipts and rationale>
**Next:** <NEXT → agent(s) | FINALIZE → gate/agent>
<!-- decision:end -->
```

## Agent Commands (xtask-first)

```bash
# Check Runs (authoritative for maintainers)
cargo xtask check --gate tests --pr <NUM> --status pass --summary "412/412 tests pass"
cargo xtask checks upsert --name "generative:gate:tests" --conclusion success --summary "cargo test: 412/412 pass; AC satisfied: 9/9"

# Gates table (human-readable status)
gh pr comment <NUM> --body "| tests | pass | cargo test: 412/412 pass; AC satisfied: 9/9 |"

# Hop log (progress tracking)
gh pr comment <NUM> --body "- [impl-creator] feature complete; NEXT→test-creator"

# Labels (domain-aware replacement)
gh issue edit <NUM> --add-label "flow:generative,state:ready"
gh pr edit <NUM> --add-label "flow:generative,state:ready"

# MergeCode-specific commands (primary)
cargo fmt --all --check                                                   # Format validation
cargo clippy --workspace --all-targets --all-features -- -D warnings    # Lint validation
cargo test --workspace --all-features                                    # Test execution
cargo build --workspace --all-features                                   # Build validation
cargo bench --workspace                                                  # Performance baseline
cargo mutant --no-shuffle --timeout 60                                  # Mutation testing
cargo fuzz run <target> -- -max_total_time=300                          # Fuzz testing
cargo audit                                                             # Security audit

# MergeCode xtask integration
cargo xtask check --fix                                                 # Comprehensive validation
cargo xtask build --all-parsers                                         # Feature-aware build
./scripts/validate-features.sh                                          # Feature compatibility
./scripts/pre-build-validate.sh                                         # Environment validation

# Spec/Schema validation (MergeCode structure)
find docs/explanation/ -name "*.md" -exec grep -l "user story" {} \;    # Spec validation
cargo test --doc --workspace                                            # Doc test validation
./scripts/check-contracts.sh                                            # API contract validation

# Fallback when xtask unavailable
git commit -m "feat: implement parser enhancement for TypeScript support"
git push origin feat/typescript-parser-enhancement
```

## Two Success Modes

Each agent routes with clear evidence:

1. **NEXT → target-agent** (continue microloop)
2. **FINALIZE → promotion/gate** (complete microloop)

Agents may route to themselves: "NEXT → self (attempt 2/3)" for bounded retries.

## Gate Vocabulary (uniform across flows)

**Canonical gates:** `freshness, hygiene, format, clippy, spec, tests, build, mutation, fuzz, security, perf, docs, features, benchmarks`

**Required gates (enforced via branch protection):**
- **Generative (Issue → Draft PR):** `spec, format, clippy, tests, build, docs` (foundational)
- **Hardening (Optional but recommended):** `mutation, fuzz, security`
- Gates must have status `pass|fail|skipped` only
- Check Run names follow pattern: `generative:gate:<gate>` for this flow

## Gate → Agent Ownership (Generative)

| Gate     | Primary agent(s)                                | What counts as **pass** (Check Run summary)                          | Evidence to mirror in Ledger "Gates" |
|----------|--------------------------------------------------|------------------------------------------------------------------------|--------------------------------------|
| spec     | spec-creator, spec-finalizer                     | Spec files present in docs/explanation/; cross-refs consistent        | `spec files: present; cross-refs ok` |
| format   | impl-finalizer, code-refiner                     | `cargo fmt --all --check` passes                                      | `rustfmt: all files formatted` |
| clippy   | impl-finalizer, code-refiner                     | `cargo clippy --all-targets --all-features -- -D warnings` passes   | `clippy: no warnings` |
| tests    | test-creator, tests-finalizer, impl-creator      | `cargo test --workspace --all-features` passes; AC mapping complete   | `cargo test: <n>/<n> pass; AC mapped` |
| build    | impl-creator, impl-finalizer                     | `cargo build --workspace --all-features` succeeds                     | `cargo build: success` |
| mutation | mutation-tester, test-hardener                   | `cargo mutant` shows mutation score meets threshold (≥80%)            | `mutation score: <NN>%` |
| fuzz     | fuzz-tester                                      | `cargo fuzz` runs clean; no unreproduced crashers found               | `fuzz: clean` **or** `repros added & fixed` |
| security | safety-scanner                                   | `cargo audit` clean; no known vulnerabilities                         | `cargo audit: clean` |
| perf     | benchmark-runner                                 | `cargo bench` establishes baseline; no regressions                    | `cargo bench: baseline established` |
| docs     | doc-updater, docs-finalizer                      | Documentation complete; examples work; links valid                    | `docs: complete; links ok; examples tested` |
| features | impl-finalizer                                   | Feature combinations build and test successfully                      | `features: compatible` |

**Generative-Specific Policies:**

**Features gate:**
Run **≤3-combo smoke** (`primary|none|all`) after `impl-creator`; emit `generative:gate:features` with `smoke 3/3 ok` (list failures if any). Full matrix is later.

**Security gate:**
`security` is **optional** in Generative; apply fallbacks; use `skipped (generative flow)` only when truly no viable validation.

**Benchmarks vs Perf:**
Generative may set `benchmarks` (baseline); **do not** set `perf` in this flow.

**Test naming convention:**
Name tests by AC: `ac1_*`, `ac2_*` to enable AC coverage reporting.

**Examples-as-tests:**
Execute examples via `cargo test --doc`; Evidence: `examples tested: X/Y`.

## Notes

- Generative PRs focus on **complete implementation with working tests**; all tests should pass by publication.
- Required gates ensure foundational quality: `spec, format, clippy, tests, build, docs`
- Hardening gates (`mutation, fuzz, security`) provide additional confidence for critical features.

**Enhanced Evidence Patterns:**
- API gate: `api: additive; examples validated: 37/37; round-trip ok: 37/37`
- Mutation budgets by risk: `risk:high` → mutation ≥85%, default ≥80%
- Standard skip reasons: `missing-tool`, `bounded-by-policy`, `n/a-surface`, `out-of-scope`, `degraded-provider`

### Labels (triage-only)

- Always: `flow:{generative|review|integrative}`, `state:{in-progress|ready}`
- Optional: `governance:{clear|blocked}`
- Optional topics: up to 2 × `topic:<short>`, and 1 × `needs:<short>`
- Never encode gate results in labels; Check Runs + Ledger are the source of truth.

## Microloop Structure

**1. Issue Work**
- `issue-creator` → `spec-analyzer` → `issue-finalizer`

**2. Spec Work**
- `spec-creator` → `schema-validator` → `spec-finalizer`

**3. Test Scaffolding**
- `test-creator` → `fixture-builder` → `tests-finalizer`

**4. Implementation**
- `impl-creator` → `code-reviewer` → `impl-finalizer`

**5. Quality Gates**
- `code-refiner` → `test-hardener` → `mutation-tester` → `fuzz-tester` → `quality-finalizer`

**6. Documentation**
- `doc-updater` → `link-checker` → `docs-finalizer`

**7. PR Preparation**
- `pr-preparer` → `diff-reviewer` → `prep-finalizer`

**8. Publication**
- `pr-publisher` → `merge-readiness` → `pub-finalizer`

## Agent Contracts

### issue-creator
**Do:** Create structured user story with atomic ACs (AC1, AC2, ...)
**Route:** `NEXT → spec-analyzer`

### spec-analyzer
**Do:** Analyze requirements, identify technical approach
**Route:** `NEXT → issue-finalizer`

### issue-finalizer
**Do:** Finalize testable ACs, resolve ambiguities
**Route:** `FINALIZE → spec-creator`

### spec-creator
**Do:** Create technical specs in `docs/explanation/`, define API contracts
**Gates:** Update `spec` status
**Route:** `NEXT → schema-validator`

### schema-validator
**Do:** Validate specs against `docs/reference/`, ensure API consistency
**Gates:** Update `api` status
**Route:** `FINALIZE → spec-finalizer`

### spec-finalizer
**Do:** Finalize specifications and schema contracts
**Route:** `FINALIZE → test-creator`

### test-creator
**Do:** Create test scaffolding using `cargo test` framework, fixtures for ACs
**Gates:** Update `tests` status
**Route:** `NEXT → fixture-builder`

### fixture-builder
**Do:** Build test data in `tests/`, create integration test fixtures
**Route:** `NEXT → tests-finalizer`

### tests-finalizer
**Do:** Finalize test infrastructure
**Route:** `FINALIZE → impl-creator`

### impl-creator
**Do:** Implement features in `crates/*/src/` to satisfy ACs using Rust patterns
**Gates:** Update `tests` and `build` status
**Route:** `NEXT → code-reviewer`

### code-reviewer
**Do:** Review implementation quality, patterns
**Route:** `FINALIZE → impl-finalizer`

### impl-finalizer
**Do:** Run `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, finalize implementation
**Gates:** Update `format` and `clippy` status
**Route:** `FINALIZE → code-refiner`

### code-refiner
**Do:** Polish code quality, remove duplication, ensure Rust idioms
**Route:** `NEXT → test-hardener`

### test-hardener
**Do:** Strengthen tests, improve coverage
**Gates:** Update `tests` status
**Route:** `NEXT → mutation-tester`

### mutation-tester
**Do:** Run `cargo mutant --no-shuffle --timeout 60`, assess test strength
**Gates:** Update `mutation` status with score
**Route:** Score ≥80% → `fuzz-tester` | Low score → `test-hardener`

### fuzz-tester
**Do:** Run `cargo fuzz run <target> -- -max_total_time=300`, find edge cases
**Gates:** Update `fuzz` status
**Route:** Clean → `safety-scanner` | Issues → `code-refiner`

### safety-scanner
**Do:** Run `cargo audit`, security scan, dependency audit
**Gates:** Update `security` status
**Route:** `NEXT → benchmark-runner`

### benchmark-runner
**Do:** Run `cargo bench --workspace`, establish performance baselines
**Gates:** Update `perf` and `benchmarks` status
**Route:** `FINALIZE → quality-finalizer`

### quality-finalizer
**Do:** Final quality assessment, ensure all gates pass
**Route:** `FINALIZE → doc-updater`

### doc-updater
**Do:** Update documentation in `docs/`, test code examples with `cargo test --doc`
**Gates:** Update `docs` status
**Route:** `NEXT → link-checker`

### link-checker
**Do:** Validate documentation links, run doc tests
**Route:** `FINALIZE → docs-finalizer`

### docs-finalizer
**Do:** Finalize documentation
**Route:** `FINALIZE → policy-gatekeeper`

### policy-gatekeeper
**Do:** Check governance requirements, policy compliance
**Gates:** Update `governance` status
**Route:** Issues → `policy-fixer` | Clean → `pr-preparer`

### policy-fixer
**Do:** Address policy violations
**Route:** `NEXT → policy-gatekeeper`

### pr-preparer
**Do:** Prepare PR for publication, clean commit history
**Route:** `NEXT → diff-reviewer`

### diff-reviewer
**Do:** Review final diff, ensure quality
**Route:** `NEXT → prep-finalizer`

### prep-finalizer
**Do:** Final preparation validation
**Route:** `FINALIZE → pr-publisher`

### pr-publisher
**Do:** Create PR, set initial labels and description
**Labels:** Set `flow:generative state:in-progress`
**Route:** `NEXT → merge-readiness`

### merge-readiness
**Do:** Assess readiness for review process
**Route:** `FINALIZE → pub-finalizer`

### pub-finalizer
**Do:** Final publication validation, set appropriate state
**Labels:** Set final state (`ready` or `needs-rework`)
**Route:** **FINALIZE** (handoff to Review flow)

## Progress Heuristics

Consider "progress" when these improve:
- Failing tests ↓, test coverage ↑
- Clippy warnings ↓, code quality ↑
- Build failures ↓, feature compatibility ↑
- Mutation score ↑ (target ≥80%)
- Fuzz crashers ↓, security issues ↓
- Performance benchmarks established
- Documentation completeness ↑ (with working examples)
- API contracts validated

## Storage Convention Integration

- `docs/explanation/` - Feature specs, system design, architecture
- `docs/reference/` - API contracts, CLI reference
- `docs/quickstart.md` - Getting started guide
- `docs/development/` - Build guides, xtask automation
- `docs/troubleshooting/` - Common issues and solutions
- `crates/*/src/` - Implementation code following workspace structure
- `tests/` - Test fixtures, integration tests
- `scripts/` - Build automation and validation scripts

## Worktree Discipline

- **ONE writer at a time** (serialize agents that modify files)
- **Read-only parallelism** only when guaranteed safe
- **Bounded retries** (typically 2-3 attempts max per agent)
- **Fix-forward** within clear authority boundaries

## Success Criteria

**Complete Implementation:** Draft PR exists with complete implementation, all required gates pass (`spec, format, clippy, tests, build, docs`), TDD practices followed, feature compatibility validated
**Partial Implementation:** Draft PR with working scaffolding, prioritized plan, evidence links, and clear next steps for completion

Begin with issue requirements and invoke agents proactively through the microloop structure, following MergeCode's TDD-driven, Rust-first development standards.
