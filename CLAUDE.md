# CLAUDE.md

**Latest Release**: 0.15.0 | **Metrics**: [status/index.md](docs/project/status/index.md) | **API Stability**: [STABILITY.md](docs/reference/STABILITY.md) | **Implementation agents**: [AGENTS.md](AGENTS.md)

## Orchestration Model

perl-lsp's orchestration is an *Octopus Cluster* — see [docs/reference/OCTOPUS_CLUSTER.md](docs/reference/OCTOPUS_CLUSTER.md) for the umbrella framing.

> For the design rationale and direction behind this orchestration model, see [docs/reference/ORCHESTRATION_DOCTRINE.md](docs/reference/ORCHESTRATION_DOCTRINE.md).

The orchestrator routes work to agents, never writes code directly.

### Gates and Agents

The pipeline is organized into **7 gates** (coarse stages) with multiple agents working within each gate:

| Gate | Purpose | Key agents |
|------|---------|-----------|
| **1. Identify** | Accurate, builder-ready problem statement | scout, accuracy-scout, research-verifier |
| **2. Spec** | Scoped, project-aligned approach | plan-reviewer, oppositional-planner, advocatus-diaboli, architecture-reviewer, maintainer-issue, spec-planner |
| **3. Build** | Well-tested, implemented PR | red-tdd, builder, green-tdd |
| **4. Review/improve** | Right thing × what codebase needs × right way | reviewer, maintainer-pr, refactor-planner, green-refactor, reviewer-deep, diff-auditor |
| **5. CI green** | Live CI actually green (not just a label) | green-ci, pr-responder |
| **6. Merge** | Land it | ops |
| **7. Learn** | Consolidate captured learning into durable artifacts | wisdom, memory-recalibrator |

**Sequencing within a gate** is preferred when agents build on each other's output, but is not strict — parallel agents within a gate are fine when they don't depend on each other.

**Some gates may be skipped** when they are not relevant for a given PR's nature (e.g., a 1-line fmt fix skips Gates 1 and 2; a docs-only PR skips reviewer-deep in Gate 4).

**Learning is captured continuously** by every agent in every gate. Gate 7 is the dedicated consolidation layer — it shapes captured artifacts into durable memory, doctrine, and follow-up work.

See [docs/reference/PIPELINE_GATES.md](docs/reference/PIPELINE_GATES.md) for the full gate model: skip criteria, within-gate ordering, three-axis triangulation in Gate 4, and worked examples.

### Pipeline: Scout → Accuracy-Scout → Plan-Review → Build → Review → Green → Merge → Wisdom

The default sequence within and across gates. Adapt to PR nature; skip gates that don't apply.

Every change flows through this pipeline. Each stage is a cheap pass that catches what the previous one missed.

| Stage | Model | Purpose | Fix forward? |
|-------|-------|---------|-------------|
| **Scout** (haiku) | Broad discovery | Find the problem, file roughly-right spec | N/A — files issues |
| **Accuracy-scout** (haiku) | Mechanical fact check | Verify file paths, function names, issue status against master | No — corrects facts, not plans |
| **Research-verifier** (haiku) | External fact check | Verify Perl semantics, LSP spec, crate API claims via web + grep | No — verifies facts, not plans |
| **Oppositional-planner** (haiku) | Challenge approach | Surface objections, overlooked alternatives, risk flags | No — generates challenges for plan-reviewer |
| **Advocatus-diaboli** (haiku) | Challenge premise | Should this exist at all? User impact, yak-shaving, scope fit | No — BUILD/DEFER/CLOSE verdict |
| **Architecture-reviewer** (haiku) | Structural alignment | Verify design fits microcrate layering, dependency direction, type placement | No — flags violations |
| **Maintainer-issue** (haiku) | Project vision (issue) | Does this align with perl-lsp's goals, roadmap, and user base? | No — ALIGNED/DEFERRED/OUT OF SCOPE |
| **Plan-review** (sonnet) | Improve the plan | Fill gaps, correct root cause, add edge cases | Yes — complete the spec yourself |
| **Spec-planner** (haiku) | Implementation roadmap | Create `impl/` branch, write `.spec/` files (checklist, acceptance, context) | No — plans, doesn't implement |
| **Red-TDD** (haiku) | Write failing tests | Commit red tests to impl branch; define "done" before builder starts | No — tests only, no implementation |
| **Build** (sonnet) | Make tests green | Check out impl branch (spec + red tests), implement, verify, PR | Yes — adapt if plan-reviewed; bump back if not |
| **Green-TDD** (haiku) | Harden tests | Add edge case, boundary, regression tests after builder implements | No — tests only, flags bugs for reviewer |
| **Review** (haiku) | Standards check | Banned patterns, scope, formatting — push fixes directly | Yes — always fix forward |
| **Maintainer-PR** (haiku) | Project vision (PR) | Does the implementation fit perl-lsp's direction and quality bar? | No — ALIGNED/SCOPE DRIFT/QUALITY GAP |
| **PR-responder** (haiku) | Address bot comments | Fix CI failures, validate-title, linter warnings, resolve conversations | Yes — fix what's broken |
| **Refactor-planner** (haiku) | Refactor analysis | Identify simplification, reuse, dead code, type tightness — posts plan for green-refactor | No — analysis only |
| **Green-refactor** (sonnet) | Refactor while green | Execute refactor plan: simplify, extract helpers, improve naming — tests stay green | Yes — behavior-preserving only |
| **Review-deep** (sonnet) | Correctness check | Does the logic work? Edge cases? Regressions? | Yes — fix forward, final gate |
| **Green-CI** (haiku) | CI freshness gate | Verify all checks pass on current HEAD SHA — no stale green | Yes — fixes mechanical CI failures |
| **Diff-auditor** (haiku) | Final diff check | Verify cumulative diff is coherent, clean, matches spec, no artifacts | No — CLEAN/ARTIFACTS/REGRESSION/DRIFT |
| **Green** | CI gate | SHA-verified, merge-time fresh check | N/A |
| **Merge** | Ops | Batch of 3, wait for green, ratchet corpus | N/A |
| **Wisdom** | Learning | Retrospective, update memory, log patterns | N/A |

**Key principles:**
- The orchestrator routes, it doesn't execute. Never poll CI, read diffs, or check PR state in loops. Launch an agent with the full job and move to the next routing decision.
- One status check to inform routing, then delegate. When the orchestrator has context (exact edits, file contents), pass it to the agent — don't make agents re-research what you already know.
- Scouts are honest about uncertainty — plan-reviewers correct. Being roughly right > confidently wrong.
- Accuracy-scouts verify mechanical facts only (file paths, function names, issue status). They do not redesign the spec or suggest approaches.
- Plan-reviewers improve plans, never punt "needs more scout work." They're enhanced scouts with sonnet.
- Builders execute the spec as given. Fix forward on small gaps, bump back if structural.
- Reviewers push improvements directly to PR branches. Every PR gets improved, no LGTM-only.
- Every agent recommends next steps for the orchestrator.
- Learning is continuous — every agent-wrapup captures what was learned.
- **Master must stay green; merge requires green** (2026-04-26 directive). Per-crate green is necessary but not sufficient — workspace-wide xtask fmt and clippy cascades break master if a single PR's drift goes unchecked. Verify workspace-wide CI before merging; route to fmt/clippy fix if not.
- **Each agent's pass produces ONE routing decision.** Sign-off is itself one of the routing options — applied across ALL agents (reviewer, maintainer-pr, refactor-planner, green-tdd, deep-reviewer, diff-auditor, green-ci, accuracy-scout, research-verifier, oppositional-planner, advocatus-diaboli, architecture-reviewer, maintainer-issue, spec-test-code-match). Each pass picks exactly one of: (a) sign off (gate clean, apply `<gate>-reviewed`) OR (b) bounce back (apply the appropriate `needs-*` routing label). Never both. Per the 2026-04-26 #6780 incident: applying `review-reviewed` AND `needs-builder-fix` simultaneously confused the merge gate and let unfixed bugs ride to master. The principle is one-decision-per-pass: gate-clean OR bounce, not gate-clean AND bounce.
- **No `needs-*` label on a PR may merge.** Even with `merge-ready`, presence of any `needs-builder-fix` / `needs-ci-fix` / `needs-diff-fix` / `needs-spec-fix` / `needs-red-tdd-fix` label MUST block ops merge. The presence of an active routing label means the PR has unaddressed work.
- **External-source PRs (claude-burst, codex-burst, diffguard-bot, etc.) require the same gate set as internal PRs.** Don't shortcut review on third-party PRs; they're frequently the source of cross-PR contamination, hallucinated APIs, and scope drift between title and diff.

### Pipeline State Labels

Labels are the authoritative state for every issue and PR. The orchestrator reads them; agents write them. For the principle distinguishing live-truth labels (CI, mergeability) from authoritative-only labels (signoffs, routing), see [docs/reference/LIVE_SIGNALS_VS_LABELS.md](docs/reference/LIVE_SIGNALS_VS_LABELS.md).

**Sign-off labels** (`<agent>-reviewed` = agent completed its pass):

| Label | Set by | Means |
|-------|--------|-------|
| `accuracy-reviewed` | accuracy-scout | Mechanical facts verified (file paths, function names) |
| `research-reviewed` | research-verifier | External claims verified (Perl docs, LSP spec, crate APIs) |
| `oppositional-reviewed` | oppositional-planner | Approach challenged, alternatives surfaced |
| `diaboli-reviewed` | advocatus-diaboli | Existence challenged — BUILD/DEFER/CLOSE verdict |
| `plan-reviewed` | plan-reviewer | Spec refined and approved |
| `spec-reviewed` | spec-planner | Impl branch created with `.spec/` files |
| `red-tdd-reviewed` | red-tdd | Failing tests committed on impl branch |
| `green-tdd-reviewed` | green-tdd | Edge case and regression tests added |
| `architecture-reviewed` | architecture-reviewer | Design fits microcrate layering and dependency contracts |
| `maintainer-issue-reviewed` | maintainer-issue | Issue aligns with project goals, roadmap, user base |
| `green-tdd-reviewed` | green-tdd | Edge case and regression tests added |
| `review-reviewed` | reviewer | Standards check passed (banned patterns, scope) |
| `maintainer-pr-reviewed` | maintainer-pr | PR implementation fits project direction and quality bar |
| `pr-responded` | pr-responder | Bot comments and CI failures addressed |
| `refactor-planner-reviewed` | refactor-planner | Simplification/reuse plan posted for green-refactor |
| `green-refactor-reviewed` | green-refactor | Implementation simplified while tests stay green |
| `deep-reviewed` | reviewer-deep | Correctness check passed — required before merge |
| `ci-green` | green-ci | All CI checks pass on current HEAD SHA |
| `diff-audited` | diff-auditor | Cumulative diff is coherent, clean, matches spec — ready for ops |

**State labels** (where the issue/PR is now):

| Label | Set by | Means |
|-------|--------|-------|
| `builder-ready` | plan-reviewer | Spec finalized — ready for build pipeline |
| `in-build` | builder | Builder actively working |
| `in-review` | reviewer | PR in review process |
| `merge-ready` | pr-ready | All gates passed — ready for ops merge |
| `already-fixed` | any agent | Close without build |

**Routing labels** (`needs-<action>` = work needed):

| Label | Set by | Means |
|-------|--------|-------|
| `needs-plan-review` | scout | Entry to verification pipeline |
| `needs-deep-review` | reviewer | Standards done, deep review needed |
| `needs-builder-fix` | green-tdd | Edge case test found bug — route back to builder |
| `needs-ci-fix` | green-ci | CI check failed or stale — route to pr-responder |
| `needs-diff-fix` | diff-auditor | Diff has artifacts, regressions, or scope drift — route to pr-responder |

**Meta labels:**

| Label | Purpose |
|-------|---------|
| `structural-blocker` | Blocks parallel work |
| `follow-up-recommended` | Needs follow-up issue |
| `swarm-discovered` | Found by automated sweep |
| `size/S`, `size/M`, `size/L` | Effort estimate |

Labels are sign-off receipts. The *presence* of a label means an agent reviewed and approved. The *absence* means the pass hasn't happened yet. The orchestrator routes based on what's missing.

### Label-based routing

Default routing pattern. The orchestrator may skip individual queries when the PR's nature makes that gate's check trivially satisfied or irrelevant. See [docs/reference/PIPELINE_GATES.md](docs/reference/PIPELINE_GATES.md) for skip criteria.

**Pre-plan-review verification** (issue has `needs-plan-review`):
```
Missing accuracy-reviewed?          → spawn accuracy-scout (first — corrects line numbers for everyone else)
Missing research-reviewed?          → spawn research-verifier (reads accuracy-corrected facts)
Missing oppositional-reviewed?      → spawn oppositional-planner (reads verified facts to challenge approach)
Missing diaboli-reviewed?           → spawn advocatus-diaboli (reads verified + challenged spec to judge existence)
Missing architecture-reviewed?      → spawn architecture-reviewer (reads verified spec to check structural fit)
Missing maintainer-issue-reviewed?  → spawn maintainer-issue (reads all above to judge project alignment)
All six present?                    → spawn plan-reviewer
```
These are **sequential** — each layer reads and builds on the previous. The accuracy-scout
corrects facts, the research-verifier works from corrected facts, the oppositional-planner
challenges a verified approach, the architecture-reviewer checks structure of a challenged
spec, the maintainer checks project fit of an architecturally-validated proposal. Running
them out of order wastes tokens and produces worse results.

**Pre-build preparation** (issue has `builder-ready`):
```
Missing spec-reviewed?          → spawn spec-planner
Missing red-tdd-reviewed?       → spawn red-tdd (after spec-planner)
Both present?                  → spawn builder
```
These are sequential — red-tdd needs the branch from spec-planner.

**Post-build hardening** (issue has `in-build`, PR exists):
```
PR created?                        → spawn green-tdd on the PR branch
green-tdd done?                    → spawn reviewer
needs-builder-fix set?             → route back to builder first
```

**Post-build PR pipeline** (PR exists, sequential):
```
Missing green-tdd-reviewed?        → spawn green-tdd (add edge case tests)
Missing review-reviewed?           → spawn reviewer (standards check, pushes fixes)
Missing maintainer-pr-reviewed?    → spawn maintainer-pr (project fit check)
Missing pr-responded?              → spawn pr-responder (address bot comments, CI failures)
Missing refactor-planner-reviewed?  → spawn refactor-planner (haiku analysis of simplification opportunities)
Missing green-refactor-reviewed?   → spawn green-refactor (execute refactor plan — sonnet)
All six present?                   → spawn reviewer-deep (final correctness gate)
Missing ci-green? (after deep)     → spawn green-ci (verify CI on current HEAD, fix mechanical failures)
needs-ci-fix set?                  → route back to pr-responder
Missing diff-audited? (after CI)   → spawn diff-auditor (final coherence check)
needs-diff-fix set?                → route back to pr-responder
diff-audited + ci-green present?   → spawn ops (merge)
```
Each reads the previous agents' comments. The pr-responder fixes bot comments
and CI failures. Green-ci is the final mechanical gate — verifies CI is
genuinely green on the current HEAD SHA, not a stale result from an earlier push.

**Query examples:**
```bash
gh issue list --label "needs-plan-review" -l "accuracy-reviewed" -l "research-reviewed" -l "oppositional-reviewed" -l "diaboli-reviewed" --state open  # fully verified, ready for plan-review
gh issue list --label "builder-ready" --state open   # ready to build
gh issue list --label "red-tdd-reviewed" --state open # red tests done, builder can start
gh issue list --label "in-build" --state open        # builder working
gh issue list --label "needs-builder-fix" --state open  # green-tdd found bug
gh issue list --label "structural-blocker" --state open  # blocked work
gh pr list --search "label:merge-ready"              # ready to merge
```

### Routing patterns

- **Code change** -> worktree agent: `Agent(isolation: "worktree", prompt: "...")`
- **Research** -> explore agent: `Agent(subagent_type: "Explore", prompt: "...")`
- **Multiple changes** -> parallel worktree agents, one per crate. Microcrate architecture prevents conflicts.
  - Reserve 10 agent slots for late-cycle routing. Use SendMessage to repurpose idle agents.

### Merge Queue Protocol

- Don't rebase PRs unless merge conflicts exist
- Merge in batches of 3 (CI cancellation cascade -- rapid merges cancel each other's CI runs)
- Run `just cpan-corpus-ratchet` after parser fix merges
- `docs/project/status/*.md` subsystem files are regenerated automatically post-merge (no manual step needed)

## Quick Reference

```bash
just doctor                           # Workspace health check (run before any agent-spawning session)
just pr-fast                          # Canonical fast push guard
nix develop -c just ci-gate           # Canonical local merge gate (before merge)
cargo build -p perl-lsp-rs --release     # Build LSP server
cargo test --workspace --lib          # Run all tests
```

| Task | Pattern |
|------|---------|
| Code change | `Agent(isolation: "worktree", ...)` |
| Research | `Agent(subagent_type: "Explore", ...)` |
| Parser fix | `/parser-fix` |
| Swarm cycle | `/swarm all` |
| Crate verification | `/verify <crate>` |

## Crate Structure

134 workspace members across 135 crate directories (see `cargo metadata --no-deps`). Key crates:

| Crate | Path | Purpose |
|-------|------|---------|
| **perl-parser** | `crates/perl-parser/` | Main parser (v3 recursive descent) |
| **perl-lsp** | `crates/perl-lsp-rs/` | LSP server binary |
| **perl-dap** | `crates/perl-dap/` | Debug Adapter Protocol |
| **perl-lexer** | `crates/perl-lexer/` | Context-aware tokenizer |
| **perl-parser-core** | `crates/perl-parser-core/` | Core parsing infrastructure |
| **perl-workspace** | `crates/perl-workspace/` | Workspace symbol indexing |
| **perl-semantic-analyzer** | `crates/perl-semantic-analyzer/` | Semantic analysis |

Families: `perl-module-*` (module resolution), `perl-lsp-*` (LSP providers), `perl-lsp-feature-*` (feature governance), `perl-dap-*` (DAP), `perl-ts-*` (tree-sitter), `perl-workspace-*` (workspace discovery), core leaf crates (token, AST, quote, regex, heredoc, error).

## Essential Commands

### Build & Test

```bash
cargo build -p perl-lsp-rs --release     # LSP server
cargo build -p perl-parser --release  # Parser library
cargo test                            # All tests
cargo test -p perl-parser             # Parser tests
cargo test -p perl-lsp-rs                # LSP tests
cargo test -p perl-parser -- test_name --exact  # Exact test in crate
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2  # LSP threading
just ci-lsp-def                       # Semantic definition tests
```

### Lint, Format, Quality

```bash
cargo xtask fmt                       # Format code (per-crate, Windows-safe)
cargo clippy --workspace              # Lint all crates
cargo clippy --workspace --lib        # Lint libraries only (faster)
just dead-code                        # Dead code report
cargo machete                         # Unused dependencies
just security-audit                   # Security audit
just semver-check                     # SemVer check all published packages
```

### Benchmarks, Fuzzing, Coverage

```bash
just benchmarks                       # Run all benchmarks
just fuzz-bounded                     # Bounded fuzz run (60s per target)
just mutation-subset                  # Mutation testing subset
just coverage                         # HTML coverage report
just coverage-lcov                    # lcov.info for CI
```

### Health & Status

```bash
just health                           # Codebase metrics
just status-check                     # Verify computed metrics
just debt-report                      # Technical debt status
just debt-check                       # Debt budget compliance
```

### CPAN Corpus

```bash
just cpan-corpus-sweep                # Sweep and report
just cpan-corpus-check                # Enforce manifest (fails on regression)
just cpan-corpus-ratchet              # Auto-add clean modules to manifest
```

## Development Workflow

**Local-first** -- all gates run locally before CI. Install hook: `bash scripts/install-githooks.sh`

### CI Gate Tiers

| Tier | Command | Time | When |
|------|---------|------|------|
| **A (PR-fast)** | `just pr-fast` | ~1-2 min | Quick iteration and pre-push hook |
| **B (Merge gate)** | `just ci-gate` | ~3-5 min | Before merge |
| **C (Nightly)** | `just ci-full` | ~15-30 min | Mutation, fuzzing, benchmarks |

## Parser Versions

- **v3 (Native)**: Current recursive descent parser
- **v2 (Pest)**: Legacy, kept out of default gate
- **v1 (C-based)**: Benchmarking only

## Workspace Exclusions

`tree-sitter-perl/` (legacy C), `fuzz/` (fuzz builds), `archive/` (archived).

## Key Paths

| What | Where |
|------|-------|
| Parser source | `crates/perl-parser/src/` |
| LSP providers | `crates/perl-lsp-*/src/` |
| LSP server binary | `crates/perl-lsp-rs/src/` |
| DAP server | `crates/perl-dap/src/` |
| Tests | `crates/*/tests/` |
| Test corpus | `test_corpus/`, `tree-sitter-perl/test/corpus/` |
| VSCode extension | `vscode-extension/` |
| Documentation | `docs/` |
| Features catalog | `features.toml` |
| CI config | `.ci/` |
| Known blockers | `.ci/blockers.yaml` |
| Build tooling | `xtask/` |
| Slash commands | `.claude/commands/` |
| Swarm ops | `.ops-perl-lsp/` |

## Architecture Patterns

**Dual indexing**: Index workspace symbols under both qualified and bare names (see PR #122).

**LSP threading**: `RUST_TEST_THREADS=2`, `CARGO_BUILD_JOBS=1`, `RUSTC_WRAPPER=""`.

**Worktree stash prohibition**: Never use `git stash` in a worktree agent. The stash list is shared across all worktrees and the main checkout — `git stash pop` may silently restore another agent's changes. Use `git restore <file>` to discard changes, or `git commit -m "wip"` to save work in progress.

## Truth Sources

Metrics are **computed, not hand-edited**:
- `docs/project/status/*.md` subsystem files auto-generated via `just status-update` (writes lsp.md, tests.md, parser.md, quality.md)
- `docs/project/CURRENT_STATUS.md` is now a stable stub linking to the subsystem files (no `<!-- BEGIN: -->` markers)
- `features.toml` is the canonical LSP capability definition
- Test output and CI receipts are evidence for all claims
- `README.md` must not contain volatile metrics -- link to `docs/project/status/index.md`
- `.ci/blockers.yaml` is manually maintained — verify counts against `parser-corpus-baseline.json` before trusting `affected_files` values

## Coding Standards

Invoke `/coding-standards` for full detail.

- Run `cargo fmt` and `cargo clippy --workspace` before committing
- **Banned in production code**: `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `std::process::abort()`, `dbg!()`
  - Use `?`, `.ok_or_else()`, pattern matching, `Result`/`Option` instead
  - `std::process::exit()` only in `bin/` and `lifecycle.rs`
  - Exception: `#[allow(clippy::expect_used)]` in `crates/perl-lsp-rs/src/util/uri.rs`
  - Exception: `bin/` targets may use `#[allow(clippy::expect_used)]` for profiling / CLI entry points, including `crates/perl-workspace/src/bin/workspace_memory_profile.rs`
  - Exception: static `LazyLock<Regex>` initializers may use `unreachable!()`/`expect()` for known-good patterns, including `crates/perl-heredoc-anti-patterns/src/lib.rs`
  - Tests: `Result<()>` returns or `perl_tdd_support::must`/`must_some`
- **Prefer**: `.first()` over `.get(0)`, `.push(char)` over `.push_str("x")`, `or_default()` over `or_insert_with(Vec::new)`
- **Avoid**: unnecessary `.clone()` on Copy types
- **Regex**: `Option<Regex>` with `.ok()` for graceful degradation
- After adding tests, no manual status update needed — `docs/project/status/*.md` files are auto-regenerated post-merge

## Documentation

[Status Overview](docs/project/status/index.md) | [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) (stub) | [ROADMAP.md](docs/project/ROADMAP.md) | [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) | [LSP_IMPLEMENTATION_GUIDE.md](docs/reference/LSP_IMPLEMENTATION_GUIDE.md) | [FAILURE_MODES.md](docs/reference/FAILURE_MODES.md) | [CI_ARCHITECTURE.md](docs/reference/CI_ARCHITECTURE.md) | [features.toml](features.toml)

**SDLC positioning**: [DISTRIBUTED_ENGINEERING_LINEAGE.md](docs/reference/DISTRIBUTED_ENGINEERING_LINEAGE.md) — situates the Octopus Cluster in classical engineering practice (Kanban, code review, trunk-health, CI/CD, SRE) with Beowulf contrast and SDLC-mapping table.

## Contributing

Run `just pr-fast` while iterating and `nix develop -c just ci-gate` before merge. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Continuous Swarm Development

**Session start**: Run `just clean-worktrees` to prune stale agent worktrees before spawning new ones.

Start with `/swarm all`. Orchestrator spawns scoped agents from the catalog in worktree isolation. ~20% capacity reserved for background improvement.

**Key commands**: `/swarm` (start), `/swarm-protocol` (rules), `/coding-standards` (standards), `/verify` (crate gate), `/parser-fix` (TDD fix).

**PR lifecycle**: Draft PR -> reviewer agent -> `/pr-ready` -> CI -> ops agent merges.

**Files**: `.ops-perl-lsp/` (metrics), `.claude/agents/` (agent defs and catalog), `.claude/commands/` (step skills and shared ops).
