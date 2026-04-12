# Layered Verification: Complementary Lenses in the PR Pipeline

> Process principle — not current state. Edit when the claim-type-to-layer mapping changes, not when metrics change.
>
> Sibling of [`verification.md`](verification.md). `verification.md` prescribes the merge gate (Tier A/B/C). This file prescribes which verification layers are mandatory for which classes of claims in a PR body.

## TL;DR

Each verification layer in the PR pipeline has a distinct **lens** — a specific class of wrongness that only that layer can see. Layers are additive in coverage, not redundant in compute. The pipeline's power comes from **diversity of lens**, not from multiplying inspections of the same kind.

When a class of wrongness can be held by multiple competent reviewers simultaneously — a shared blind spot — that class can only be caught by a layer with a different operating mechanism. Doubling the same kind of review does not help; adding a different kind does.

**Operational corollary**: for any PR whose body includes a claim type in the table in section 4, the matching verification layer is **mandatory** before `merge-ready`, not optional.

---

## 1. The principle

A verification layer is defined by its **operating mechanism** — what it checks and how. Two layers that share an operating mechanism can catch the same class of error but cannot catch each other's blind spots. A layer whose mechanism differs can catch wrongness the other layers cannot see, even when their conclusions are unanimous.

Concretely:

- **Banned-pattern scanning** (first-pass review) catches `unwrap()`, `panic!()`, missing tests, scope drift. It cannot catch a test that passes against a false premise.
- **Edge-case analysis** (deep review) catches logic errors, missed call sites, off-by-one bugs, vacuous assertions. It cannot catch a correctly-implemented false premise, because the logic is internally consistent.
- **Reference-implementation probing** (research-verifier) catches false premises about external systems (Perl runtime, LSP/DAP specs, crate APIs) by running the real thing. It cannot catch a banned `unwrap()` in the implementation.

These are not three stages of thoroughness on the same question. They are three different questions. A PR that passes all three has been challenged on three axes: **standards**, **internal logic**, and **external ground truth**.

The pipeline's value is the union of the lenses' coverages, not the intersection of their thoroughness. Two lenses that both catch standards violations do not compound. Two lenses where one catches standards violations and the other catches premise errors compound completely: neither could have caught what the other caught.

### The redundancy trap

A common failure mode in pipeline design is **apparent-diversity-without-actual-diversity**. Adding a second reviewer who looks at the same diff, reads the same PR body, applies the same style checklist, and reasons from the same documentation base as the first reviewer adds runtime without adding coverage. The second reviewer is shaped by the same training data, the same prior beliefs, and the same blind spots as the first. A consensus among N reviewers who share an operating mechanism is only N=1 worth of evidence about whether the code is correct — it is N=N worth of evidence about whether the reviewers share a misconception.

The fix is not "add more reviewers". The fix is "add a reviewer whose operating mechanism is different in a way that matters for this PR's claim set". The phrase *in a way that matters* is load-bearing: which mechanism matters depends on which claims the PR is making. This is what section 4 formalizes.

### Why "lens" and not "stage"

A pipeline stage is sequential — stage N runs after stage N-1 and consumes its output. A lens is parallel — multiple lenses look at the same artifact from different angles, and their outputs are independently informative. The PR pipeline has both characteristics, but the essential insight of this protocol is the *lens* property, not the *stage* property. Stages can be compressed (run in parallel, reordered) without losing correctness. Lenses cannot be merged without losing coverage.

---

## 2. The layers and their lenses

Evidence cells below are drawn from the 2026-04-11 session (see [session retrospective](../wisdom/2026-04-11-session-learnings.md)). Each example is verifiable against GitHub or the git log at the cited reference.

A note on reading the table: the "Lens" column is the important one. The operating mechanism column tells you *how* the layer works, but the lens column tells you *what class of wrongness would escape without it*. A reader deciding whether to add a new layer to the pipeline should be able to fill out the lens column for their proposal in one sentence; if they cannot, the layer is probably redundant with an existing lens.

| Layer | Operating mechanism | Lens — what only this layer can see | 2026-04-11 evidence |
|---|---|---|---|
| **Scout** | Broad discovery — reads code + issues + corpus, files findings | Problems that nobody has filed yet; capability gaps visible only by sweeping a subsystem | Scout sweep found 8 uncatalogued substrate items across refactoring, hover, completion, code actions, semantic tokens, inlay hints, benchmarks, and workspace index (see [session retrospective Pattern 1](../wisdom/2026-04-11-session-learnings.md)) |
| **Accuracy-scout** | Mechanical fact check against current master | File paths, line numbers, function signatures match the scout's claim; issue isn't already closed by a recent merge | Guard work in [#4102](https://github.com/EffortlessMetrics/perl-lsp/issues/4102) — audit of unwired test files that compile but never run; this is accuracy-checking applied to the test infrastructure surface |
| **Plan-reviewer** | Stress-tests the proposed approach against edge cases | Hidden premises in the spec, missing acceptance criteria, gaps between "tool supports X" and "CI invokes X with X" | [#4068](https://github.com/EffortlessMetrics/perl-lsp/issues/4068) multi-root workspace tests compiled but never ran in CI because feature flags and env vars were not wired into any justfile recipe — Pattern 2 case "not exercised" |
| **Research-verifier** | Runs the reference implementation — Perl, LSP spec, DAP spec, docs.rs — doesn't trust docs | False premises about external systems that every internal reviewer shares | [PR #4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) false phase-block pragma premise, caught in ~30 min by `perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'`; a [follow-up audit of 31 semantic claims](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) confirmed 30 were correct — the #4090 issue was isolated, not systemic |
| **Builder** | TDD red-green cycle, produces the diff | Implementation correctness against a specified test | Standard builder workflow — visible in every merged PR from the 2026-04-11 batch, e.g. [#4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089), [#4091](https://github.com/EffortlessMetrics/perl-lsp/pull/4091), [#4093](https://github.com/EffortlessMetrics/perl-lsp/pull/4093) |
| **Reviewer (first-pass)** | Scans diff for banned patterns, standards, test coverage, scope | `unwrap()`, `panic!()`, `expect()`, `todo!()`, missing tests, scope drift, formatting | [PR #4046](https://github.com/EffortlessMetrics/perl-lsp/pull/4046), [#4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052), [#4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089) all passed first-pass review against the banned-pattern checklist |
| **Reviewer-deep** | Traces logic under adversarial input, checks edge cases | Edge cases the builder missed, vacuous assertions, missed call sites | [PR #4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089) — deep review caught **two** correctness gaps the builder's audit had missed: the `\\?\UNC\server\share\...` UNC path case (stripping the generic `\\?\` prefix left an invalid `UNC\server\share\...` path) AND a **10th** call site in `perl.debugFile` in `misc.rs` that bypassed the normalization helper. Both fixed in commit `2b66aa3e` |
| **Ops** | Verifies the CI gate actually passes, watches for regressions introduced by the merge batch | Regressions that pass the PR's own CI but break the gate when combined with concurrent merges | [PR #4098](https://github.com/EffortlessMetrics/perl-lsp/pull/4098) — ops caught that [#4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052) and [#4088](https://github.com/EffortlessMetrics/perl-lsp/pull/4088) together produced a clippy `needless_borrow` error that neither PR's own CI exercised; autonomous hotfix merged to recover the gate |
| **Docs-sweep** | Verifies historical attribution claims via `gh pr view <N> --json closingIssues,title` | Git-history attribution errors — "fixed in PR #NNNN" claims that don't survive `gh pr view` | [Session retrospective Pattern 4](../wisdom/2026-04-11-session-learnings.md) itself contained the claim "#3472 `qw()` imports — shipped in #3808". Verification: `gh pr view 3808 --json closingIssuesReferences,title` returns an empty `closingIssuesReferences` array and title `(#3466)`, not `(#3472)`. The actual fix was direct-to-master commit [`a114059c`](https://github.com/EffortlessMetrics/perl-lsp/commit/a114059c) (`fix(semantic): resolve parens-list imports and cross-file use constant (#3472, #3475)`). Issue [#3472](https://github.com/EffortlessMetrics/perl-lsp/issues/3472) remains OPEN because the commit landed outside a PR and the "Closes #3472" trailer was not honored |
| **Wisdom** | Reads the full issue->PR->merge trail, synthesizes cross-cycle patterns | Patterns across multiple PRs that no individual PR review could see | The [2026-04-11 session retrospective](../wisdom/2026-04-11-session-learnings.md) identified 7 durable patterns including the shared-blind-spot pattern (Pattern 3) and the research-verifier ROI pattern (Pattern 7) — this protocol is the formalization of both |

Every cell in the "Lens" column describes a kind of wrongness that **cannot** be caught by any of the layers above or below it. That is what makes the table additive.

### How to test whether a proposed new layer adds coverage

The test is **the counterfactual-blind-spot check**. For a proposed new layer L, ask:

1. *What class of wrongness can L see that no existing layer can see?*
2. *Construct a concrete failure mode in that class. Would every existing layer approve the failing artifact?*
3. *Would L reject it?*

If the answer to all three is yes, L is additive. If any existing layer would also reject the artifact, L overlaps with that layer — possibly redundantly. A new layer does not have to be *uniquely* the only catcher for every failure; it is sufficient for L to be uniquely the only catcher for at least one failure class that would otherwise merge.

This is how the docs-sweep layer was added during the 2026-04-11 session. The counterfactual check was: "would scout, accuracy-scout, plan-reviewer, builder, first-pass reviewer, deep reviewer, research-verifier, and ops all approve a PR body that says 'fixed in PR #NNNN' where NNNN actually fixed a different issue?" Yes, all of them would. None of their operating mechanisms involve running `gh pr view` on cited PR numbers. Docs-sweep adds exactly one kind of catch that nothing else catches. It is additive.

---

## 3. The clearest example: PR #4090

The false phase-block pragma PR is the clearest single demonstration of why diversity-of-lens matters more than depth.

**The claim in the PR body**: "per Perl semantics (perlmod, perlop), phase blocks are compile-time-transparent: pragmas declared inside propagate to the surrounding file scope."

**What each layer did**:

1. **Scout** documented the premise by citing `perlmod`. The citation was real. The inference drawn from it was wrong.
2. **Builder** wrote 6 tests against the premise. Tests were non-vacuous and green.
3. **First-pass reviewer** scanned the diff. No banned patterns. Standards clean. **Approved.**
4. **Deep reviewer** traced parser invariants, recursion paths, range tracking, verified the tests were not vacuous. The code was internally consistent with the stated premise. **Approved. `reviewed-deep` label set. `merge-ready` set.**
5. **Research-verifier** ran `perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'`. Output: `ok`. `strict` was not active outside the `BEGIN` block. **The premise was false.** Pragmas in phase blocks are lexically scoped. Every prior approval rested on a misconception none of the prior layers could challenge.

All four prior layers shared the same false mental model of Perl semantics. **More of any of them would have approved the same wrong code with the same confidence.** Scaling up the number of first-pass reviewers, adding a second deep reviewer, re-running the tests, or letting CI run longer would have done nothing. The consensus was an illusion of correctness built on a shared wrong premise.

Only a layer with a different operating mechanism — one that runs the reference implementation instead of reasoning from documentation — could break the illusion. The research-verifier's cost was one `perl -e` invocation taking under a minute.

The aftermath is recorded in [issue #4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100): the PR was closed without merge, the already-merged [#4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052) workaround was tracked for correction, 9 tests were rewritten, and [issue #4101](https://github.com/EffortlessMetrics/perl-lsp/issues/4101) was filed for the correct positive-direction lint. A proactive audit of 30 other semantic claims confirmed the issue was isolated, not systemic.

**Generalization**: when the premise is wrong, internal-consistency checks cannot find the error. They can only confirm that the implementation matches the premise. Only a layer that can challenge the premise itself — by running the reference implementation — can.

### The 9-minute margin

The research-verifier caught #4090 approximately 9 minutes before the scheduled ops merge window. The margin was coincidental, not structural — a different ordering of the day's events would have put #4090 into the merge batch before the verifier's dispatch. That *this* incident was caught in time is not evidence that *the next* incident will be. The margin was luck; the process change in section 6 removes the dependence on luck.

### Why deep review did not suffice

It is tempting to say "we just need better deep reviewers". The deep reviewer on #4090 did their job correctly. Their analysis was technically thorough: parser invariants checked, recursion paths traced, range tracking verified, test non-vacuity confirmed. None of that work requires running Perl. The deep reviewer trusted documentation (perlmod citation) over empirical verification because deep review's operating mechanism is *code analysis*, not *runtime probing*. Changing deep review's operating mechanism to include runtime probing would make every deep review longer and more expensive, and would still miss claims that only research-verifier's broader scope (LSP spec, DAP spec, crate APIs, docs.rs) can cover.

The correct fix is not "make deep review more thorough". It is "dispatch a different lens alongside deep review, automatically, for the claim types where runtime probing is needed". That is what section 4 prescribes.

### Why not one reviewer with all the operating mechanisms?

A reasonable alternative would be to build a single super-reviewer that runs code analysis, reads the spec, runs the reference implementation, checks banned patterns, and traces edge cases. In practice this concentrates all the blind spots into one agent's context window and one agent's reasoning chain. If that agent shares a blind spot with the builder, the blind spot is preserved. The power of separate lenses is that they are *separately instantiated* — their reasoning does not coordinate with the reasoning that produced the error. This is a form of noise injection into the review process, and the diversity is what produces the signal.

The analogy is ensemble methods in machine learning: a single large model is not equivalent to an ensemble of smaller specialized models even if both cost the same, because the ensemble's errors are less correlated. Pipeline lenses work the same way.

---

## 4. Operational implication: claim-type-to-layer mapping

The operational question on every PR is no longer "should we review this?" but **"which specific lenses does this PR need?"** The answer is determined by the claims in the PR body.

| Claim type in PR body | Mandatory layer | Why |
|---|---|---|
| Cites `perlmod`, `perlop`, `perlfunc`, `perlref`, `perlsyn`, `perlvar`, or any Perl language semantics | **Research-verifier running `perl -e`** | Internal reviewers cannot challenge a shared false belief about Perl. Only running Perl can. |
| Cites LSP specification sections (e.g. "per LSP 3.17 `textDocument/hover`…") | **Research-verifier checking the spec text** | Spec behavior drifts version-to-version; docs-cached memory is stale |
| Cites DAP specification sections | **Research-verifier checking the spec text** | Same reason as LSP |
| Cites a crate API from `docs.rs` | **Research-verifier checking current `docs.rs`** | Crate APIs change between versions; `docs.rs` for the pinned version is the source of truth |
| Claims "fixed in PR #NNNN" for historical attribution | **Docs-sweep: `gh pr view NNNN --json title,closingIssuesReferences`** | Attribution claims decay as issue numbers get reshuffled between PRs; see the docs-sweep row in section 2 |
| Touches production code | **Builder + reviewer + reviewer-deep** | Standard pipeline (see [`verification.md`](verification.md) Tier A) |
| Touches CI/CD pipelines | **Ops check of actual pipeline run**, not just CI green | `just ci-gate` passing locally does not prove the cloud gate passes; see [#4102](https://github.com/EffortlessMetrics/perl-lsp/issues/4102) |
| Adds a feature claimed in `features.toml` | **Accuracy-scout verifying the claimed tests and handlers exist** | Catalog entries routinely outrun implementation; the inverse (implementation outrunning catalog) is also common — see [session retrospective Pattern 1](../wisdom/2026-04-11-session-learnings.md) |
| Cites a metric (coverage %, corpus %, test count) | **Accuracy-scout verifying the computed value** | Hand-edited metrics drift; truth-source is the computation, not the PR body |

A PR whose body falls into one of the rows above is **not merge-ready** until the mandatory layer has produced a verification receipt. This is stronger than "should be reviewed by"; it is **gate-blocking**.

Reviewers before the mandatory layer should scan the PR body for claim types in this table and request the missing layer *before* making an approval decision, not after. Approving first and verifying second is what produced the [#4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) near-miss.

### Exempt PRs

Not every PR needs every lens. A PR that:

- Touches only `README.md` or other root documentation
- Touches only a single `docs/` file
- Touches only a comment line
- Bumps a test timeout constant with no logic change
- Updates a changelog entry

...does not need research-verifier, accuracy-scout, or docs-sweep unless its body happens to cite an external claim. The default pipeline for docs-only PRs (see the [docs-only fast-track gate in #4097](https://github.com/EffortlessMetrics/perl-lsp/issues/4097)) is intentionally thinner. The layered-verification protocol is about **selecting** the lenses a PR needs, not about running them all always.

The risk of under-selecting is a merged error. The risk of over-selecting is a slow pipeline. The claim-type-to-layer table is the mechanism for balancing the two — selection is driven by what the PR body *claims*, not by a PR's size or urgency.

### Receipts, not trust

Each mandatory layer produces a **receipt** — an artifact (label, comment, or both) that proves the layer ran and what it concluded. The receipt convention on this repository is:

- **Labels** — `research-verified`, `reviewed-deep`, `merge-ready`, and the reserved `accuracy-reviewed` (see [`CLAUDE.md`](../../../CLAUDE.md) — reserved for the accuracy-scout agent, issue [#2628](https://github.com/EffortlessMetrics/perl-lsp/issues/2628)) — are set by the agent that performed the check. See the label list in [`../../../CLAUDE.md`](../../../CLAUDE.md) for the canonical definitions.
- **Comments** — the agent posts a structured comment on the PR or issue summarizing what it verified, what evidence it used, and what remained unverified.
- **Commits** — when the layer's output is a code fix, the commit message cites the lens that caught the issue. Example: commit `2b66aa3e` on [#4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089) explicitly says "Two correctness fixes found during deep review".

Receipts matter because trust between layers is transitive and falsifiable. If the plan-reviewer's receipt says "verified against master", the builder trusts it and acts. If the receipt is stale (the master it verified against has moved), the receipt is invalid and must be re-produced. A pipeline built on unreceipted trust silently accumulates stale assumptions. A pipeline built on receipts has auditable state.

The `label-receipt-validate` and `label-receipt-write` skills in this repository implement the receipt convention for labels. Every mandatory layer in section 4 should produce a receipt; receipts are the interface between layers.

---

## 5. Cost-vs-benefit math

Each verification layer is cheap per-run — roughly 5 to 30 minutes of agent compute. Running the full stack on a single PR costs on the order of 1 to 2 hours. This is the upper bound; most PRs need only a subset. The costs below are agent-compute costs, not wall-clock costs; parallelism reduces wall-clock by a factor of 2-4 in typical cases.

Compare to the cost of the failure modes the layers prevent:

| Failure caught | Layer that caught it | Cost avoided | ROI |
|---|---|---|---|
| [#4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) false-premise pragma cascade | Research-verifier | ~2-3 builder slots + a half-day elapsed time to revert the PR, revert the companion [#4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052) workaround, rewrite 9 tests, update doc comments, notify downstream scorecard work | 8-16x conservatively |
| [#4098](https://github.com/EffortlessMetrics/perl-lsp/pull/4098) clippy regression (cross-PR interaction) | Ops | CI gate stays red until someone notices and investigates; blocks the entire merge queue | Infinite (a red gate halts all pending merges) |
| [#4068](https://github.com/EffortlessMetrics/perl-lsp/issues/4068) multi-root tests compiled but never ran | Plan-reviewer / accuracy-scout | Silent: the tests would have counted as "wired" in every subsequent coverage calculation while never producing signal | Infinite (silent failures never self-detect) |
| [#4070](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) `cargo mutants` CI invocation missing `--json` | Plan-reviewer | Scorecard PR would silently return a placeholder mutation-coverage value forever | Infinite (same — silent) |
| Session-retrospective `#3808/#3472` attribution error | Docs-sweep | Future scouts re-read the retrospective and waste a scout slot re-investigating a still-open issue | 3-5x (saves 1-2 future scout cycles from repeating the mistake) |

The pattern: **layered verification's ROI is highest on silent-failure PRs** — PRs that pass every cheap gate and produce wrong-but-plausible output. Cheap gates catch cheap errors. The layers whose job is to *prove the premise* catch the errors the layers whose job is to *check the implementation* cannot.

The inverse is also true: running the full stack on a docs-only PR that changes one sentence is wasteful. The claim-type-to-layer mapping in section 4 is the mechanism that selects which layers are mandatory for any given PR.

### The "infinite ROI" cells

Three rows in the table above are marked *infinite* ROI. This is not rhetorical. A silent failure is not just "a bug we haven't found yet" — it is "a bug that the feedback loop has no mechanism to ever find". Consider the [#4070](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) case: `cargo mutants` is invoked in CI without `--json`, so the structured output that would allow the scorecard to compute a per-crate mutation score is never written. CI is green. The scorecard runs. It reports a placeholder value. The placeholder is written to `docs/project/status/`. The next release cites the placeholder in the announcement. Nobody ever knows the real value. There is no moment at which any layer of the pipeline could self-detect the gap — because the gap is in the shape of the data the pipeline operates on, not in the data itself.

Only a layer that asks "for every tool invoked in CI, does the invocation use the tool's structured-output flags?" can catch this. That layer is the plan-reviewer, acting on knowledge of the tool's capabilities. It is not a standards review (no banned pattern), not an edge-case review (no edge case exists), not a runtime check (the runtime behavior is fine). It is a **meta-question about whether the measurement surface is wired**. The plan-reviewer's lens includes this question; no other lens does.

The lesson: silent failures are the error class most vulnerable to pipeline blind spots, because they do not fail loudly when unverified. A cheap gate that never runs produces no red CI signal. A verification step that asks "is the measurement wired at all?" costs ~10 minutes and prevents a class of error that would otherwise persist indefinitely.

### Parallel execution and wall-clock cost

The full stack is ~1-2 hours of *agent compute*, not ~1-2 hours of *wall-clock time*. Layers that do not depend on each other's outputs run in parallel. The 2026-04-11 session ran first-pass reviewer, deep reviewer, and research-verifier concurrently on at least one PR. The bottleneck is typically sequencing, not compute — some layers must run post-merge (wisdom), some must run post-gate (ops), and some must run pre-approval (all the rest). The wall-clock overhead for a fully-layered PR is ~30-45 minutes when parallelism is exploited, not ~1-2 hours. This is acceptable for PRs that would otherwise merge a silent failure.

For PRs that are not at risk of silent failure — docs-only changes, test timeout bumps, version bumps — the fast-track path skips layers that have no mandatory claim triggers in the PR body. Throughput is preserved for the common case; the full stack is reserved for the high-risk case. This selectivity is the protocol's efficiency guarantee.

---

## 6. What to do structurally

Three structural changes codify this protocol.

### 6.1 Codify the claim-type-to-layer mapping

The `CONTRIBUTING.md` addition in [PR #4118](https://github.com/EffortlessMetrics/perl-lsp/pull/4118) adds the reference-implementation rule for external-semantics citations. This is the first row of the section 4 table formalized in contributor-facing documentation.

The remaining rows — historical-attribution, features.toml-claim, CI-invocation, metric-citation — should follow the same pattern: a one-subsection rule in `CONTRIBUTING.md` and a corresponding checkpoint in the matching agent's decision checklist.

Agent-file edits require the control-plane lock (see [`.claude/commands/control-plane-lock.md`](../../../.claude/commands/control-plane-lock.md)). The blocking issue for the reviewer-deep checklist edit is [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111).

### 6.2 Treat docs-sweep as a first-class layer

The git-history attribution failure mode (section 2 docs-sweep row) is a new class of error this session uncovered. It does not fit any existing layer's lens:

- Scout cannot catch it — scouts *create* attribution claims.
- First-pass reviewer cannot catch it — attribution claims don't trip banned-pattern scans.
- Deep reviewer cannot catch it — the PR's internal logic is unaffected by a wrong citation.
- Research-verifier cannot catch it — it's not an external-system claim.

Docs-sweep's operating mechanism is one command: `gh pr view NNNN --json closingIssuesReferences,title`. The `scout-dedup` skill should include this verification for any cited PR number before accepting it. The `wisdom-document` skill should include the same check before writing a retrospective that cites PR numbers.

### 6.3 Make the claim-type-to-layer mapping machine-checkable

A pre-review hook that reads PR body text and labels the required verification layers (`needs-research-verifier`, `needs-docs-sweep`, `needs-accuracy-scout`, etc.) removes the reviewer's "did I remember to scan for this" step. Reviewers then check that the labels are satisfied before approving.

Implementation shape:

- Parse PR body for regex patterns corresponding to each row of the section 4 table (`\bperl(mod|op|func|ref|syn|var)\b`, `\bLSP 3\.\d+\b`, `\bdocs\.rs\b`, `\bfixed in PR #\d+\b`, `\bfeatures\.toml\b`, etc.).
- For each match, apply a label like `needs-research-verifier`.
- `reviewer-deep-decide` refuses to set `merge-ready` if any `needs-*` label is present without a corresponding `*-verified` label.

This is a pure-CI addition, no agent file edits required. Track as a follow-up to [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111).

### 6.4 Add new rows as new failure classes are discovered

The table in section 4 is a **living taxonomy**, not a closed set. Each new failure mode that a session uncovers is evidence either that an existing row needs a broader trigger pattern, or that a new row is needed. The 2026-04-11 session contributed the **docs-sweep** row; before that session, git-history attribution errors were uncategorized.

The process for adding a row is:

1. **Identify the failure class** — a specific kind of wrongness that slipped through the existing pipeline.
2. **Counterfactual check** (section 2) — confirm that no existing lens would have caught it.
3. **Propose the trigger** — what pattern in a PR body (or other artifact) signals that the new lens is needed?
4. **Propose the operating mechanism** — what does the new lens actually *do* to verify the claim?
5. **Document the receipt** — what label or comment proves the new lens ran?
6. **Add the row** to section 4 via a normal PR, cross-referencing the incident that motivated it.

The protocol is self-improving: every caught incident teaches something about the shape of the pipeline's blind spots, and the shape of the pipeline's blind spots is exactly what the section 4 table represents.

### 6.5 Do not retrofit lenses to already-merged PRs retroactively

When a new lens is added, it applies to future PRs. Going back and auditing merged PRs against the new lens is tempting but expensive and rarely productive — the distribution of merged-PR errors is weighted toward the errors the prior pipeline could catch, so retroactive audits have poor yield. The exception is when a specific incident strongly suggests a recent merge is contaminated by the same error class (as with [#4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) and the already-merged [#4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052) workaround); targeted audit of the suspect region is warranted, but full-pipeline retrofits are not.

### 6.6 Anti-patterns to avoid

When implementing or tuning this protocol, three anti-patterns are worth naming explicitly so they are recognized and rejected:

**Anti-pattern 1: "Just add another reviewer".** When a PR has a tricky claim, the instinct is to dispatch a second reviewer of the same kind. This doubles the compute cost and halves the independence of the verdict — if both reviewers share a training base or a reasoning style, they are not two samples, they are one correlated sample. The correct response is to dispatch a reviewer of a *different* kind.

**Anti-pattern 2: "Trust the label, don't read the receipt".** A label like `research-verified` is a summary; the underlying comment is the evidence. When a merging operator trusts the label without checking the comment, stale labels start slipping through. The `label-receipt-validate` skill enforces freshness for labels, but the comment content is only checked by humans. If the comment says "I looked at perlmod and concluded X" instead of "I ran `perl -e '...'` and got output Y", the receipt is weaker than its label implies, and the reviewer should flag it.

**Anti-pattern 3: "Apply the protocol uniformly".** Every PR does not need every lens. A PR adding a test for an already-merged fix does not need research-verifier. A docs-only PR does not need deep review. Applying every lens to every PR is the noisiest possible policy; it trains everyone to ignore lens rejections because most of them are spurious. Selectivity is the mechanism that keeps the protocol credible. Over-application is not more safety; it is *less* safety, because it desensitizes reviewers to real rejections.

**Anti-pattern 4: "Trust the wisdom retrospective without re-verification".** The session retrospective is itself a document that can contain errors. The #3808/#3472 attribution error in the 2026-04-11 retrospective is the canonical example. Retrospectives should be treated as high-quality draft material that still benefits from a docs-sweep pass on any PR numbers they cite. A cited PR number in a retrospective is not an authoritative claim until `gh pr view` confirms the title and the closing references.

**Anti-pattern 5: "Make the plan-reviewer check everything".** Expanding one lens to cover another lens's job is the lazy version of section 6.4. The plan-reviewer already has a heavy workload; adding research-verifier responsibilities to the plan-reviewer dilutes both lenses. Separate lenses are separate because their operating mechanisms are different; merging them destroys the diversity property that makes layered verification work.

### 6.7 What success looks like

The protocol is working when:

- No PR in a given month produced a merged silent failure that a known lens should have caught.
- The claim-type-to-layer table added at most one new row in the month (meaning the taxonomy is converging).
- The wall-clock overhead for full-stack PRs stayed under 45 minutes.
- At least one PR was rejected by a mandatory lens (meaning the lenses are actually running and actually catching things).
- The receipts on merged PRs were all fresh at merge time (no stale-label slippage).

The protocol is failing when:

- Silent failures land on master and require retrospective revert.
- The table grows by more than 2-3 rows per month consistently (suggesting the baseline pipeline is too thin and lenses are being bolted on reactively).
- Reviewers routinely override mandatory-lens rejections instead of fixing them.
- Receipts are stale at merge time because nobody re-dispatches the lens after master moves.
- Authors stop reading their own PR bodies for claim types because the mapping feels bureaucratic.

Early signs of failure are easier to catch than late signs; the monthly wisdom retrospective is the natural place to observe them.

---

## 7. Open questions

These are unresolved and explicitly marked as such. Do not treat them as prescriptive.

**How many layers is too many before the pipeline becomes the bottleneck?** The 2026-04-11 session ran 9 distinct layer types on at least one PR each. Wall-clock throughput stayed acceptable because layers ran in parallel where possible. The diminishing-returns curve has not been measured. A reasonable upper bound is "until two consecutive additions catch zero new classes of error in 30 consecutive PRs." At that point the cost of additional layers probably exceeds their catch rate, and the pipeline should be pruned rather than extended. This metric — catches-per-layer-per-30-PRs — is the natural measurement surface for evaluating the protocol's ongoing health.

**Can layers be run in parallel safely, or do some have ordering dependencies?** First-pass review, deep review, and research-verifier can run in parallel on the same diff — they consult different surfaces and their outputs are independent. Ops must run last because it verifies a merge, not a diff. Docs-sweep has no ordering dependency. Wisdom runs post-merge only. Accuracy-scout has a soft dependency on scout (it verifies scout's claims) but can run against any artifact that cites file paths. The explicit dependency graph should be documented before automation.

**What is the right escalation path when two layers disagree?** Observed during the 2026-04-11 session: two plan-reviewers dispatched on the same gold-corpus question returned conflicting file-location recommendations. The orchestrator picked manually. A general rule — "escalate to the layer whose operating mechanism is most orthogonal to both disagreeing layers" — is intuitive but unvalidated. When two layers of the *same kind* disagree, the disagreement is evidence that the operating mechanism itself is insufficient for the question, and the answer is to invoke a layer of a different kind. When two layers of *different kinds* disagree, the disagreement is evidence about the artifact, not the layers, and both disagreements should be treated as data rather than noise. The session retrospective should log disagreement events as training data for the protocol.

**Does the claim-type-to-layer mapping have false negatives?** PRs with claims that don't fit any row of the section 4 table get the default pipeline. If a new failure mode appears that the default pipeline cannot catch, a new row should be added. The appearance of a new row is evidence of a newly-discovered blind spot class; tracking the row history is itself a measurement of pipeline maturity. A reasonable measurement window is "how long has it been since the last new row was added?" — increasing intervals suggest the taxonomy is converging on completeness; persistent short intervals suggest the pipeline is still exploring.

**How do lenses degrade over time?** A lens whose operating mechanism depends on an external resource (e.g. research-verifier depends on `docs.rs` being reachable and up to date) can silently drift from accurate to inaccurate as the external resource changes. The protocol currently has no periodic lens-health check. A candidate approach: run each lens against a known-passing reference PR once per week and confirm the expected verdict. If the verdict drifts, the lens has drifted. This is an open implementation task.

**Can lenses be composed?** If research-verifier verifies a perlmod claim and the same PR also makes a docs.rs claim, can the same agent invocation handle both, or should they be separate invocations? Composition saves setup cost but risks reasoning-chain contamination (the agent's analysis of claim A may bias its analysis of claim B). The conservative default is separate invocations; the aggressive default is composition with explicit instructions to reason independently about each claim. More empirical data needed.

**Do lenses have user-visible failure modes?** When a lens rejects a PR, the rejection should include an actionable explanation that the author can act on. A lens that says "rejected" without saying *why this specific lens rejected it* provides the author with no path to repair. The receipt convention (section 4) requires a structured comment; the format of that comment is currently informal. A template — what the rejection said, what evidence was used, what the author should change — would make the protocol's feedback loop tighter.

---

## 8. A reviewer's quick reference

A practical checklist for applying this protocol on a specific PR. Use this before setting `merge-ready`.

**Step 1: read the PR body.** Identify every claim. Write them down if necessary.

**Step 2: classify each claim.** For each claim, find the row in the section 4 table whose trigger matches. If no row matches, the claim goes through the default pipeline.

**Step 3: check the receipts.** For each matched row, verify that the mandatory layer's receipt is present on the PR — label, comment, or both. If a receipt is missing, dispatch the layer before approving.

**Step 4: confirm receipts are fresh.** A stale receipt (produced against an older master) is not valid. The `label-receipt-validate` skill implements the freshness check.

**Step 5: check for uncategorized claims.** If the PR makes a claim that felt important but didn't match any row, note it in the review comment and consider whether a new row should be added (section 6.4).

**Step 6: approve or request changes.** Approval means: standards pass, internal logic passes, all mandatory lenses have fresh receipts. Any of these failing is grounds to request changes; do not "approve with reservations" — dispatch the missing lens instead.

This checklist is deliberately simple. The complexity is in the table, not in the procedure — once the table is right, applying it is mechanical.

### Self-review question for authors

Before opening a PR, the author should ask: **if I read this PR body and I was a reviewer, which lenses would I say this PR needs?** If the answer is "I'm not sure" or "probably none", read the section 4 table again. Common author traps:

- *"I'm citing perlmod to explain why my change is correct, not to assert a behavior."* — Doesn't matter. The citation is in the PR body; the lens should fire.
- *"The docs.rs link is just for the reader's convenience."* — Doesn't matter. The link is in the PR body; the lens should fire.
- *"This PR is tiny; surely it doesn't need the full stack."* — The full stack is not mandatory for tiny PRs; but any tiny PR whose body makes a claim in the table needs the lens that matches that claim. Size does not exempt a PR from claim-type verification.
- *"The claim is obviously true; everybody knows it."* — This is exactly the shared-blind-spot scenario. If everybody knows it, the lens is needed, because everybody may be wrong together.

The protocol's purpose is to make the reviewer's job auditable. If authors do the self-review step, reviewers find receipts where they expect them, and approvals are faster, not slower.

---

## 9. Glossary

**Lens** — an operating mechanism by which a verification layer checks a PR. Two layers with the same lens cannot catch each other's blind spots. Two layers with different lenses can.

**Shared blind spot** — a class of wrongness that multiple competent reviewers can hold simultaneously because they share a premise, a training base, or an operating mechanism. The canonical fix is a layer with a different operating mechanism, not more layers with the same mechanism.

**Operating mechanism** — what a layer does to verify a claim. Examples: banned-pattern scanning, edge-case tracing, reference-implementation probing, `gh pr view` on cited PRs, metric re-computation.

**Claim type** — a class of assertion in a PR body that maps to a mandatory verification layer. Enumerated in section 4.

**Receipt** — an artifact (label, comment, commit trailer) proving that a layer ran and what it concluded. Receipts make inter-layer trust falsifiable.

**Counterfactual-blind-spot check** — the test for whether a proposed new layer is additive. "Would every existing layer approve this failing artifact? Would the new layer reject it?" If both are yes, the new layer adds coverage.

**Silent failure** — a PR that passes every cheap gate and produces wrong-but-plausible output, with no loud failure signal. Silent failures have the highest ROI for layered verification because no other mechanism catches them.

**Docs-sweep** — the layer that verifies historical PR-number citations against `gh pr view`. Added during the 2026-04-11 session after discovering the `#3808/#3472` attribution error in the session retrospective itself.

**Research-verifier** — the layer that runs the reference implementation (Perl runtime, LSP spec text, DAP spec text, docs.rs) to check external-semantics claims. Canonically dispatched on any PR citing perlmod/perlop/perlfunc/LSP/DAP/crate APIs.

---

## 10. Invariants

These invariants are stated prescriptively. A PR violating any of them is not merge-ready regardless of CI state.

1. **Every row in section 4 has a receipt on any PR whose body matches its trigger.** Receipts are labels, comments, or commit trailers; any form is acceptable as long as it is machine-readable and post-merge auditable.
2. **A receipt is valid only against the master it was produced against.** A stale receipt is not evidence. Re-dispatch the layer if master has advanced materially.
3. **A rejection from any mandatory lens is gate-blocking.** Reviewers cannot override a research-verifier rejection with a standards approval, and vice versa — the lenses are independent, and all must clear.
4. **New rows are added only with a documented counterfactual failure.** Section 6.4 governs the addition process; rows added without evidence are vanity extensions to the pipeline and should be rejected.
5. **Docs-only PRs use the thin path.** The full stack is not mandatory for docs-only changes unless the docs body itself contains a claim-type trigger.
6. **The `CLAUDE.md` label list is the canonical source of truth for receipt names.** If this protocol and `CLAUDE.md` disagree on a label name, `CLAUDE.md` wins and this file should be updated.

---

## Cross-references

- [`verification.md`](verification.md) — the merge-gate protocol (Tier A/B/C). This file is its complement: Tier A/B/C defines *what a passing gate looks like*; layered verification defines *which layers must have run before the gate is meaningful for this particular PR*.
- [`../wisdom/2026-04-11-session-learnings.md`](../wisdom/2026-04-11-session-learnings.md) — the session retrospective that generated the evidence for this protocol. Pattern 3 (shared blind spots) and Pattern 7 (research-verifier ROI) are the sources for this doc's principle; this doc formalizes them as the protocol those patterns imply.
- [`../../../CONTRIBUTING.md`](../../../CONTRIBUTING.md) — the external-claim verification rule is being added via [PR #4118](https://github.com/EffortlessMetrics/perl-lsp/pull/4118).
- [`../../../CLAUDE.md`](../../../CLAUDE.md) — the pipeline state label list, the orchestrator model, and the canonical definitions of each stage.
- [Issue #4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111) — the mandatory-research-verifier tracker; blocked on reviewer-deep agent file edit permissions.
- [Issue #4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) — the #4090 cascade write-up.
- [Issue #4102](https://github.com/EffortlessMetrics/perl-lsp/issues/4102) — test-wiring regression guards; addresses the CI-wiring gap that plan-review caught.
- [Issue #4101](https://github.com/EffortlessMetrics/perl-lsp/issues/4101) — the correct positive-direction phase-block lint (replacing the false-premise approach in #4090).
- [`docs/forensics/`](../../forensics/) — single-incident case studies; the #4090 incident has its own forensic narrative separate from this protocol.
- [`.claude/commands/research-verify-perl.md`](../../../.claude/commands/research-verify-perl.md) — the research-verifier skill file (Perl language claims).
- [`.claude/commands/research-verify-spec.md`](../../../.claude/commands/research-verify-spec.md) — the research-verifier skill file (LSP/DAP protocol claims).
- [`.claude/commands/research-verify-api.md`](../../../.claude/commands/research-verify-api.md) — the research-verifier skill file (crate API claims).

---

## Appendix A: the #4090 timeline in detail

For operators who want the full reconstruction of the incident that motivated this protocol:

1. **Scout files the issue.** The scout reads `perlmod` for context on phase blocks and draws the inference that phase-block pragmas propagate outward. The inference is confident and cites the documentation. The scout files an issue against the current `PragmaTracker` behavior, framing the existing behavior as a bug.
2. **Plan-reviewer approves the plan.** The plan-reviewer reads the spec, does not re-read `perlmod`, does not run Perl, and extends the plan with parser-invariant acceptance criteria.
3. **Builder writes 6 tests.** The tests are non-vacuous — each test verifies a specific claim (e.g. "`use strict` inside `BEGIN { }` enables strict at file scope"). All tests fail on master (red), then pass after the PragmaTracker change (green).
4. **CI gate passes.** All relevant test suites are green. No banned patterns detected. No clippy warnings.
5. **First-pass reviewer approves.** Scans for `unwrap()`, `panic!()`, scope drift. Nothing flagged. Sets `in-review`.
6. **Deep reviewer approves.** Traces the parser path into phase blocks. Verifies the new range accounting does not break existing tests. Checks for vacuous assertions in the new tests; all non-vacuous. Checks edge cases (nested phase blocks, multiple pragmas per block, empty blocks). Sets `reviewed-deep`.
7. **Ops prepares merge.** `merge-ready` is set. Ops notices that a parallel research-verifier run was in flight and decides to wait for its result before merging.
8. **Research-verifier runs `perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'`.** Output: `ok`. `strict` is not active at file scope. The premise is false.
9. **Research-verifier comments on the PR.** Evidence is posted: the command, the output, the interpretation. The `research-verified` label is applied — but the receipt is negative.
10. **Orchestrator removes `merge-ready`.** The PR is closed without merge. Issue [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) is filed for the correction cascade.
11. **Proactive audit.** The research-verifier runs against 31 other semantic claims in the codebase. 30 are correct; the one isolated discrepancy is the #4090 issue. The audit is evidence that the pipeline is not systemically broken — but the #4090 incident alone is sufficient evidence that the previous pipeline had a shared-blind-spot gap.

The total elapsed time from `merge-ready` being set to the research-verifier rejection was under 30 minutes. The merge was 9 minutes away. The protocol change in section 6 is the structural fix that removes the dependence on that margin.

---

_This protocol is a formalization, not a retrospective. For the narrative source of the principle, read the wisdom retrospective first. For the prescriptive rule a reviewer needs to apply to their next PR, read section 4 of this file. For the self-check a contributor needs to apply to their own PR body before opening it, read section 8._
