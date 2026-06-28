# Agentic Maintenance Field Notes: June 2026

*A technical narrative of three days of autonomous software maintenance — what the agents built, what they broke, how the system corrected itself, and what it changed about how we work.*

---

## Context

Between June 11 and June 13, 2026, the perl-lsp-swarm system ran an extended autonomous campaign: a sustained wave of parallel agents filing issues, specifying changes, writing tests, building implementations, reviewing PRs, and merging work. Forty-some PRs merged in roughly 72 hours. This document is the retrospective: not a changelog, but a reading of the patterns — what went well, what did not, and what the system learned about itself in the process.

The repo's documentation already contains the portable abstractions this campaign produced: `docs/concepts/shift-left-ladder.md`, `docs/concepts/cache-aware-agent-lanes.md`, `docs/concepts/hazard-class-invariants.md`, `docs/concepts/serialize-merges-and-cancellation.md`. Those documents strip out the specifics. This one keeps them. The incidents are the evidence.

---

## Theme 1: The Code's Bug and the Fleet's Bug Were the Same Shape

The campaign opened with two apparently unrelated fixes arriving in the same window. One was a parser bug: `qw`/`q`/`qq` delimiter parsing existed in five independent copies across the workspace — `perl-parser-core`, `perl-workspace`, `perl-semantic-analyzer`, `perl-module`, `use_lib/extract.rs` — and one of them was silently wrong. The `ImportExtractor` in `perl-semantic-analyzer` contained a comment asserting "the parser normalises all qw delimiters to parentheses" — a false premise that made the function skip non-parenthesis delimiters entirely. Users writing `use List::Util qw[first any]` would silently lose import tracking.

The fix (#1292) patched the broken copy. Then #1294 did the natural follow-up: centralize the five copies into a single canonical `parse_quote_operator_content` and `parse_qw_words` in `perl-parser-core`. The broken behavior that had persisted through four independent re-implementations disappeared by making it physically impossible to reimplement incorrectly — one place, one truth.

At the same time, the agent fleet was exhibiting the same failure mode at a larger scale. CI was pinned to `ripr` 0.5.0 in `.github/workflows/ripr.yml`. Local installs were on 0.9.0. The two versions disagreed about what counted as a coverage gap: 0.5.0 flagged predicate/string/closure seams that 0.9.0 did not. Builders were running local checks, seeing "0 seams, no suppression needed," pushing to CI, and getting gate failures. Each agent's model of the tool was divergent from the authoritative one — and because there was no conformance check, each agent ran confident in a false belief.

The parallel is exact. Five `qw` implementations, one silently wrong: the correctness lived in the code that happened to work, invisible to the copies that did not. Multiple agents with divergent ripr models, the wrong one producing false confidence: the correctness lived in the CI instance, invisible to local builds.

The fix for both problems was the same: centralize truth, then validate conformance against it. For the code: `perl-parser-core` becomes the single canonical `parse_quote_operator_content`. For the fleet: CI receipt becomes the authoritative ripr result, and the guardrail update (#1316) explicitly tells every agent "verify ripr against the CI `ripr+ New Gap Gate` receipt after push — never trust local ripr." Before that rule was written, agents had no reason to distrust their local toolchain. After it was written, they had an explicit instruction to prefer a specific external signal. Conformance-before-centralize is the pattern: you cannot centralize what you do not yet agree is the authority.

The `NodeKind` classification story is the same problem at a different layer. Six consumers — breakpoint validator, semantic analyzer, type inference, workspace indexer, LSP providers, DAP adapter — each maintained their own match-arm forests over NodeKind's 69 variants, each making different judgments about which variants were "executable" or "safe for breakpoint." When a new variant arrived (say, `Defer` or `Class`), some consumers updated and some did not. Issue #911 named the problem and proposed a solution: `NodeKindCategory` and `NodeKindFlags` in `perl-ast`, a single classification API that all consumers would query instead of re-implement. The architecture reviewer caught a real invariant error in the draft (the `safe_for_breakpoint` rule contradicted the mapping table for 20 variants), the plan-reviewer corrected it, and #1295 implemented the centralized classification. Downstream consumers now have one place to change when the semantics of a new variant are settled.

---

## Theme 2: Shift-Left as a Controlled Experiment

The most legible thing the campaign produced is a before/after measurement of a deliberate process change.

In the first part of the campaign, deep-review — the sonnet-model correctness pass near the end of the pipeline — was functioning as the primary catcher of recurring bug classes. Three PRs illustrate the pattern.

**#1219** (allocating variablesReference in the DAP evaluate handler): the implementation chose a base of 50,000 for its newly-allocated refs. Deep-review caught that this collided with the existing scope-ref encoding: `frame_id * 10 + scope_type`. The collision was silent — the debugger would silently serve the wrong variable container. The fix was correct, but the catch came after the builder had written tests, submitted the PR, and gone through review and maintainer passes.

**#1337** (clearing `stack_frames` on resume): there was a pre-existing test that asserted the broken behavior. The test was named `test_stack_trace_uses_recent_output_when_available` and it asserted `frames.len() >= 2` from snapshot buffer data. This was not a test of desired behavior; it was a test of what the buggy implementation happened to do. When #1337's builder arrived to fix the real bug (stale frames served between resume and next stopped event), the test resisted the fix. Deep-review had to identify that the test was encoding the defect rather than proving correctness, and the fix had to fight the test suite before it could pass.

**#1327** (coverage measurement): the xtask LCOV brace scanner was stripping `#[cfg(test)]` blocks from coverage measurement. It did this by scanning for matching braces. The problem: the scanner was blind to braces inside string literals, character literals, and comments. A production file containing a brace in a string or comment would have lines incorrectly stripped from LCOV, making coverage appear higher than it was — a measurement error that would pass CI while silently losing correctness signal. Deep-review found this before it caused an invisible CI problem.

These were real catches. But they were expensive catches — sonnet model, late in the pipeline, after significant builder work.

Then #1340 landed: "front-load hazard-class invariants into spec system." It named six recurring bug classes (ID/ref-space collision, bounds/overflow, protocol-safety, scanner literal/comment blindness, test-encodes-the-bug, coverage/measurement integrity), added them to `SPEC_UPDATE_CHECKLIST.md`, and instructed spec-planner and red-tdd to enumerate applicable classes as explicit acceptance criteria and adversarial tests before the builder starts.

The next PR to go through the full pipeline was #1227 (protocol-safe empty response for invalid variablesReference). Its deep-review comment read: "All hazard invariants verified. No logic bugs." Zero fix-forward items. The same reviewer, the same codebase, the same class of DAP handler fix — but the invariants were now in the spec, the red-tdd tests were adversarial by construction, and deep-review confirmed rather than discovered.

The merge log is the dataset. The #1340 commit is the before/after boundary. The shift is measurable.

---

## Theme 3: The Substrate-Model Was the Binding Constraint

Almost every significant time-sink during the campaign was not about code. The code was, mostly, smooth once specified. The time-sinks were about the orchestrator's model of the environment being wrong.

**The origin/master stall.** The PR-fast gate in `.github/workflows/ci.yml` invoked `xtask gates --tier pr-fast --base origin/master`. This repo's default branch is `main`. `origin/master` does not exist. The gate exited with code 128, fell back to an unscoped full-repo gate, found pre-existing fmt drift in `dap_edge_cases_test.rs`, and blocked every queue branch for approximately two hours (#1308). The fix was a one-line change: `origin/master` to `origin/main`. The two-hour stall was not a code problem.

The misstep that caused it was a stale assumption carried over from an upstream repo (which uses `master` as its default branch). Several agent definitions still referenced `origin/master` as of campaign start. The human correction that unblocked the queue was exactly one word. The guardrail update (#1316, item 5) then propagated that correction to all agent defs that contained the stale ref.

**The rapid-merge cancellation cascade.** Early in the campaign, several PRs were merged in rapid succession. GitHub's CI cancellation behavior: when a new merge fires, CI for the previous concurrent merge is cancelled. The result was a cascade of CI cancellations that looked like failures but were actually scheduling artifacts — the commits were correct, but their CI runs never completed. The mitigation: serialize merges in batches of three, waiting for each batch to complete before starting the next. This is documented in `docs/concepts/serialize-merges-and-cancellation.md` as a portable pattern, but the campaign was the experiment that confirmed it was load-bearing.

**The two-agents-one-branch tangle (#1309).** Issues #964 (stack frames not cleared on resume) and #933 (degraded transport path returning stale first frame) had accumulated four near-identical open PRs by mid-campaign: #1315, #1312, #1309, #1279. The cause was straightforward: agents were filing and building without checking for existing work. The source issue stayed open with no `in-build` label, so each new agent spawned saw the open issue and started fresh. PR #1309 additionally became entangled: it mixed the xtask ripr schema fix (pure infrastructure, should land independently) with the DAP behavioral change (needs its own test surface). The branch resisted review because reviewers were evaluating two unrelated changes that had different risk profiles.

The resolution was clean re-creation (#1337): read the spec, extract the cleanly-specified part (the xtask fix became #1335, the DAP fix became #1337), start fresh branches from main, rebuild. This was faster than untangling — and it avoided carrying any of the tangle's accumulated drift. `docs/concepts/re-create-over-untangle.md` captures this as a portable pattern.

**Cold-spawn vs warm-lane economics.** A verification pipeline with six sequential haiku passes (accuracy-scout, research-verifier, oppositional-planner, advocatus-diaboli, architecture-reviewer, maintainer-issue) runs each pass over the same large context: the spec, the issue body, the prior pass's output. Spawning six independent agents for this pipeline pays full cold cost for each. Keeping the context warm (completing one pass and immediately feeding the next within the cache window) costs roughly one-tenth per subsequent pass for the shared context portion. The multi-angle haiku spec-builder workflow that emerged from this campaign (`docs/concepts/multi-angle-haiku-early-spec.md`) is a fan-out pattern — the six angles are independent. But the sequential verification pipeline benefits from lanes. These are not interchangeable: fan-out for independent breadth, lanes for sequential depth over shared context.

---

## Theme 4: Two Independent Cost Models Converging on the Same Principle

Front-loading is the recommendation of two entirely separate optimization frameworks that arrived at the same answer independently.

**Bug-catch economics.** The shift-left ladder measures where in the pipeline a failure class is caught and what it costs. A deep-review catch (sonnet model, after full builder implementation) costs orders of magnitude more than a spec-level catch (a few lines in acceptance.md, before the builder starts). For a recurring class like ID/ref-space collision — which appeared independently in #1219 and would have appeared again in any future ref-allocating handler — the spec-level catch is a one-time cost. The deep-review catch recurs on every future PR that touches ref allocation.

**Token-cache economics.** The cache window is approximately five minutes. Cached tokens cost roughly one-tenth of fresh tokens. A verification pipeline that stays warm pays dramatically less per pass than one that cold-spawns. Front-loading verification (running cheap haiku passes early, before expensive sonnet builder work) keeps the cost of finding a problem cheap. Catching the same problem after a sonnet builder has written 500 lines means paying the deep-review cost on a large diff, plus the builder's re-implementation cost, plus the test-update cost.

When orthogonal optimizers — one measuring developer effort, one measuring token cost — agree on "front-load," the principle has support from two independent directions. The campaign produced explicit documentation of both (`docs/concepts/shift-left-ladder.md`, `docs/concepts/cache-aware-agent-lanes.md`) precisely because seeing them converge on the same answer made the principle more legible.

---

## Theme 5: The Instrument Kept Being the Bug, Recursively

The most recursive failure mode of the campaign was measurement systems that misrepresented what they were measuring.

**Codecov false-low (#1282).** The changed-file coverage pack ran `cargo test -p <crate> --tests` but the measurement filter `coverage_filters = ["workspace-lib"]` counted only `--lib` profdata. Integration tests in `crates/*/tests/` would run, exercise the changed lines, and produce profdata — but that profdata was discarded from the patch coverage calculation. PRs whose fixes were genuinely covered by integration tests (not lib unit tests) showed false-low patch coverage. The gate was measuring something real, but the wrong thing, in a way that punished correct behavior.

**ripr 0.5.0 pin vs 0.9.0 local (#1289, #1329).** The CI gate was pinned to ripr 0.5.0. Local installs were on 0.9.0. The two versions had meaningfully different seam detection — 0.5.0 flagged classes that 0.9.0 did not. Builders running local checks and seeing "0 seams" were measuring against a different instrument than CI. When they pushed, CI measured something different and failed them. The measurement disagreement was invisible until the CI artifact was read directly.

**ripr 0.9.x output schema break (#1335).** After the pin was bumped to 0.9.0 (#1329), a second layer of the same problem emerged: the xtask gate evidence parser read ripr's JSON output using 0.5.x field names (`classification`, `probe.file`). The 0.9.x output used different names (`grip_class`, `seam.file`). The parser silently skipped all findings it could not decode, treating unrecognized findings as having no classification — which meant it never applied the suppression rules that were supposed to exempt known-false-positive paths. `suppressed_by_policy` stayed at 0 even for findings that had matching suppression entries. The gate was over-strict in an invisible way.

**ripr suppression-application gap (#1346).** One layer deeper still: even after the schema break was fixed (#1336), a path-suppression check was positioned after a `continue` on unrecognized classification. A finding with `classification: "static_unknown"` or `"infection_unknown"` was counted in summary totals but never reached the path-suppression code. So `suppressed_by_policy` remained 0 for these findings even with a correct suppression entry in `policy/ripr-suppressions.toml`. The fix (#1349) moved the path suppression check before the continue — and added `raw-check.json` as a CI artifact so future diagnosis would have the raw findings available rather than only processed summaries.

**The meta-instrument: the orchestrator's own model.** Every example above was a case where the instrument being used to evaluate the system was itself wrong. The same was true of the orchestrator: its model of the system (which branch is default, how fast merges can go, which agents are working on which branch) was repeatedly the bottleneck. The human corrections during the campaign were, with near-complete consistency, corrections to the orchestrator's model of the environment — not to the code.

"Verify the instrument" is standard engineering discipline. The campaign demonstrated that in an autonomous system, the principle recurses upward: the measurement tools, the CI scripts, the gate parsers, and the orchestrator's own world-model are all instruments, and all can be wrong.

---

## Theme 6: Observability Emerged from Pain

The ripr suppression-application gap (#1346) was diagnosed blind. The CI artifact contained processed summaries — `severe_gaps: 4`, `suppressed_by_policy: 0` — but not the raw `findings[]` array from `ripr check --format json`. Without the raw findings, the exact field names in use, and the exact classification values being produced, diagnosis required code inspection and inference. This took substantially longer than it should have.

The fix to #1346 added observability as part of its resolution: `target/ripr/pr/raw-check.json` is now written by the CI job and included in the `target/ripr/pr/**` artifact glob. Future diagnosis starts with the raw findings. The friction of blind diagnosis was generative — it produced a durable improvement to the system's introspectability.

The same pattern recurred with learnings documentation. After the first set of deep-review catches (#1219, #1327, #1337), the patterns were visible in individual PR comments and issue bodies — but not in a searchable, cross-linked form. A future agent working on a similar handler would encounter the same class of problem without any signal that it had been encountered before. The two-layer learnings structure (#1342, #1344) was created explicitly to fix this: `docs/concepts/` holds portable patterns (no repo-specific terms, could drop into any agent-maintained codebase), and `docs/learnings/` holds repo-specific incidents with exact symbols, error strings, PR numbers, and hazard class cross-links. The search terms in each entry are chosen for greppability by future agents.

Observability improvements that emerge from debugging friction tend to be more durable than improvements added proactively, because they encode real information about what the next debugger will need.

---

## Theme 7: Human Corrects Substrate, Not Code

The human interventions during the campaign form a clean pattern when read together.

- `origin/master` to `origin/main` — branch name correction
- "Merging too fast, serialize to batches of three" — merge pacing correction
- "The agents are not checking for existing work before filing" — process gap
- "ripr local install is a different version than CI" — toolchain version alignment
- "The worktree agent stash is shared, never use git stash" — environment isolation
- "main must stay green; verify workspace-wide before merging" — scope of the green requirement

Not one of these was about the Perl parser, the LSP protocol, the DAP implementation, or any of the Rust code. Every human correction was about the environment in which agents run: branch naming, merge timing, agent coordination, toolchain versions, working-tree isolation, CI scope. The code was largely correct once built to spec. The environment was where reality disagreed with assumptions.

This is a meaningful signal about division of labor. Agents are effective at specifying, implementing, testing, reviewing, and merging code changes within a well-modeled environment. Humans are effective at correcting the environment model when it drifts — and at recognizing when the model has drifted from signals that are not legible to agents (a two-hour CI stall that looks like a code problem until you notice the branch name).

The implication for future campaigns: invest in environment legibility. The corrections that cost two hours (origin/master stall) could be near-zero cost if agents had an explicit preflight check against the live default branch name before writing any CI config. The corrections that generated duplicate PRs (four builds of the same fix) could be near-zero cost if agents had a mandatory search step before any `gh pr create`. Both of these were added as guardrails after the campaign; neither was there at the start.

---

## What Changed

The campaign was not only a wave of fixes. It produced durable changes to how future campaigns will run.

**Spec system formalized.** The spec template (`docs/reference/SPEC_TEMPLATE.md`), spec-builder workflow, and subsystem hazard defaults (#1340, #1347, #1348, #1391) mean that future specs start rich by default — with hazard classes enumerated, contracts pointed, and blast radius considered — rather than accumulating that information reactively through deep-review findings.

**Two-layer learnings structure.** `docs/concepts/` (portable, no repo specifics) and `docs/learnings/` (repo-specific, greppable incidents). The portability constraint on Layer 1 produced better abstractions — the discipline of stripping repo-specific terms forces higher-quality generalization.

**NodeKind centralized (#1295).** `NodeKindCategory` and `NodeKindFlags` in `perl-ast` replace six independent classification forests. When a new AST variant is added, there is one place to classify it, and exhaustive match arms enforce completeness at compile time.

**qw centralized (#1294).** Five copies of `parse_quote_operator_content` became one in `perl-parser-core`. The broken copy in `perl-semantic-analyzer` is gone. The conformance matrix tests (#1320, #1321, #1322, #1323, #1324) provide drift-guard coverage.

**Guardrails in agent definitions (#1316, #1318).** Duplicate-PR prevention (preflight search before filing or building), ripr verification via CI receipt not local, base-ref correctness (`origin/main` not `origin/master`), Codecov false-low recipe, verify-fix-premise discipline — all added to the relevant agent definitions.

**Parser contract index (#1317, #1319).** `docs/reference/PARSER_CONTRACTS.md` documents the parser's behavioral contracts with cross-links to learnings.

**Campaign scope guardrails.** Agent definitions now include autonomous-campaign guardrails: scope limits, deduplication requirements, environment verification steps.

---

## What Was Honest About the Missteps

The two-hour CI stall was preventable. The fix was one word. The word existed in the authoritative source (the GitHub UI showing the default branch name). The agents did not check it.

Four near-duplicate PRs for the same issue were opened and reviewed before anyone noticed. The duplication cost was not just the review effort on three rejected PRs; it was also the confusion about which PR was authoritative when deep-review and green-tdd had commented on multiple versions.

PR #1240 (protocol-safe errors for execution-control handlers) merged with a bug that deep-review had identified but the review had not yet resolved. This happened because the merge was triggered by three green CI checks passing before deep-review's in-flight analysis completed. The bug — `handle_pause` conflating session-presence with signal-delivery success — was real: a Perl debug session could exist, signal delivery could fail (zombie process, Windows event failure), and the handler would incorrectly report "no Perl debug session is active." The fix (#1364) had to be filed as a fresh PR after the fact. The lesson — that "3 green required checks" is a necessary but not sufficient merge signal when a synchronous review pass is in flight — was added to the learnings system (#1388).

Ripr suppression over-subtract in #1349's Path B: the fix to the suppression-application gap included a "no summary" fallback that zeroed out `suppressed_unclassified` rather than subtracting it, to avoid masking real gaps via saturating subtraction. Deep-review caught that this logic could nonetheless mask a real gap when `reachable_unrevealed` was less than `suppressed_unclassified`. The edge case was real and would have produced a silent false negative in the gate.

These are not catastrophes. None caused a production regression. But they were avoidable, and documenting them honestly is the premise of a learning system.

---

## Closing Note

The 2026-06 campaign is most useful as a demonstration that the system's failure modes and the code's failure modes have isomorphic structure — and that the same engineering instincts that fix the code (centralize truth, validate conformance, measure the right thing, front-load verification) also fix the system.

The code is debugged with the same tools as the system that debugs it. That recursion is not surprising, but it is clarifying: when the instrument is the bug, you need a meta-instrument. When the meta-instrument is the bug, you need the human who can step outside the system and correct the environment model. The campaign produced a cleaner picture of where that boundary lies.

---

*Related documentation:*
- *Portable patterns:* `docs/concepts/shift-left-ladder.md`, `docs/concepts/cache-aware-agent-lanes.md`, `docs/concepts/hazard-class-invariants.md`, `docs/concepts/serialize-merges-and-cancellation.md`, `docs/concepts/re-create-over-untangle.md`, `docs/concepts/multi-angle-haiku-early-spec.md`
- *Repo-specific incidents:* `docs/learnings/` (entries seeded from this campaign, plus backfill from #1388)
- *Lineage context:* `docs/reference/DISTRIBUTED_ENGINEERING_LINEAGE.md`
- *Pipeline gates:* `docs/reference/PIPELINE_GATES.md`
- *Spec system:* `docs/reference/SPEC_TEMPLATE.md`, `docs/agents/SPEC_UPDATE_CHECKLIST.md`
