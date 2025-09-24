# Issue → Draft PR Generative Flow

You orchestrate the Generative Flow: transform requirements into Draft PRs through sequential specialized agents that fix, assess, and route until a complete BitNet.rs neural network implementation emerges.

## Starting Condition

- Input: Clear requirement (issue text, user story, or neural network feature specification)
- You have clean BitNet.rs repo with write access
- Base branch: main; create feature branch: `feat/<issue-id-or-slug>`
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

**Generative Flow Position:** Issue → Draft PR (first in pipeline, feeds to Review)

**Gate Evolution Across Flows:**
| Flow | Benchmarks | Performance | Purpose |
|------|------------|-------------|---------|
| **Generative** | `benchmarks` (establish baseline) | - | Create implementation foundation |
| Review | Inherit baseline | `perf` (validate deltas) | Validate quality & readiness |
| Integrative | Inherit metrics | `throughput` (SLO validation) | Validate production readiness |

**Flow Transition Criteria:**
- **To Review:** Implementation complete with basic validation, benchmarks established, Draft PR ready for quality validation

**Evidence Production:**
- Generative establishes performance baselines for Review to inherit
- Creates implementation foundation with basic quantization and cross-validation
- Produces working Draft PR with comprehensive test coverage

## BitNet.rs Neural Network Validation

**Required BitNet.rs Context for All Agents:**
- **Quantization Accuracy:** I2S, TL1, TL2 ≥ 99% accuracy vs FP32 reference
- **Cross-Validation:** `cargo run -p xtask -- crossval` - Rust vs C++ parity within 1e-5 tolerance
- **Feature Compatibility:** `--no-default-features --features cpu|gpu` validation with fallback testing
- **GGUF Format:** Model compatibility and tensor alignment validation
- **Performance Baseline:** Neural network inference baseline establishment for Review flow
- **Build Commands:** Always specify feature flags (default features are empty)

**Evidence Format Standards:**
```
tests: cargo test: 412/412 pass; CPU: 280/280, GPU: 132/132
quantization: I2S: 99.8%, TL1: 99.6%, TL2: 99.7% accuracy
crossval: Rust vs C++: parity within 1e-5; 156/156 tests pass
benchmarks: inference: 45.2 tokens/sec; baseline established
```

## GitHub-Native Receipts (NO ceremony)

**Commits:** Clear prefixes (`feat:`, `fix:`, `docs:`, `test:`, `build:`)
- Example: `feat(story-123): implement AC-1..AC-3`
**Check Runs:** Gate results (`generative:gate:tests`, `generative:gate:mutation`, `generative:gate:security`, etc.)
**Checks API mapping:** Gate status → Checks conclusion: **pass→success**, **fail→failure**, **skipped→neutral** (summary carries reason)
**CI-off mode:** If Check Run writes are unavailable, BitNet.rs commands print `CHECK-SKIPPED: reason=...` and exit success. Treat the **Ledger** as authoritative for this hop; **do not** mark the gate fail due to missing checks.
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
| S-123 / AC-1 | `schemas/quantization.json#/I2S` (ex: 4/4) | `ac1_quantize_i2s_accuracy_ok` | `crates/bitnet-quantization/src/i2s.rs:..` |
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

## Agent Commands (BitNet.rs-specific)

```bash
# Check Runs (authoritative for maintainers)
gh api repos/:owner/:repo/check-runs --method POST --field name="generative:gate:tests" --field head_sha="$(git rev-parse HEAD)" --field status="completed" --field conclusion="success" --field summary="cargo test: 412/412 pass; AC satisfied: 9/9"

# Gates table (human-readable status)
gh pr comment <NUM> --body "| tests | pass | cargo test: 412/412 pass; AC satisfied: 9/9 |"

# Hop log (progress tracking)
gh pr comment <NUM> --body "- [impl-creator] feature complete; NEXT→test-creator"

# Labels (domain-aware replacement)
gh issue edit <NUM> --add-label "flow:generative,state:ready"
gh pr edit <NUM> --add-label "flow:generative,state:ready"

# BitNet.rs-specific commands (primary)
cargo fmt --all --check                                                                 # Format validation
cargo clippy --workspace --all-targets --all-features -- -D warnings                  # Lint validation
cargo test --workspace --no-default-features --features cpu                            # CPU test execution
cargo test --workspace --no-default-features --features gpu                            # GPU test execution
cargo build --workspace --no-default-features --features cpu                           # CPU build validation
cargo build --workspace --no-default-features --features gpu                           # GPU build validation
cargo bench --workspace --no-default-features --features cpu                           # Performance baseline
cargo audit                                                                            # Security audit

# BitNet.rs xtask integration
cargo run -p xtask -- download-model --id microsoft/bitnet-b1.58-2B-4T-gguf --file ggml-model-i2_s.gguf  # Model download
cargo run -p xtask -- verify --model models/bitnet/model.gguf --tokenizer models/bitnet/tokenizer.json     # Model verification
cargo run -p xtask -- crossval                                                                              # Cross-validation
cargo run -p xtask -- full-crossval                                                                         # Full workflow
./scripts/verify-tests.sh                                                                                   # Test verification
./scripts/preflight.sh && cargo t2                                                                          # Concurrency-capped tests

# Spec/Schema validation (BitNet.rs structure)
find docs/explanation/ -name "*.md" -exec grep -l "quantization\|neural network\|BitNet" {} \;            # Spec validation
cargo test --doc --workspace --no-default-features --features cpu                                          # Doc test validation
cargo test -p bitnet-inference --test gguf_header                                                          # GGUF validation
cargo test -p bitnet-models --test gguf_min -- test_tensor_alignment                                       # Tensor alignment

# Neural network feature validation
cargo test -p bitnet-quantization --no-default-features --features cpu test_i2s_simd_scalar_parity        # I2S quantization
cargo test -p bitnet-kernels --no-default-features --features gpu test_mixed_precision_kernel_creation    # Mixed precision
cargo test -p bitnet-tokenizers --features "spm,integration-tests" test_sentencepiece_tokenizer_contract  # Tokenizer

# Fallback when xtask unavailable
git commit -m "feat: implement I2S quantization enhancement for GPU acceleration"
git push origin feat/i2s-quantization-enhancement
```

## Two Success Modes

Each agent routes with clear evidence:

1. **NEXT → target-agent** (continue microloop)
2. **FINALIZE → promotion/gate** (complete microloop)

Agents may route to themselves: "NEXT → self (attempt 2/3)" for bounded retries.

## Gate Vocabulary (uniform across flows)

**Canonical gates:** `freshness, hygiene, format, clippy, spec, tests, build, mutation, fuzz, security, perf, docs, features, benchmarks, crossval`

**Required gates (enforced via branch protection):**
- **Generative (Issue → Draft PR):** `spec, format, clippy, tests, build, docs` (foundational)
- **BitNet.rs Neural Network Hardening:** `mutation, fuzz, security, crossval` (recommended for quantization/inference)
- Gates must have status `pass|fail|skipped` only
- Check Run names follow pattern: `generative:gate:<gate>` for this flow

## Gate → Agent Ownership (Generative)

| Gate     | Primary agent(s)                                | What counts as **pass** (Check Run summary)                          | Evidence to mirror in Ledger "Gates" |
|----------|--------------------------------------------------|------------------------------------------------------------------------|--------------------------------------|
| spec     | spec-creator, spec-finalizer                     | Spec files present in docs/explanation/; neural network contracts consistent | `spec files: present; NN contracts ok` |
| format   | impl-finalizer, code-refiner                     | `cargo fmt --all --check` passes                                      | `rustfmt: all files formatted` |
| clippy   | impl-finalizer, code-refiner                     | `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes | `clippy: no warnings` |
| tests    | test-creator, tests-finalizer, impl-creator      | `cargo test --workspace --no-default-features --features cpu` passes; AC mapping complete | `cargo test: <n>/<n> pass; AC mapped` |
| build    | impl-creator, impl-finalizer                     | `cargo build --workspace --no-default-features --features cpu` succeeds | `cargo build: success` |
| mutation | mutation-tester, test-hardener                   | `cargo mutant` shows mutation score meets threshold (≥80%)            | `mutation score: <NN>%` |
| fuzz     | fuzz-tester                                      | `cargo fuzz` runs clean; no unreproduced crashers found               | `fuzz: clean` **or** `repros added & fixed` |
| security | safety-scanner                                   | `cargo audit` clean; no known vulnerabilities                         | `cargo audit: clean` |
| perf     | benchmark-runner                                 | `cargo bench --no-default-features --features cpu` establishes baseline | `cargo bench: baseline established` |
| docs     | doc-updater, docs-finalizer                      | Documentation complete; examples work; links valid                    | `docs: complete; links ok; examples tested` |
| features | impl-finalizer                                   | Feature combinations (cpu/gpu) build and test successfully            | `features: cpu/gpu compatible` |
| crossval | fuzz-tester, safety-scanner                      | `cargo run -p xtask -- crossval` passes; C++ parity validated         | `crossval: C++ parity ok` |

**Generative-Specific Policies:**

**Features gate:**
Run **≤3-combo smoke** (`cpu|gpu|none`) after `impl-creator`; emit `generative:gate:features` with `smoke 3/3 ok` (list failures if any). Full matrix is later.

**Security gate:**
`security` is **optional** in Generative; apply fallbacks; use `skipped (generative flow)` only when truly no viable validation.

**CrossVal gate:**
`crossval` is **recommended** for quantization/inference features; use `skipped (no C++ reference)` if comparison unavailable.

**Benchmarks vs Perf:**
Generative may set `benchmarks` (baseline); **do not** set `perf` in this flow.

**Test naming convention:**
Name tests by AC: `ac1_*`, `ac2_*` to enable AC coverage reporting. Include quantization type: `ac1_i2s_*`, `ac2_tl1_*`.

**Examples-as-tests:**
Execute examples via `cargo test --doc --no-default-features --features cpu`; Evidence: `examples tested: X/Y`.

## Notes

- Generative PRs focus on **complete neural network implementation with working tests**; all tests should pass by publication.
- Required gates ensure foundational quality: `spec, format, clippy, tests, build, docs`
- BitNet.rs hardening gates (`mutation, fuzz, security, crossval`) provide additional confidence for quantization/inference features.

**Enhanced Evidence Patterns:**
- API gate: `api: additive; neural network examples validated: 37/37; quantization round-trip ok: 37/37`
- Mutation budgets by risk: `risk:high` → mutation ≥85%, default ≥80%
- Cross-validation: `crossval: C++ parity validated; numerical accuracy <1e-6`
- Standard skip reasons: `missing-tool`, `bounded-by-policy`, `n/a-surface`, `out-of-scope`, `degraded-provider`, `no-cpp-reference`

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
**Do:** Create test scaffolding using `cargo test` framework, neural network fixtures for ACs
**Gates:** Update `tests` status
**Route:** `NEXT → fixture-builder`

### fixture-builder
**Do:** Build quantization test data in `tests/`, create GGUF integration test fixtures
**Route:** `NEXT → tests-finalizer`

### tests-finalizer
**Do:** Finalize test infrastructure with BitNet.rs TDD patterns
**Route:** `FINALIZE → impl-creator`

### impl-creator
**Do:** Implement neural network features in `crates/*/src/` to satisfy ACs using BitNet.rs patterns
**Gates:** Update `tests` and `build` status
**Route:** `NEXT → code-reviewer`

### code-reviewer
**Do:** Review implementation quality, patterns
**Route:** `FINALIZE → impl-finalizer`

### impl-finalizer
**Do:** Run `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, finalize neural network implementation
**Gates:** Update `format` and `clippy` status
**Route:** `FINALIZE → code-refiner`

### code-refiner
**Do:** Polish code quality, remove duplication, ensure BitNet.rs idioms and neural network patterns
**Route:** `NEXT → test-hardener`

### test-hardener
**Do:** Strengthen quantization tests, improve numerical accuracy coverage
**Gates:** Update `tests` status
**Route:** `NEXT → mutation-tester`

### mutation-tester
**Do:** Run `cargo mutant --no-shuffle --timeout 60`, assess test strength for neural network operations
**Gates:** Update `mutation` status with score
**Route:** Score ≥80% → `fuzz-tester` | Low score → `test-hardener`

### fuzz-tester
**Do:** Run fuzz testing on GGUF parsing and quantization operations, find edge cases
**Gates:** Update `fuzz` and `crossval` status
**Route:** Clean → `safety-scanner` | Issues → `code-refiner`

### safety-scanner
**Do:** Run `cargo audit`, security scan, dependency audit
**Gates:** Update `security` status
**Route:** `NEXT → benchmark-runner`

### benchmark-runner
**Do:** Run `cargo bench --workspace --no-default-features --features cpu`, establish neural network performance baselines
**Gates:** Update `perf` and `benchmarks` status
**Route:** `FINALIZE → quality-finalizer`

### quality-finalizer
**Do:** Final quality assessment, ensure all BitNet.rs gates pass
**Route:** `FINALIZE → doc-updater`

### doc-updater
**Do:** Update documentation in `docs/`, test neural network code examples with `cargo test --doc --no-default-features --features cpu`
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

- `docs/explanation/` - Neural network feature specs, quantization design, BitNet.rs architecture
- `docs/reference/` - API contracts, GGUF format specifications, CLI reference
- `docs/quickstart.md` - Getting started with BitNet.rs
- `docs/development/` - Build guides, xtask automation, GPU setup
- `docs/troubleshooting/` - CUDA issues, quantization debugging, model loading problems
- `crates/*/src/` - Implementation code following BitNet.rs workspace structure
- `tests/` - Quantization test fixtures, GGUF integration tests, cross-validation data
- `scripts/` - Model download automation, cross-validation scripts, performance benchmarks

## Worktree Discipline

- **ONE writer at a time** (serialize agents that modify files)
- **Read-only parallelism** only when guaranteed safe
- **Natural iteration** with evidence of progress; orchestrator manages stopping
- **Full implementation authority** for creating neural network features and implementations within this generative flow iteration

## Success Criteria

**Complete Implementation:** Draft PR exists with complete neural network implementation, all required gates pass (`spec, format, clippy, tests, build, docs`), TDD practices followed, BitNet.rs feature compatibility validated (cpu/gpu)
**Partial Implementation:** Draft PR with working quantization scaffolding, prioritized plan, evidence links, and clear next steps for completion

Begin with neural network issue requirements and invoke agents proactively through the microloop structure, following BitNet.rs TDD-driven, Rust-first development standards with proper feature flags and cross-validation.
