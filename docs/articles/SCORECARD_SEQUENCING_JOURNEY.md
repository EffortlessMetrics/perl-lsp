## The Scorecard Sequencing Journey: From Theoretical to Empirical

**Date**: 2026-04-11
**Umbrella**: [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)
**Sub-issues**: [#4063](https://github.com/EffortlessMetrics/perl-lsp/issues/4063) parser, [#4065](https://github.com/EffortlessMetrics/perl-lsp/issues/4065) diagnostics, [#4066](https://github.com/EffortlessMetrics/perl-lsp/issues/4066) editor intelligence, [#4067](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) module resolution, [#4068](https://github.com/EffortlessMetrics/perl-lsp/issues/4068) workspace, [#4069](https://github.com/EffortlessMetrics/perl-lsp/issues/4069) DAP, [#4070](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) engineering health

There are two kinds of sequencing plans. The first is the one you write on the back of a napkin when you file the umbrella — it is ordered by vibes and defensible as a hypothesis. The second is the one you commit to after evidence, where every position in the sequence is anchored to a linkable comment. This session had both. The story of how the first became the second, in the space of a single day, is what this article is about.

The napkin-to-evidence transformation is not automatic. It requires a specific discipline: scouts dispatched in parallel before any builder claims a scorecard, plan-reviewers cross-checking scout evidence against workflow files and crate metadata, research verifiers pulling in outside reference models, and an umbrella plan-reviewer weaving the whole thing into a revised rollout. Each piece of that discipline is cheap individually and load-bearing collectively. Remove any one and the sequence collapses back into the napkin version with extra steps.

---

## TL;DR

The metric-stack umbrella ([#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)) was filed early in the 2026-04-11 session with a clean, first-principles rollout order: A + G first, B + C next, D / E / F last. By the end of the same session, that sequence had been revised four separate times — each revision anchored to a specific scout or plan-reviewer comment that contradicted one of the original assumptions. The rollout we now recommend is not the rollout we started with, and the transformation happened in hours, not weeks, because the scouts and plan-reviewers were dispatched *before* any builder claimed a scorecard. This article is a narrative retrospective of that journey, and an argument that the journey itself is the valuable artifact: **theoretical sequencing is a starting point, not a plan, and sequencing decisions should be held open until at least one reality-check pass has completed.**

---

## The Initial Sequence (as filed into #4062)

When the umbrella landed, the sequence section read exactly as follows ([#4062 body](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)):

> **Priority order once implementation starts**:
> 1. **A** and **G** first (extend existing metrics; cheap to land)
> 2. **B** and **C** next (highest public-credibility leverage)
> 3. **D**, **E**, **F** (largest engineering lift, most editor-specific)

That is a perfectly reasonable ordering for someone writing it in the body of a new umbrella issue with no evidence in hand. The logic is transparent:

- **A (Parser)** and **G (Engineering Health)** are cheap because they extend signals the project already computes. Parser corpus rates, node-kind coverage, mutation scores — all already in `.ci/` receipts somewhere. Surfacing them in `docs/project/status/` is plumbing, not invention.
- **B (Diagnostics)** and **C (Editor Intelligence)** are where the public credibility leverage lives. These are the metrics an external reviewer wants to see before trusting that the language server is *right*, not just *wired up*. Everyone feels this instinctively: "capability coverage" is easy to fake; "precision/recall on a gold set" is not.
- **D (Module Resolution)**, **E (Workspace)**, **F (DAP)** are framed as the heavy lifts. They sound heavy. They involve state machines, fixture corpora, cross-process debugging, multi-root workspace semantics. It is intuitive to push them to the end.

The ordering is defensible *a priori*. It is also almost entirely wrong once you look at the actual codebase. But you cannot know that from the umbrella body alone. You only know that after somebody reads the subsystems.

The interesting question is not "why was the umbrella wrong?". The umbrella was not wrong in any blameworthy sense — it was filed by a scout who correctly read the design space and proposed a reasonable hypothesis. The interesting question is: **how wrong would it have been if the sequence had been treated as authoritative and handed directly to builders?** Refinement 4 (the 7×3 conformance matrix) alone would have cost a builder a full cycle — they would have written a single-row scalar test, shipped it, and then been asked to re-do it as a matrix next session. Refinement 2 (the missing `--json` flag) would have cost a silent no-op merge that took days to diagnose. Refinement 3 (F PR 2's alpha-blocking promotion) would have cost a post-alpha filing of work that could have landed pre-alpha with 50 marginal LOC. The cumulative cost of the four unfounded assumptions, if not caught, is somewhere between 1.5 and 3 builder-cycles plus a trust hit on the engineering health dashboard. That is the number the scout wave is competing with.

So the orchestrator did the one thing that turned the umbrella from a spec into a measurable process: dispatched seven scouts in parallel, before any builder touched a file.

---

## The Seven Scouts

All seven sub-issues ([#4063](https://github.com/EffortlessMetrics/perl-lsp/issues/4063), [#4065](https://github.com/EffortlessMetrics/perl-lsp/issues/4065), [#4066](https://github.com/EffortlessMetrics/perl-lsp/issues/4066), [#4067](https://github.com/EffortlessMetrics/perl-lsp/issues/4067), [#4068](https://github.com/EffortlessMetrics/perl-lsp/issues/4068), [#4069](https://github.com/EffortlessMetrics/perl-lsp/issues/4069), [#4070](https://github.com/EffortlessMetrics/perl-lsp/issues/4070)) got scouts dispatched concurrently. Each scout's job was the same: verify what already existed in the subsystem, figure out the cheapest path to the first real scorecard row, and file the MVP. What the seven scouts returned was surprising — not one by one, but in aggregate.

### #4063 — Parser

The [scout comment](https://github.com/EffortlessMetrics/perl-lsp/issues/4063#issuecomment-a1) opens with a breakdown of six proposed metrics into three cost tiers. The top four ("FREE METRICS" — clean file rate, node-kind coverage, timeouts/panics/unreadable counts, strict-clean subset pass rate) were **all already computed**. The baseline JSONs in `.ci/parser-corpus-baseline.json` and `.ci/cpan-corpus-baseline.json` contained them. `corpus_audit_report.json` contained the 65/69 (94.1%) node-kind coverage number. The strict-clean subset was already being enforced by `just common-corpus-check`. The scout's MVP was 50 lines in `xtask/src/tasks/update_status.rs` to promote the existing values to first-class dashboard rows. **Cheap as predicted.**

### #4065 — Diagnostics

The [diagnostics scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4065#issuecomment-b1) found mature diagnostics tests — but no gold corpus and no scorecard-shaped harness. The MVP was ~4 hours of work but was **blocked on a fixture format decision** the scout could not make alone. Three scouts (this one, #4066, #4067) each proposed a format. None agreed. Without a shared format, B and C could not land in parallel. More on that in the plan-reviewer section.

The scout's proposal deserves credit even though it lost the arbitration: the header-comment format would have allowed diagnostic assertions to live inside the `.pl` file as `# expect: PL100 at line 0..10 "strict"` comments, which is ergonomic for humans reading a fixture. The cost — correlating line numbers through a custom parser — was not visible to the scout until the plan-reviewer weighed it against two alternative proposals. That is the kind of local-maximum trap individual scouts are prone to and plan-reviewers catch.

### #4066 — Editor Intelligence

The [editor intelligence scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4066#issuecomment-b2) found that the LSP test harness (`crates/perl-lsp-ux-tests/`) could drive hover / completion / goto-definition requests for essentially free — on the order of 0-5ms per request. A shared fixture format was feasible. Scope was "medium" — nothing in the subsystem was missing, only the scorecard harness.

### #4067 — Module Resolution

This is the one that changed the whole sequence. The [module resolution scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4067#issuecomment-b3) opens with a claim that seems too good to be true:

> **CRITICAL**: Position-aware lexical resolution is ALREADY IMPLEMENTED and working correctly.

The scout documented six metrics the umbrella had listed as D-scorecard targets (workspace-relative include root, absolute `use lib`, lexical `use lib` / `no lib`, FindBin-relative, system `@INC`, consumer consistency) and — for the first five — pointed at existing tests that already pass. `resolve_use_lib_paths_from_source()` at `crates/perl-module-resolution/src/use_lib.rs:147` already preserves order of `use lib` and `no lib` statements. A test at `use_lib.rs:524` (`resolves_use_and_no_lib_order`) already validates the canonical `"use lib 'lib'; no lib 'lib'"` case. The only gap the scout found was a **consumer consistency harness** — a single test that calls `textDocument/definition`, `textDocument/hover`, and `textDocument/diagnostic` on the same module reference and asserts all three agree. That harness was estimated at ~100 LOC.

**The umbrella had framed D as one of the three heaviest lifts. The scout found it was one of the cheapest.**

### #4068 — Workspace

The [workspace scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4068#issuecomment-b4) catalogued the existing substrate: `perl-workspace-index-slo` crate with `SloTracker` and `OperationType` already defined, 20 benchmarks (the plan-reviewer later corrected this to 19) already registered in `workspace_index_benchmark.rs`, and 8 multi-root tests (1020 lines) from PR [#3984](https://github.com/EffortlessMetrics/perl-lsp/pull/3984) at `crates/perl-lsp-rs/tests/multi_root_workspace_tests.rs`. The scout offered **Option A** (surface existing data, 2-3 weeks) versus **Option B** (full stale-defect harness, 4-5 weeks). What the scout missed — and what the plan-reviewer caught — was that the multi-root tests were gated behind `--features workspace,expose_lsp_test_api` **and** required `PERL_LSP_WORKSPACE=1`, neither of which any `justfile` recipe or `.ci/` configuration activated. The tests existed but did not run.

### #4069 — DAP

The [DAP scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4069#issuecomment-b5) reported a mature test landscape: 758 integration tests, 186 unit tests, 13 fixture debuggees, and a working `dap_smoke_e2e.rs` harness that already imports `Duration` and `Instant` and uses `wait_for_event`. An MVP for launch success rate was ~150 LOC. The plan-reviewer later corrected the counts to 715 integration and 311 unit — the scout's "758" was a hybrid count — but both numbers say the same thing: **there was much more DAP substrate than the umbrella's "largest engineering lift" framing suggested.**

### #4070 — Engineering Health

The [engineering health scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4070#issuecomment-b6) found `generate_quality_status()` at `xtask/src/tasks/update_status.rs:712` hard-coded to a placeholder "87%" mutation score. The MVP was ~80-100 LOC to parse `mutants.out/mutants.json` and render per-crate counts, but it was **blocked on accuracy verification of the `cargo mutants --json` output shape**. An accuracy-scout unblocked it a few hours later by running `cargo mutants --json` locally and confirming the record format.

The accuracy-scout pass is a separate layer from the plan-reviewer. Where the plan-reviewer reads the codebase in its current state and corrects factual errors, the accuracy-scout *runs commands* to verify that what the scout described actually behaves as described. In the #4070 case the scout proposal included reading a `cargo mutants --json` output file whose format no committed evidence confirmed. The accuracy-scout ran the command locally, captured the output, and wrote a verification comment with the actual record shape pasted in — `{"package": "perl-quote", "file": "...", "genre": "FnValue"}`. That is what unblocked the builder. The accuracy-scout's value-add is not "is the scout wrong?"; it is "does this thing actually produce what the scout said it would produce?". It is a third kind of reality check, complementary to (but distinct from) the plan-reviewer's source-tree cross-check.

### The aggregate surprise

Five of seven scorecards had significantly more existing infrastructure than the umbrella body assumed. The gap in most of them was **dashboard surfacing**, not measurement investment. That single observation changes the sequencing economics. If your "heavy lift" scorecard turns out to be a 100-LOC consumer-consistency harness, it does not belong at the back of the queue. If your DAP scorecard turns out to be a 150-LOC wrapper over existing e2e infrastructure, the cold/warm timing row is not a post-alpha concern.

It is worth stating what the aggregate pattern actually was, because it is load-bearing for the meta-lesson at the end. Tally of substrate against umbrella assumption:

| Scorecard | Umbrella framing | Scout-verified reality | Gap type |
|-----------|------------------|----------------------|----------|
| A — Parser | Cheap, extends existing | 4 of 6 metrics already computed in baseline JSONs | Surfacing |
| B — Diagnostics | Highest public leverage | Tests mature, no gold corpus, blocked on format | Harness + format |
| C — Editor Intelligence | Same | Harness exists, 0-5ms per request, blocked on same format | Format |
| D — Module Resolution | Heavy lift | Implementation already correct, 1 harness gap | Surfacing + harness |
| E — Workspace | Heavy lift | SLO crate + 19 benches + 8 tests exist, tests CI-dormant | Activation + surfacing |
| F — DAP | Heavy lift | 715 int + 311 unit tests, `Duration`/`Instant` already imported | Surfacing |
| G — Engineering Health | Cheap, extends existing | Placeholder 87%, blocked on `--json` flag + scope creep | Surfacing + scope-split |

Six of seven scorecards have "Surfacing" or "Harness" in the Gap type column. Not one of them has "Design new subsystem" or "Write new production code". That is the pattern. The umbrella's framing of D/E/F as "largest engineering lift" was reading the problem space from outside — seeing how hard module resolution sounds in the abstract, how intricate DAP is in principle, how bug-prone multi-root workspace semantics are in general. The scout wave was reading the problem space from inside — seeing what had already been built. The two views disagreed because one was about the shape of the domain and the other was about the shape of the codebase, and the metric-stack umbrella was fundamentally a codebase-surfacing project, not a domain-discovery project.

Once that is named, the sequencing follows. Codebase-surfacing projects are ordered by "where is the substrate thickest and cheapest to expose", not by "which domain is hardest". D-MVP is cheap because the substrate is thick and ready. F PR 2 is cheap because the substrate is thick and ready. B and C are gated on a format decision that is *upstream* of their substrate, not because the substrate is thin.

Notice what the scouts *did not* do. They did not redesign the umbrella. They did not rewrite the sequence. They did one thing well: they counted what already existed. The redesign came from the plan-reviewers.

---

## The Plan-Reviewer Refinements

Plan-reviewers ran on each scout's output, and on the umbrella itself. Their corrections are the load-bearing evidence that turned the theoretical sequence into an empirical one. Four refinements mattered.

### Refinement 1 — #4068 plan-reviewer caught the CI-dormancy gap

The [#4068 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4068#issuecomment-c1) opens with a line-by-line check of the scout's infrastructure claims against current master. The `SloConfig` / `SloTracker` / `OperationType` claims were correct. The benchmark count was off by one (19, not 20). The 8 multi-root tests existed at the right line numbers. But then:

> **Critical finding — multi-root tests are CI-dormant:** The 8 multi-root tests require `--features workspace,expose_lsp_test_api` AND `PERL_LSP_WORKSPACE=1`. No existing justfile recipe or `.ci/` configuration activates those features. The tests were merged (commit `a180fdf9`, PR #3984) but are not running in any CI gate today.

This is the kind of gap a scout cannot catch because the scout is reading the source tree, not the workflow files. The plan-reviewer cross-checked against the `.github/workflows/` and `justfile` and found the tests had been silently inert since merge. The scout's "4 of 6 metric coverage" claim was corrected to "2.5 of 6" — because the multi-root correctness and cross-workspace navigation rows were being counted against test existence, not test execution. Option A (surface existing data + activate multi-root) was picked over Option B (unbounded stale-defect harness). Revised effort: 3 PRs over 1.5-2 weeks, bundled with a new `ci-workspace-multiroot` justfile recipe.

The correction is more than a coverage-number revision; it is an argument about what counts as "done" for a test. A test that exists in source but does not run in any CI gate is halfway between done and not-done. It is a regression guard against the specific state the test captures, *at the moment the test was written*, against a developer's local machine, *if they remember to run it*. It is not a regression guard against future drift, because nothing in the release path will fail when drift happens. The plan-reviewer's move was to refuse to count CI-dormant tests as coverage. That is a standard the umbrella body had not articulated — and probably had not thought to articulate, because until you run scouts against multiple subsystems, the CI-dormancy failure mode is not a pattern you expect to see twice.

The Option A spec the plan-reviewer produced is concrete: a new `ci-workspace-multiroot` justfile recipe wired into the nightly gate (not the merge gate, because each test uses 15s indexing timeouts — too slow for the merge path), promoted to the merge gate only after confirming stability. A new `docs/project/status/workspace.md` with an SLO table anchored to `perl-workspace-index-slo`'s `SloConfig`. A benchmark results loader in `xtask/src/tasks/update_status.rs` that reads `benchmarks/results/latest.json` and rewrites the Status column with actual p95 values vs. targets. That is what "4 of 6 metrics, for real this time" looks like in code.

**Effect on sequencing:** E stays in Wave 3, but E's Wave 3 work now includes a single-justfile-recipe CI activation of tests that were supposed to be running already. A known-unknown became a known-known. The gap was *silent*; the plan-reviewer made it *loud*.

### Refinement 2 — #4070 plan-reviewer caught the load-bearing `--json` flag

The [#4070 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4070#issuecomment-c2) is the most important single comment in the whole session for this retrospective. The plan-reviewer ran through the scout's file references — `xtask/src/tasks/update_status.rs:712`, `.cargo/mutants.toml`, `justfile:281`, `.ci/debt-ledger.yaml` — and verified all of them. Then the plan-reviewer looked one hop further than the scout had:

> **Critical correction for PR 1 — CI mutation run lacks `--json` flag:** The CI nightly mutation job (`.github/workflows/ci-nightly.yml`) runs `cargo mutants --timeout 60 --no-shuffle` with no `--json` flag. Without `--json`, `mutants.out/mutants.json` is never written. The accuracy-scout's code sketch assumes this file exists; it will not be present in CI without a workflow patch. **PR 1 must also patch `ci-nightly.yml` to add `--json`.**

Without this correction, the entire G scorecard PR would have landed as a silent no-op. The `update_status.rs` code would read `mutants.out/mutants.json`, find nothing, fall back to the 87% placeholder, and write a dashboard row that looked correct but never updated. Nobody would notice for days. By then the code would be merged, the debug would be expensive, and the trust cost of a "quality scorecard" that did not update would be larger than the entire PR savings.

The plan-reviewer also scope-split #4070. The original scout spec had ballooned under the pressure of the #4099 research (on which more later) from "per-crate mutation row" to "per-crate mutation + per-subsystem latency + hierarchical memory + release-health dashboard + product-vs-execution separation". The plan-reviewer recognized this as a scope creep, kept PR 1 (per-crate mutation + per-crate tests, ~130 LOC) in #4070, and filed PRs 2-5 as a new umbrella [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106) (`xtask-metrics-framework`). That split is what made G tractable for Wave 1.

**Effect on sequencing:** G stays in Wave 1 but is now **~130 LOC** instead of ~500-800. The rest of G migrates to #4106, where it becomes a post-alpha umbrella. Without the split, G would have been forced to Wave 3 or later, contradicting the umbrella's "A + G first" assumption.

The scope-split itself is a second-order sequencing move worth dwelling on. The [#4070 plan-reviewer](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) did not simply shrink the scope — they filed a new umbrella, [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106), that captured the dropped work as a structured multi-PR plan. That means the cost of keeping G in Wave 1 is not "throw away the other 400 LOC of spec"; it is "re-home the other 400 LOC of spec in a post-alpha umbrella where the research inputs (#4099 rec B, clangd memory model, ratchet cadence tiers) are load-bearing and the urgency is lower". The spec survives intact. The sequencing change is "later, not never". That distinction matters for trust — if the original scout had seen their scope cut from 5 PRs to 1 PR with no follow-up filed, the next session's scouts would be more conservative about filing comprehensive specs. By filing #4106, the plan-reviewer preserves the incentive for scouts to be thorough even when the thoroughness exceeds the immediate wave.

(A note on the coordination history: the initial plan-reviewer recommended the `--json` patch go into `ci-nightly.yml` directly. A subsequent builder's work — commit `a717174b` on the `feat/parser-scorecard-engineering-health` branch — discovered that the actual fix was a different shape: an artifact upload step, not a flag change. The plan-reviewer caught the *existence* of the dependency correctly; the exact shape was refined in build. This is the expected division of labor between plan-review and build.)

### Refinement 3 — #4069 plan-reviewer promoted F's PR 2 from post-alpha to alpha-blocking

The [#4069 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4069#issuecomment-c3) worked through the scout's counts (correcting 758 to 715 integration, 186 to 311 unit across 12 DAP crates) and confirmed the fixture list. Then the plan-reviewer did something the umbrella had not anticipated:

> **Scope vs. Alpha Framing (#4062):** The original #4062 umbrella framed DAP cold/warm timing as post-alpha. **Recommend revising:** `dap_smoke_e2e.rs` already has `Instant` and `wait_for_event`; PR 2 is ~50 additional LOC. Median time to first stopped event was explicitly requested in the #4069 body. **Promote PR 2 to alpha-blocking.** PR 3 can remain post-alpha if bandwidth is tight.

The umbrella's "D/E/F last" framing had assumed DAP timing instrumentation was a heavy separate lift. The plan-reviewer pointed out that the scaffolding was already in `dap_smoke_e2e.rs` — `Instant`, `Duration`, `wait_for_event`. A cold-launch p50/p95 measurement was ~50 marginal lines, not a new harness. Once that was visible, there was no structural reason to push PR 2 past the alpha window.

The plan-reviewer also spelled out the three PR sequence in detail:

- **PR 1** (launch success rate, ~150 LOC): a new `crates/perl-dap/tests/dap_scorecard_harness.rs` file that loops over 5 launch-safe debuggees, sends initialize/launch/stopped, records elapsed time, asserts `passed >= 4/5` (80% threshold), and emits `cold_launch_p50` / `cold_launch_p95` via `eprintln!` as observational output.
- **PR 2** (cold/warm/step-latency instrumentation, ~100 LOC): three test functions — `scorecard_cold_launch_timing` (run hello.pl 5 times with fresh DebugAdapter each, measure launch→stopped), `scorecard_step_latency` (after first stopped on loops.pl, send 5× `next` requests, measure request→next-stopped), and `scorecard_variables_heavy_hitters` (informational, sort 10 `variables` calls by elapsed, emit top-3).
- **PR 3** (heavy-hitter detail reports, ~80 LOC, post-alpha eligible): 20 `evaluate` expressions of increasing complexity, 10 `variables` reference depths, sort by elapsed time, emit top-5 slowest with detail.

Each PR has a soft-target vs. hard-assert split. Hard asserts are permissive (< 10s launch, < 5s step) and serve as CI timeout guards. Soft targets (< 2000ms p50 launch, < 5000ms p95 launch, < 500ms p50 step, < 2000ms p95 step) are emitted as observational output, not asserted. That split is the #4105 ratchet model's "floor vs. improvement" distinction made concrete: the hard asserts are the floor, the soft targets are the improvement goals that will be ratcheted up once stable. A builder reading the plan-review comment knows exactly which numbers cost them a red CI and which are for the dashboard.

**Effect on sequencing:** F's Wave 3 shrinks. PR 1 (launch success rate, ~150 LOC) and PR 2 (cold/warm timing, ~100 LOC) are both alpha-eligible. Only PR 3 (heavy-hitter detail reports) stays post-alpha.

### Refinement 4 — #4067 plan-reviewer rejected the one-test MVP for a conformance matrix

The [#4067 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4067#issuecomment-c4) is the subtlest of the four refinements. The scout had proposed a single fixture workspace with one test row exercising all six resolution-mode metrics through one consumer. The plan-reviewer agreed the implementation was correct — position-aware lexical resolution already worked — and agreed a consumer consistency harness was the right shape. But the plan-reviewer rejected the MVP on a specific ground:

> The scout MVP spec (one fixture, one test row) is rejected in favor of the **conformance matrix** shape recommended by research issue #4099. That research explicitly states: "Turn `@INC` into an explicit conformance dashboard. Track separately as a **conformance matrix** (not a scalar score)."

A scalar "consumer consistency pass rate: 1/1" is not a scorecard — it is a trivia column. The plan-reviewer upgraded the scope to a **7 x 3 matrix** (7 resolution modes × 3 LSP consumers: PL701 diagnostic, goto-definition, hover — completion was dropped after the plan-reviewer confirmed `completion.rs` does not call `resolve_module_path*`). Same implementation cost (the work was already written), much more signal.

The seven rows of the matrix deserve naming because they are the concrete thing the scorecard will report against:

| Mode | Fixture | Notes |
|------|---------|-------|
| Relative `includePaths` | `inc_relative_include_path` | `lib/` in config, module in `lib/` |
| Absolute `includePaths` | `inc_absolute_include_path` | absolute path in config |
| `PERL5LIB` env var | `inc_perl5lib_env` | injected via `ScenarioConfig::env()` + separate tempdir |
| Lexical `use lib` | `inc_use_lib_lexical` | `use lib 'lib'` in source |
| `no lib` cancellation | `inc_no_lib_cancellation` | `use lib 'lib'; no lib 'lib'` — module must NOT resolve |
| FindBin-relative | `inc_findbin_relative` | `use FindBin; use lib "$FindBin::Bin/../lib"` |
| System `@INC` | `inc_system_inc` | `use_system_inc: true` in config, module in temp system dir |

Each of the 21 cells (7 modes × 3 consumers) becomes a floor in the ratchet sense from #4105 — must not regress, merge-blocking. If PR #4200 next month accidentally breaks `use lib` resolution for the hover consumer but leaves goto-definition untouched, one cell flips red and CI blocks the merge. That granularity is what a scalar "consumer consistency pass rate: 21/21" would not give you; the scalar tells you *something broke*, but the matrix tells you *what broke*, which is the information you need to route the fix without spelunking through every consumer.

The scout had also listed completion as one of four module-resolution consumers. The plan-reviewer checked `crates/perl-lsp-rs/src/runtime/language/completion.rs`, found no `resolve_module_path*` call, and corrected the consumer set to three. A scout-scope error of 25% was caught before it turned into 25% of a builder's effort being wasted. That is the kind of correction that looks trivial in hindsight and saves hours in practice — a builder would have wired up a completion call, watched it return unrelated results, and spent a debugging session before realizing the consumer was never supposed to be on the list.

**Effect on sequencing:** D-MVP stays in Wave 1 (the effort is still ~100 LOC because the matrix is data, not new harness code), but the shape of the resulting scorecard is an actual conformance matrix rather than a single row. The difference matters because a single row does not tell you which resolution mode regresses when the next #4077-style fix lands.

### The umbrella plan-reviewer's integrated synthesis

The [umbrella plan-reviewer synthesis](https://github.com/EffortlessMetrics/perl-lsp/issues/4062#issuecomment-d1) consolidated all seven scout reports plus the four per-scorecard refinements into one revised rollout. The opening move was to reject three incompatible gold-corpus format proposals (from scouts #4065, #4066, and #4067) and pick **companion JSON** (`.gold.json` files alongside `.pl` fixtures) as the canonical format. The directory is `crates/perl-lsp-rs/tests/fixtures/gold/`. The schema has `diagnostics`, `hover`, `goto_definition`, `completion`, and `module_resolution` sections; each consumer scorecard reads only the sections it owns; any section may be absent; `"diagnostics": []` is the negative-case encoding.

The format argument is worth a paragraph on its own, because it is the cleanest example of the whole session's pattern: **scouts can identify a problem, but they cannot arbitrate between three honest proposals**. The inline-marker format (from #4066) is ergonomic for human readers but breaks under `perltidy` reformatting and cannot express multi-position assertions. The Perl header-comment format (from #4065) is low-friction but requires correlating assertions to line numbers via a custom parser. Companion JSON (what the plan-reviewer chose) has the ugliest visual but matches the format already used in `crates/perl-dap/tests/fixtures/golden_transcripts/`, survives reformatting, and lets each consumer scorecard read only its own section without parsing the others. The plan-reviewer's authority for that call came from cross-checking against an existing pattern in the DAP tests — a piece of evidence no individual scorecard scout had cause to look at.

That format decision is the unblocker for Wave 2. Without it, B and C could not land in parallel. With it, they share a fixture directory and a single `gold_scorecard.rs` harness — one test function driving all fixtures sequentially through one persistent `LspServer` instance, because one test per fixture would deadlock under `RUST_TEST_THREADS=2`. The "one test function, one server instance" detail matters and is the kind of thing that only surfaces when an experienced plan-reviewer looks at the shape of `lsp_master_integration_test.rs` and recognizes the failure mode in advance. A naive scorecard scaffold (one `#[test]` per fixture) would have been flaky for weeks before anyone traced it back to the test-threading configuration.

Then the umbrella plan-reviewer committed to the revised sequence. The key line:

> Original umbrella order: A+G first, then B+C, then D/E/F. **Revised sequence:** ... Wave 1 -- No infrastructure required (hours each): A (Parser scorecard) ~50 lines ... D-MVP: Module Resolution consumer consistency ~100 lines, zero blockers ... G-partial: Engineering Health test counts ~20 lines ...

D moved from Wave 3 to Wave 1 on the strength of two concrete facts: (a) the implementation was already correct, and (b) the harness was ~100 LOC of test code in `crates/perl-lsp-ux-tests/`, not new production code. That is an empirical move, not a framing move. The plan-reviewer did not argue that D was "more important than we thought" — they argued that D was *cheaper than we thought*, which is a different and stronger argument.

The synthesis also produced a risk register. Five risks were called out explicitly, and each one is a thing that would have cost a build cycle if it had been discovered in-build rather than in-review:

1. **Gold corpus directory collision (MEDIUM)** — scouts #4065 and #4066 each proposed a different directory, both wrong. Correct directory: `crates/perl-lsp-rs/tests/fixtures/gold/`. Mitigation: the synthesis comment is the canonical spec; builders read it before writing.
2. **Engineering Health mutation blocker (HIGH if rushed, LOW if staged)** — the `cargo mutants --json` flag was not wired through CI. See refinement 2 above.
3. **Gold scorecard threading (MEDIUM)** — one `#[test]` per fixture would deadlock under `RUST_TEST_THREADS=2`. The synthesis called this out before any builder wrote the scaffold.
4. **DAP scorecard platform guard (LOW)** — `dap_smoke_e2e.rs:83` has a soft-skip for missing `perl` executable. The DAP scorecard harness must use the same guard pattern. A hard failure would break CI on any runner without Perl installed.
5. **UxHarness multi-file path handling (LOW)** — the synthesis explicitly told the #4067 builder to verify that `with_file("lib/MyModule.pm", content)` creates the file at the correct relative path within the temp workspace root. If the harness flattened paths, `use lib 'lib'` resolution would fail silently and the scorecard would report false positives.

Three of those five risks (1, 3, 5) are risks that only exist because of implementation detail in the test harness — nothing a scout could have flagged from the issue body alone, because the issue body does not say "use the companion JSON format" or "one test function not one per fixture". Plan-review is where implementation-detail risks get surfaced. The risk register is not a failure of scout work; it is the value-add that plan-review is supposed to provide.

### The coordination resolution subdrama

There is one more piece of the gold-corpus-format arc worth recording, because it shows the pipeline's self-correction in action. The umbrella plan-reviewer picked `crates/perl-lsp-rs/tests/fixtures/gold/` as the canonical directory. A separate plan-reviewer working on the gold-corpus format spec independently picked `test_corpus/gold/<fixture-name>/`. The #4065 plan-reviewer had picked the same `test_corpus/gold/<fixture-name>/` earlier still. Two of three votes went to the second location.

The [coordination resolution comment on #4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062) adjudicated the conflict with four specific reasons:

1. Two of three plan-reviewers independently picked `test_corpus/gold/`.
2. **Cross-crate neutrality.** `test_corpus/` is project-wide and already consumed by multiple crates. `crates/perl-lsp-rs/tests/fixtures/` would lock the gold fixtures to the `perl-lsp` crate — forcing #4067 (module resolution) and #4065 (diagnostics), neither of which live in `perl-lsp`, to reach across crate boundaries.
3. `CorpusPaths` discovery mechanism already handles `test_corpus/` via `crates/perl-corpus/src/`. Using a sibling directory under the same root is the path of least resistance.
4. **Explicit trap avoidance.** `crates/perl-corpus/Cargo.toml` `include = ["src/**", ...]` excludes `fixtures/`, so putting fixtures under `crates/perl-corpus/fixtures/` would not be reachable at test time — both plan-reviewers had caught this independently.

The third reason is a detail no single plan-reviewer was wrong about — they all agreed the directory mattered — but they had different mental models of which crate should own the fixtures. The resolution came from a fourth pass that weighed the evidence and committed to `test_corpus/gold/<fixture-name>/` as the canonical location. The umbrella plan-reviewer's other decisions (A+G+D-MVP Wave 1 promotion, README line 168 correction, 7×3 matrix, risk register) all stood unchanged. Only the directory line moved.

This is a small drama but an instructive one. It shows that plan-reviewers disagreeing is not a failure of the pipeline — it is a signal that a decision needs one more pass. The fix is not to force the first plan-reviewer to be right; it is to let the conflict surface and then resolve it with a fresh evidence weighing. The resolution comment is itself linkable, which means a future builder reading the issue history sees three proposed locations and one adjudication with four explicit reasons. That is a cleaner audit trail than "we decided on X" with no evidence for why X beat Y.

---

## The Research-Verifier Inputs

Two research artifacts landed mid-wave and fed into the umbrella synthesis. Neither was in the original umbrella body.

### #4099 — Reference-model research

[#4099](https://github.com/EffortlessMetrics/perl-lsp/issues/4099) studied how rust-analyzer, gopls, pyright, and clangd handle metrics. The [synthesis comment on #4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062#issuecomment-e1) extracted six takeaways. Three of them changed the shape of the metric stack:

1. **Developer instrumentation is the biggest current gap.** rust-analyzer's `analysis-stats` / `RA_PROFILE` and pyright's `--stats --verbose` make per-phase timings and slowest-file reports first-class. perl-lsp had SLO targets but no per-phase breakdown CLI. The synthesis called this out as "probably the single highest-ROI metrics improvement".
2. **Cold / warm / incremental must be tracked separately** per subsystem. Current perf numbers did not distinguish.
3. **@INC should be a conformance matrix**, not a scalar score.

The other three takeaways (hierarchical memory accounting per clangd's `$/memoryUsage` pattern, gopls-style release-health model, explicit product-vs-execution separation) landed in #4106 as PRs 3, 4, and 5 respectively. They are not sequencing-reshaping in the same way as the first three, but they are shape-reshaping — they tell you what the Wave 4 destinations look like when the climbing sequence gets there. Without them, #4106 would have been a thinner umbrella and Wave 4 would have been a collection of orphan ideas.

Takeaway 3 is the one that drove refinement 4 above. But takeaways 1 and 2 fundamentally reshaped what "Wave 2" means. The umbrella had framed Wave 2 as "B + C". After #4099, Wave 2 also has to include the `cargo xtask metrics` subcommand tree (phase timings, slowest-file reports) and hierarchical memory accounting (clangd's `$/memoryUsage` pattern). That work is what #4106 was filed to capture. Wave 2 is no longer B + C; it is B + C + the xtask framework PRs 2-3.

A subtlety in the developer-instrumentation takeaway is worth pulling out. rust-analyzer's `analysis-stats` is not a scorecard — it is a CLI that a developer runs to understand what the analyzer spent time on for a specific file or workspace. It is a profiling tool, not a product metric. But the data it produces (per-phase timings, slowest-file reports) feeds directly into the scorecards that *are* product metrics. The distinction is that developer instrumentation is the substrate and the scorecards are the presentation. perl-lsp had the scorecards planned (A through G) but no substrate. #4099 pointed out that you cannot have credible scorecards without credible substrate, because the scorecard is only as good as the profiling it is built on. That observation moved the xtask subcommand tree from "nice-to-have tooling" to "Wave 2 dependency for B and C". The sequencing change is structural — not just "do this work", but "this work is a prerequisite for the work that was already sequenced in Wave 2".

### #4105 — Ratchet model

[#4105](https://github.com/EffortlessMetrics/perl-lsp/issues/4105) landed after #4099 and added a different layer: operational discipline on top of the scorecard shape. Its four layers:

1. One machine-readable `.ci/metrics/<subsystem>.json` artifact per scorecard.
2. **Floor metrics** (must not regress, merge-blocking) separated from **improvement metrics** (tracked but not blocking every PR).
3. Ratchet only on **re-baselined wins** — improved score stable across N runs before raising the floor.
4. Every open issue tied to exactly one scorecard it improves — the review question shifts from "does this work?" to "which metric does this move?".

The ratchet is the glue that makes sequencing decisions persistent. Without a ratchet, ordering is a one-shot product call — you decide "D in Wave 1", you ship it, and then the next cycle is a fresh argument. With a ratchet, ordering becomes a measurable progression with floor-raise events: Wave 1 ships, the ratchet records initial floors, Wave 2 improvements are tracked as deltas against those floors, and the re-baselining cadence makes "the scorecard improved" a first-class CI event.

The [#4105 synthesis comment on #4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062#issuecomment-e2) also called out an anti-pattern the original umbrella had implicitly indulged:

> Do NOT ratchet activity metrics (PRs merged, agents launched, lines changed, features.toml count). They're execution signals, not improvement signals, and they incentivize volume over value.

This is consistent with #4099's product-vs-execution separation and it is the reason G (Engineering Health) scope was split into #4070 (per-crate mutation, product) and #4106 PR 5 (product vs. execution separation, execution). The split maps onto the ratchet's floor-vs-improvement distinction cleanly.

### The LSP 3.18 delta audit

The [LSP 3.18 conformance audit](https://github.com/EffortlessMetrics/perl-lsp/issues/4062#issuecomment-e3) extended #4099's baseline from LSP 3.17 to 3.18. It is a smaller finding than the other two and it did not change the sequencing directly, but it raised a meta-question the scorecards will eventually have to answer: **what version claim should the scorecards assume as their baseline?** The audit found 7 of 14 LSP 3.18 delta features fully implemented (50%), with 2 implemented but uncataloged. The README currently claims only "LSP 3.17" — it could honestly claim "LSP 3.17 + 7/14 3.18 features" or "LSP 3.17.5 (partial 3.18)". The uncataloged features (`diagnostic_markup_support`, `relative_pattern_support`) fall under the same undersell pattern as #4107. The audit filed concrete priority-1 catalog-alignment work and left priority-2-3 spec implementation to the builder queue.

For sequencing, the LSP 3.18 finding matters because it is evidence that *capability coverage metrics themselves* must include a version delta column. A scorecard that says "102/102 features" means something different in an LSP 3.17 world than in an LSP 3.18 world. The C scorecard (editor intelligence) will need to distinguish between "spec conformance delta" and "correctness on what is advertised". That is not in Wave 1, but it is now captured as a requirement on Wave 2's C spec.

### How the three research inputs compose

The three research artifacts (#4099, #4105, LSP 3.18) do not overlap — they bracket different aspects of the same problem:

- **#4099** answers "what should we measure and how do other language servers measure it?". It is **instrumentation design**.
- **#4105** answers "how do we use those measurements operationally once they exist?". It is **operational discipline**.
- **LSP 3.18** answers "what is the external baseline the scorecards should report against?". It is **spec baseline**.

Without #4099, the umbrella's Wave 2 would have been just "B + C" with no supporting framework, and the product-vs-execution confusion would have persisted. Without #4105, the scorecards would be one-shot measurements with no floor-raise discipline — a new scorecard every session, no memory. Without the LSP 3.18 audit, the C scorecard would have been denominated in a stale baseline and the coverage numbers would have silently drifted out of date within a release cycle. Each of the three research inputs closes a specific gap the umbrella body did not name, and together they reshape what "done" means for the whole metric stack.

---

## The Final Sequence

After all the refinements, the rollout we committed to looks like this:

### Wave 1 — This session and immediate follow-up

| Item | Effort | Evidence |
|------|--------|---------|
| **A**: Parser scorecard | ~50 LOC, combined with G in one PR ([#4124](https://github.com/EffortlessMetrics/perl-lsp/pull/4124)) | [#4063 scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4063) — four metrics already computed |
| **G**: Engineering Health per-crate test counts | ~20 LOC in `count_tier_a_lib_tests()` | [#4070 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) — scope split into #4106 |
| **G**: Per-crate mutation (bundled with CI-nightly `--json` fix) | ~130 LOC + CI workflow patch | [#4070 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) — caught the silent-no-op dependency |
| **D-MVP**: Module resolution consumer-consistency conformance matrix | ~100 LOC, independent PR | [#4067 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) — 7×3 matrix, not one row |

The structural move: **D-MVP promoted from Wave 3 to Wave 1.**

### Wave 2 — Next session or two, after gold-corpus format settles

| Item | Blocker lifted by |
|------|------------------|
| **B**: Diagnostics precision/recall on gold set | Umbrella plan-review chose `.gold.json` companion format |
| **C**: Editor Intelligence hover / completion / goto-definition / symbols | Same — shared fixture directory with B |
| xtask metrics framework PR 2 (subcommand tree + parser-stats) | [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106), scope-split from #4070 |
| xtask metrics framework PR 3 (hierarchical memory) | [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106) |

The structural move: **Wave 2 is no longer "B + C" — it is "B + C + xtask framework".** The xtask framework sits underneath B and C rather than following them, because the shared `.ci/metrics/<subsystem>.json` artifact pattern (from #4105's ratchet model) is what makes both B and C commit their results to durable evidence rather than stdout.

### Wave 3 — Alpha window

| Item | Evidence |
|------|---------|
| **F PR 1**: DAP launch success rate (~150 LOC) | [#4069 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4069) — existing `dap_smoke_e2e.rs` scaffolding |
| **F PR 2**: DAP cold/warm/time-to-first-stopped-event (~100 LOC) — **promoted to alpha-blocking** | [#4069 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4069) — 50 marginal lines |
| **E Option A**: Workspace surfacing + SLO + multi-root activation | [#4068 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4068) — 1 justfile recipe + 3 PRs |

The structural move: **F's PR 2 is alpha-blocking, not post-alpha**. The umbrella had framed all of D/E/F as "post-alpha largest engineering lift". After the scouts and plan-reviewers, only two rows in the final sequence remain post-alpha: F PR 3 and E Option B.

### Wave 4 — Post-alpha

- xtask metrics framework PRs 4-5 (release-health dashboard, product-vs-execution separation) — [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106)
- E Option B (stale-index defect harness) — only if Option A surfaces real regressions
- Ratchet model baselines raised across subsystems as floors stabilize
- Optional opt-in telemetry, per #4099 takeaway on rust-analyzer's privacy-respecting opt-in signal (not committed to in this session; listed as a Wave 4 candidate only)
- LSP 3.18 delta completion — implement the 7 missing features from the LSP 3.18 audit and promote README claim from "LSP 3.17 + 7/14 3.18" to full "LSP 3.18"

Wave 4 is intentionally thin. Most of what looked like Wave 3 or Wave 4 work in the original umbrella migrated to earlier waves once the scout evidence landed. The remaining Wave 4 items are either (a) optional polish that depends on the earlier waves' outcomes, or (b) items that genuinely need to see what the world looks like after alpha ships before committing to a shape. The sequence's center of mass moved earlier, and Wave 4 is where that shift settles.

### What the ratchet enforces

One detail from #4105 is worth pulling out because it is what makes this sequence hold together operationally. The ratchet model's third layer is "ratchet only on re-baselined wins — improved score stable across N runs before raising the floor". In practice this means that once Wave 1 ships, the initial floors are set but not raised; Wave 2 lands under those floors; Wave 3 lands under Wave 2's new floors; and the re-baselining cadence (per-PR smoke / nightly sweep / release-time full) is what converts sequence into progression. Every wave's deliverable is "raise the floor" or "add a new row that will later be floored". The sequence is not a TODO list; it is a climbing sequence, and each hold has to be stable before the next move.

That framing also means the sequencing decisions from this session are not fragile. If a future session finds that D's consumer-consistency row is not ratchet-ready (say, because one of the resolution modes turns out to be flaky on a specific platform), the fix is to demote that row from "floor" to "improvement metric" — not to renegotiate the whole Wave 1. The ratchet's floor-vs-improvement split gives the sequence a graceful degradation path that the original umbrella, which implicitly treated every metric as a product commitment, did not have.

The cadence tier split from #4105 (per-PR / nightly / release) also maps onto the wave sequence cleanly. Wave 1's deliverables (parser scorecard, engineering-health per-crate rows, D-MVP conformance matrix) all fit in the per-PR cadence — they are cheap to run, floor-metric-shaped, and merge-blocking. Wave 2's deliverables (B + C gold corpus work) fit in the nightly cadence — the gold scorecard is too expensive to run every PR but cheap enough to run every night, and that is the natural pace at which diagnostics precision/recall drift would be noticed. Wave 3's deliverables (DAP timing, workspace stale-defect) fit in the release cadence — they involve subprocesses, multi-root setup, or time-sensitive measurements that do not belong in either the fast lane or the nightly lane. The sequence is not just "these three waves in order" but "these three waves onto three cadence tiers", which means the infrastructure cost scales with wave depth instead of all landing on the merge gate at once.

---

## What It Cost vs. What It Saved

A rough accounting of the scout-plus-plan-review investment is worth recording so the cost-benefit is transparent:

| Pass | Agents | Approximate session budget |
|------|--------|---------------------------|
| 7 scouts (parallel) | 7 × haiku | ~1.5% |
| 7 plan-reviewers (parallel) | 6 × sonnet + 1 haiku | ~2-3% |
| Umbrella plan-reviewer synthesis | 1 × sonnet | ~0.5% |
| Research verifier (#4099 ingestion) | 1 × sonnet | ~0.3% |
| Research verifier (#4105 ingestion) | 1 × sonnet | ~0.3% |
| Research verifier (LSP 3.18 audit) | 1 × sonnet | ~0.4% |
| **Total pre-build investment** | **16 agents** | **~5-6%** |

Against that cost, the sequencing corrections prevented:

- At least one wasted builder cycle (D-MVP as scalar rather than matrix) — estimated 0.5%
- A silent-no-op dashboard merge (G per-crate mutation without `--json`) — estimated 0.3% of build plus 2-3% of trust recovery when discovered
- A post-alpha framing of alpha-eligible work (F PR 2) — estimated 0% short-term cost, unknown long-term framing cost
- A CI-dormancy coverage claim (E multi-root) — estimated 0.2% of build plus long-term trust cost
- A fixture-format collision between parallel B and C builders — estimated 1% of build

Total prevented cost, rough range: **~2-4% of session budget plus trust-recovery costs that do not fit in a single session's accounting**. The scout-plus-plan-review wave is roughly breakeven on direct cost even in the narrow sense, and strongly positive once trust costs are included. The trust costs matter more than the direct costs because a metric-stack retrospectively-wrong dashboard damages the credibility of every number on the same dashboard. A silent-no-op mutation row next to an accurate node-kind coverage row is worse than no mutation row at all, because a reader cannot tell which number is stale.

This is also why "dispatch scouts in parallel" matters more than "dispatch scouts at all". A single serial scout wave would have caught most of the individual errors but missed the aggregate pattern (five of seven scorecards have thick substrate) that drove the umbrella synthesis. Parallel dispatch is not a cost-saving shortcut; it is the only configuration that makes the aggregate visible.

---

## The Four Sequencing Changes

Looking back at the delta between the original umbrella body and the final rollout, four specific sequencing assumptions changed. Each one is anchored to a specific scout or plan-reviewer comment:

1. **D moved from Wave 3 to Wave 1.** The umbrella framed module resolution as a "largest engineering lift, most editor-specific" scorecard. The [#4067 scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) found position-aware lexical `@INC` already worked correctly and the only gap was a ~100-LOC consumer-consistency harness. The [#4067 plan-reviewer](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) upgraded the shape to a 7×3 conformance matrix but kept the effort bounded.

2. **F PR 2 moved from post-alpha to alpha-blocking.** The umbrella framed DAP as one of the three heaviest lifts and cold/warm timing as a post-alpha concern. The [#4069 plan-reviewer](https://github.com/EffortlessMetrics/perl-lsp/issues/4069) found `dap_smoke_e2e.rs` already had `Duration`, `Instant`, and `wait_for_event` imported — PR 2 was 50 marginal LOC, not a new harness.

3. **G scope split between #4070 and #4106.** The umbrella framed Engineering Health as "cheap, extends existing metrics". The [#4099 research](https://github.com/EffortlessMetrics/perl-lsp/issues/4099) and [#4105 ratchet model](https://github.com/EffortlessMetrics/perl-lsp/issues/4105) ballooned the ask into a multi-crate instrumentation framework. The [#4070 plan-reviewer](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) kept PR 1 (per-crate mutation, ~130 LOC) in #4070 and filed [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106) as a new umbrella for PRs 2-5. G-partial stays in Wave 1; G-full moves to Wave 2+.

4. **B and C blocked on a gold-corpus format decision not in the original umbrella.** The umbrella framed B and C as "highest public-credibility leverage, next after A and G". The scouts for [#4065](https://github.com/EffortlessMetrics/perl-lsp/issues/4065), [#4066](https://github.com/EffortlessMetrics/perl-lsp/issues/4066), and [#4067](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) each proposed a different fixture format (companion JSON, inline markers, Perl header comments). The [umbrella plan-reviewer](https://github.com/EffortlessMetrics/perl-lsp/issues/4062) picked companion JSON and wrote the canonical schema. Without that decision, B and C could not land in parallel — they would race on directory layout. Wave 2 is gated on the format decision, not on implementation effort.

None of these four changes were visible from the umbrella body alone. All four became visible within hours once the scouts had run and the plan-reviewers had checked their claims.

A useful way to read the four changes is as four *different kinds* of sequencing bug:

- **Change 1 is a cost misestimation.** D was assumed expensive; it was actually cheap. This is the most common sequencing bug and the easiest to catch — a scout reads the subsystem, counts the substrate, and the cost drops by an order of magnitude.
- **Change 2 is a framing mismatch.** F PR 2 was in the wrong bucket ("post-alpha") because the umbrella author had grouped DAP timing with DAP end-to-end workflow tests. Once you separate them (PR 1 launch success rate, PR 2 cold/warm timing, PR 3 heavy hitters), PR 2 reveals itself as alpha-eligible and PR 3 keeps its post-alpha placement.
- **Change 3 is a scope balloon caught in time.** G was sized for "cheap extension" and grew into a multi-crate instrumentation framework after #4099 / #4105 landed. The fix was scope-splitting (#4070 keeps PR 1; #4106 captures PRs 2-5) before a builder committed to the larger scope.
- **Change 4 is a missing prerequisite.** B and C were sequenced without noting that they share a gold-corpus format dependency. The umbrella had not named the format requirement at all, so it could not sequence it. Scouts surfaced it; plan-reviewers resolved it.

Cost misestimation, framing mismatch, scope balloon, missing prerequisite. Four patterns. Each is caught by a different part of the pipeline, and each is invisible from the umbrella body alone.

---

## The Three Layers' Division of Labor

Reading back through the session, a clean separation of concerns becomes visible across the three layers of the pipeline that ran before any builder touched a file:

**Scouts count.** They read source trees, they enumerate what exists, and they file cost-ordered MVPs. They are honest about uncertainty — in this session, the scouts happily wrote things like "position-aware lexical resolution is ALREADY IMPLEMENTED" and "758 integration tests" without claiming authority over what the plan should be. Scouts do not arbitrate between incompatible proposals, do not read workflow files, and do not verify that code paths are reachable from CI gates. That is not a failure mode — it is a division of labor.

**Plan-reviewers verify and synthesize.** They read the scout output *and* the workflow files *and* the cargo metadata *and* the existing patterns in related parts of the tree. They catch the CI-dormant tests, the missing `--json` flag, the "completion is not a module-resolution consumer" error, the test-threading deadlock pattern. They arbitrate between incompatible scout proposals (three gold-corpus formats → one companion JSON schema). They scope-split when the scope balloons (#4070 PR 1 stays, PRs 2-5 fork to #4106). Plan-reviewers turn scout evidence into builder-ready specs with risk registers.

**Research verifiers cross-reference.** They pull in evidence from outside the codebase — rust-analyzer's profiling CLI, clangd's memory accounting, gopls's release-health model, LSP 3.18 delta spec. They answer questions the internal scouts cannot answer ("what do other language servers do?", "what does the newest protocol version cover?"). They reshape the sequence by changing what the destinations are, not by correcting internal evidence.

Each layer catches things the previous layer cannot, and each layer costs less than the one it informs. Scouts are haiku; most plan-reviewers are sonnet; research verifiers are sonnet with web access. The layering is what makes the ~5-6% pre-build investment pay off against ~2-4% prevented build cost plus trust recovery. If you collapse the layers — say, by letting scouts arbitrate between format proposals, or by letting plan-reviewers skip cross-referencing against workflow files — the prevented-cost column shrinks faster than the investment column.

The layers also enforce a specific discipline: **no layer is allowed to redesign the layer below it**. The plan-reviewer does not replace the scout's MVP with a different architecture; they enhance the MVP with the risks and details the scout missed. The research verifier does not rewrite the plan-reviewer's sequence; they add dimensions the sequence needs to account for. The umbrella plan-reviewer does not retcon the original umbrella body; they file a synthesis comment that supersedes conflicting details. Each layer adds; no layer subtracts. That is what makes the journey reproducible — a reader can walk the issue comment history and see the hypothesis-to-evidence transformation in chronological order.

---

## The Meta-Lesson

Theoretical sequencing is a starting point, not a plan. The original umbrella's "A + G then B + C then D/E/F" was entirely reasonable given no evidence. After one wave of scouts and plan-reviewers — a wave that cost a few hours of agent time, not weeks — four of those sequencing assumptions had changed materially. One scorecard moved two waves earlier. One scorecard's second PR moved from post-alpha to alpha-blocking. One scorecard scope-split into two umbrellas. Two scorecards turned out to be blocked on a shared format decision that was not in the original umbrella at all.

The lesson is not "the umbrella was wrong". The umbrella was the right artifact to file: it captured the design space, it listed the seven scorecards, it gave each one a focus, and it proposed a sequence so there was something to react to. That is the job of an umbrella issue. The lesson is about what happens *next*.

**Sequencing plans should explicitly include a "scout wave first" step before committing to order.** The failure mode the original umbrella was close to — and that the session happened to avoid — is the one where a builder claims the top of the sequence on the strength of the umbrella body alone, commits to implementation, and discovers mid-build that the cheap thing was actually hard or the hard thing was actually cheap. Every one of the four sequencing changes above would have cost a full build cycle if it had been discovered during a builder's implementation instead of during a scout's investigation. Some of them (the silent `--json` no-op, the CI-dormant multi-root tests, the completion-is-not-a-consumer error) would have cost more than one build cycle because the first fix would not have been visible until CI ran.

The session avoided that failure mode because the scouts were dispatched **in parallel**, **before** any builder claimed a scorecard. That is the load-bearing detail. Sequential scouts would not have caught the aggregate pattern — that *five of seven* scorecards had more existing infrastructure than assumed. Only parallel dispatch surfaces aggregate patterns, because aggregate patterns are not visible in any single scout's output. You see them when seven comments arrive in the same hour and they all rhyme.

Planning without scout evidence is guessing. Planning after scout evidence — and after plan-reviewer refinement of that evidence — is empirical. Both are legitimate. The mistake is conflating them. The mistake the original umbrella *almost* made was presenting the first kind as if it were the second. The thing that rescued it was not a better umbrella writer — it was the pipeline's insistence that scouts run before builds, and that plan-reviewers run before scouts are trusted.

There is a subtler version of the lesson that is worth naming separately. **The umbrella body should be written as a hypothesis, not as a spec.** If the umbrella body says "Wave 1: A and G. Wave 2: B and C. Wave 3: D, E, F" in declarative form, a builder reading it in isolation will treat the ordering as load-bearing. If the umbrella body says "*Hypothesis (pending scout wave)*: ...", the same builder will know to wait for the evidence. The cost of the declarative form is measured in how many builders pick up a task on the strength of the body alone and how many of those builders then have to re-do work when the sequence changes. In this session the cost was zero because the orchestrator held the umbrella open for scouts before dispatching any builders. In a future session with less discipline, the cost is one full build cycle per unfounded assumption in the declarative sequence. The fix is stylistic: label hypotheses as hypotheses, and reserve the declarative form for evidence-backed specs.

One final observation. The journey this article describes took roughly a day of wall-clock time and roughly 5-6% of the session's budget. In the same day, the alternative — committing to the umbrella sequence and dispatching builders against it — would have cost roughly the same direct budget (builders are not free) and bought substantially less. The dollar savings from scout-plus-plan-review are small. The *trust savings* are large, because every correction caught in the reality-check wave is a correction that does not have to be explained in a postmortem later. The value proposition is not "do this because it is cheap"; it is "do this because the alternative damages the credibility of every downstream decision". Metric stacks are especially sensitive to this: a dashboard with one wrong number next to nine right numbers is worse than a dashboard with nine right numbers and no tenth row, because a reader cannot tell which number is the wrong one without re-checking all ten. That asymmetry is what makes empirical sequencing worth the budget even when the direct cost is a wash.

An umbrella is a hypothesis. The scout wave is the first reality check. The plan-reviewer is the second. The research-verifier (#4099, #4105, LSP 3.18) is the third — pulling in evidence from outside the codebase that reshapes what the codebase evidence means. By the time the builder touches a file, the hypothesis has been through three independent passes and the sequence is no longer a guess.

That is why the journey itself is the artifact worth retrospecting on. The final sequence matters less than the fact that the final sequence is anchored in linkable evidence — every item in every wave above traces back to a specific scout or plan-reviewer comment. A year from now, when one of these scorecards regresses or a new scorecard is added, the question "why was this in Wave 1 and not Wave 3?" has an answer that does not depend on anyone's memory.

### The practical checklist for next time

This is where the meta-lesson becomes operational. The session produced a sequence, a risk register, and a journey. The lesson is in the journey, not the sequence.



If this session's pattern is worth repeating (and the session's budget suggests it is), the practical form of the meta-lesson is a checklist that should be attached to any future metric-stack-shaped umbrella:

1. **File the umbrella with a sequence, but label it "Draft — pending scout wave".** The draft label is not just a politeness; it signals to builders that they cannot claim a position in the sequence until the scout wave has run. That is how you avoid the failure mode where a builder jumps on "Wave 1, Item 1" on the strength of the draft sequence and burns a build cycle discovering Wave 1 Item 1 was actually hard.
2. **Dispatch all scouts in parallel, not serially.** The aggregate pattern is only visible when seven comments arrive in the same hour. Serial dispatch destroys the signal.
3. **Run plan-reviewers on each scout's output before trusting it.** Scouts count what exists. Plan-reviewers check whether what exists is what the umbrella thought it was. The CI-dormant multi-root tests and the missing `--json` flag are the canonical examples of the gap between "the code exists" and "the code is producing signal".
4. **Run at least one research-verifier that looks outside the codebase.** #4099's reference-model research and #4105's ratchet model were both external inputs. They reshaped the sequence in ways no amount of internal scouting could have. A metric-stack project is partly about "what do we measure" and partly about "what does the industry measure". Both need evidence.
5. **Let the umbrella plan-reviewer synthesize the whole wave into one revised sequence, with every change linked to a specific comment.** The final sequence must be reproducible from the evidence. If someone a year from now cannot trace "why D-MVP is in Wave 1" back to the [#4067 scout](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) and [#4067 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) comments, the sequence is not empirical — it is still a guess with better framing.
6. **Commit to the revised sequence in a comment on the umbrella, not by editing the umbrella body.** The umbrella body is the hypothesis; the revised sequence is the result. Both should be readable. Editing the body erases the journey.
7. **Label the scout-wave outputs as linkable evidence, not as scratch notes.** Every scout comment and plan-review comment should be written as a standalone artifact that a future reader can cite. The session worked because the comments were written with that audience in mind — they quote file paths, they include specific line numbers, they correct their predecessors explicitly. Scratch-note quality would have collapsed the audit trail.
8. **Keep the labels in sync with the pipeline state.** Every sub-issue in this session moved through `needs-plan-review` → `plan-reviewed` → `builder-ready`. The labels are the authoritative state; the orchestrator reads them to route work. If a plan-reviewer's refinement changes a scorecard's wave placement, the label change is part of the refinement, not an afterthought.

The checklist reads like overhead when you imagine it in the abstract. In the session itself, it took hours. The cheapest version of this process is still dramatically cheaper than any version of "build the wrong thing first and fix it later".

---

## A Concrete Walkthrough: One Assumption, Three Reality Checks

It helps to see the full trajectory of a single sequencing assumption as it passes through the pipeline. Take the most dramatic one — **D (Module Resolution) moving from Wave 3 to Wave 1** — and trace it through the three layers.

**Starting hypothesis (umbrella body, [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)):** D is one of the three "largest engineering lift, most editor-specific" scorecards. The scorecard is framed around five resolution modes (workspace / absolute / lexical `use lib` / FindBin / system) plus consumer consistency plus position-aware lexical resolution. The umbrella author's mental model is "each of these modes is a state machine that could be wrong; the scorecard surfaces when they go wrong; writing the state machines is hard". Priority: D is in Wave 3.

**First reality check (scout [#4067](https://github.com/EffortlessMetrics/perl-lsp/issues/4067)):** The scout reads `crates/perl-module-resolution/` and finds that `resolve_use_lib_paths_from_source()` at `use_lib.rs:147` already preserves order of `use lib` and `no lib` statements as they appear in source. A unit test at `use_lib.rs:524` (`resolves_use_and_no_lib_order`) already validates the canonical "use lib 'lib'; no lib 'lib'" case. Five of the six metrics listed in the umbrella body are individually tested at the unit level. The only gap is a consumer consistency harness — a test that calls goto-definition, hover, and the PL701 diagnostic on the same module reference and asserts they agree. The scout estimates 2-3 hours of test code in `perl-lsp-ux-tests`.

**Corrected hypothesis (end of first reality check):** D is cheap because the implementation is already correct. Only the harness is missing. MVP: one fixture, one test, one row in a new `docs/project/status/module_resolution.md`. The scout proposes moving D to Wave 1 on cost grounds.

**Second reality check (plan-reviewer [#4067 plan-review](https://github.com/EffortlessMetrics/perl-lsp/issues/4067)):** The plan-reviewer accepts that the implementation is correct but rejects the one-row MVP. The rejection reasons are two: (a) a scalar "consumer consistency: 1/1" does not tell you *which* resolution mode regresses when a future fix lands, and (b) the #4099 research specifically recommends an `@INC` conformance matrix, not a scalar. The plan-reviewer expands the matrix to 7 modes × 4 consumers = 28 cells. Then the plan-reviewer cross-checks the four consumers against source — and finds that `completion.rs` does not call `resolve_module_path*` at any point. The consumer count drops from 4 to 3. The final matrix is 7 × 3 = 21 cells. Effort is still bounded (~100 LOC, because the matrix is data + fixtures, not new harness code), but the scorecard is now actually shaped like a conformance dashboard.

**Corrected hypothesis (end of second reality check):** D-MVP is a 7×3 conformance matrix with 8 fixtures (one mode needs two fixtures for a polarity pair). Still Wave 1. Still ~100 LOC. Now actually useful as a regression anchor.

**Third reality check (umbrella plan-reviewer synthesis, [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)):** The umbrella synthesis confirms the 7×3 shape, initially anchors the fixture directory at `crates/perl-lsp-rs/tests/fixtures/gold/` (a call that the coordination resolution — described in the subsection above — later overrode to the adjudicated `test_corpus/gold/<fixture-name>/`), and adds Risk 5 to the risk register: "verify that `with_file('lib/MyModule.pm', content)` creates the file at the correct relative path within the temp workspace root. If the harness flattens paths, `use lib 'lib'` resolution will fail silently." That is the kind of risk a scout could not flag — it depends on an implementation detail of `UxHarness` in `crates/perl-lsp-ux-tests/src/lib.rs:143` that becomes relevant only when you think through how the fixtures will be loaded.

**Final hypothesis:** D-MVP is in Wave 1. Shape is 7×3 conformance matrix. Directory is shared with B and C. Risk register includes UxHarness path-flattening guard. Total trajectory: the umbrella's "Wave 3, largest engineering lift" assessment was revised to "Wave 1, ~100 LOC, matrix shape, shared directory" across three passes that took maybe half an hour of wall-clock time. Each pass caught something the previous pass had no way to catch, and each pass is linkable.

That is what "empirical sequencing" looks like in practice. It is not "the scouts run and you are done". It is a chain of passes, each adding specific information, each traceable to a comment you can re-read a year from now.

Running the same walkthrough on any of the other three changes produces a similar shape. Change 2 (the `--json` flag) has a two-pass trajectory: scout proposes the update, accuracy-scout confirms `cargo mutants --json` produces per-crate data, plan-reviewer notices that the CI workflow doesn't pass `--json`, orchestrator posts a coordination note linking the chain. Change 3 (F PR 2 promotion) has a three-pass trajectory: scout reports 758 tests + Duration imports, plan-reviewer corrects 758 to 715 and argues PR 2 is alpha-eligible, umbrella synthesis accepts the promotion. Change 4 (the format prerequisite) has a four-pass trajectory: three scouts propose three formats, umbrella plan-reviewer picks companion JSON + `crates/perl-lsp-rs/tests/fixtures/gold/`, separate plan-reviewer picks `test_corpus/gold/`, coordination resolution comment adjudicates. Each trajectory has its own characteristic shape — the number of passes matches the complexity of the problem — but all four converge on the same quality: every position in the final sequence is anchored to a specific comment, and the audit trail is walkable in chronological order.

---

## Cross-References

**Umbrella and synthesis:**
- [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062) — metric-stack umbrella. Original sequence in body. Integrated synthesis comment from umbrella plan-reviewer commits the revised wave plan; gold-corpus format decision; risk register; pre-alpha candidates table. Also hosts the #4099 and #4105 synthesis comments and the LSP 3.18 conformance audit.

**Per-scorecard sub-issues** (each has a scout comment and a plan-reviewer comment that together are the evidence for that scorecard's wave placement):
- [#4063](https://github.com/EffortlessMetrics/perl-lsp/issues/4063) — Parser scorecard. **Wave 1.** Four of six metrics already computed; MVP is ~50 LOC in `xtask/src/tasks/update_status.rs`. Plan-reviewer extended with phase timings and slowest-file reports (#4099 takeaway 1).
- [#4065](https://github.com/EffortlessMetrics/perl-lsp/issues/4065) — Diagnostics scorecard. **Wave 2, gated on gold-corpus format.** Plan-reviewer produced the canonical `.gold.json` schema spec; adjudicated directory is `test_corpus/gold/<fixture-name>/` (see coordination resolution in main text).
- [#4066](https://github.com/EffortlessMetrics/perl-lsp/issues/4066) — Editor Intelligence scorecard. **Wave 2, same gate.** Plan-reviewer extended with developer instrumentation from #4099 (per-phase timings for hover / completion / goto-definition).
- [#4067](https://github.com/EffortlessMetrics/perl-lsp/issues/4067) — Module Resolution scorecard. **Wave 1, promoted from Wave 3.** Scout found position-aware lexical resolution already worked; plan-reviewer upgraded the MVP from a single test row to a 7×3 conformance matrix.
- [#4068](https://github.com/EffortlessMetrics/perl-lsp/issues/4068) — Workspace scorecard. **Wave 3, Option A picked over Option B.** Plan-reviewer caught the CI-dormant multi-root tests from PR #3984; scope bundled with `ci-workspace-multiroot` justfile recipe.
- [#4069](https://github.com/EffortlessMetrics/perl-lsp/issues/4069) — DAP scorecard. **Wave 3, PR 2 promoted to alpha-blocking.** Plan-reviewer pointed out that `dap_smoke_e2e.rs` already had `Instant` and `wait_for_event`; PR 2 is ~50 marginal LOC, not a new harness.
- [#4070](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) — Engineering Health scorecard. **Wave 1, scope-split.** Plan-reviewer caught the missing `--json` flag in nightly CI; filed [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106) to capture PRs 2-5 as a new umbrella.

**Research verifiers and new umbrellas:**
- [#4099](https://github.com/EffortlessMetrics/perl-lsp/issues/4099) — reference-model research (rust-analyzer `analysis-stats`, pyright `--stats`, clangd `$/memoryUsage`, gopls release-health). Drove the 7×3 conformance matrix shape for D, the xtask metrics framework, and the product-vs-execution separation.
- [#4105](https://github.com/EffortlessMetrics/perl-lsp/issues/4105) — 4-layer ratchet model: per-subsystem JSON artifact, floor vs. improvement metrics, re-baselined-wins ratchet, issue-to-scorecard ties. Provides the operational discipline that makes the sequence hold together across waves.
- [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106) — xtask metrics framework umbrella. Scope-split from #4070 PRs 2-5. Covers `cargo xtask metrics` subcommand tree, hierarchical memory accounting, release-health dashboard, product-vs-execution separation.

**First Wave 1 PR:**
- [#4124](https://github.com/EffortlessMetrics/perl-lsp/pull/4124) — parser scorecard + per-crate engineering-health metrics. Combines A (Parser) and G (Engineering Health test-count grouping) in one PR per the umbrella plan-reviewer recommendation.

**Related learning anchors:**
- [feedback_verify_before_build](../../.claude/memory/feedback_verify_before_build.md) — 42% of builders found work already done. The scorecard wave's "surfacing not measurement" aggregate pattern is a larger instance of this.
- [feedback_triage_false_positives](../../.claude/memory/feedback_triage_false_positives.md) — builders should be trusted over triage when they disagree. The session's plan-reviewers played the triage role correctly by enhancing rather than overriding scout evidence.

---

_Retrospective captured 2026-04-11 during the scorecard wave. Anchored to scout and plan-reviewer comments linked inline above. Every sequencing change traces to a specific, linkable reality-check pass. The final sequence is not the one the umbrella proposed; it is the one the evidence supports. Both are in the record, and both are legible in chronological order on the issue comment history._

_The sequence will change again. That is not a failure — it is the point. Every future scout wave on a new scorecard or a new ratchet pass on an existing one has license to revise the wave placements based on fresh evidence. The journey that produced this sequence is the template for the journey that will produce the next one._
