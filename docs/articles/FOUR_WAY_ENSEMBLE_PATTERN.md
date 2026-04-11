# The 4-Way Ensemble PR Spawn Pattern: Monte Carlo Code Generation with Swarm-Level Selection

*A pattern observed during the 2026-04-11 perl-lsp session — where one agent
surface deliberately produced four independent implementations of the same
problem for the other surfaces to review, compare, and pick between — and the
meta-signal that variance across the four tells you something the single
winning PR cannot.*

*This article corrects and extends the framing in
[SWARM_OPERATIONS_2026_04_11.md](SWARM_OPERATIONS_2026_04_11.md). That earlier
retrospective characterized several parallel PR clusters as duplicate-dispatch
waste or as merge races between uncoordinated operators. After more careful
reading of the session logs it became clear that at least one surface (Codex
web) was doing something fundamentally different: ensemble sampling at the PR
level. This article documents that pattern and the variance-as-meta-metric
insight that falls out of it.*

---

## TL;DR

One agent surface deliberately spawns N independent PRs against the same
problem. The N variants differ in style, structure, and sometimes in
fundamental approach. The other surfaces review the N, compare trade-offs,
pick one to merge, and archive the rest. This is **Monte Carlo code
generation**: N samples from a stochastic code-generation process, a reward
function provided by cross-surface review, selection of the best for merge,
variance across the samples informing the posterior. The novel piece is that
**the variance between the N variants is itself a meta-metric on how
well-specified the problem was**. If all four converge, the spec is stable
enough to become a ratchet floor on day one. If all four diverge, the spec
needs more work before any baseline is locked in. It's a swarm-level analogue
of property-based testing: PBT varies the input to find edge cases in the
code, ensemble spawning varies the implementation to find edge cases in the
spec.

---

## The Pattern in Plain Language

Most of the time a perl-lsp issue flows through the pipeline as a
single-threaded narrative. Scout files it, plan-reviewer improves it, one
builder writes one PR, reviewer improves that PR, ops merges it. The pipeline
stages are redundant against each other — several lenses on the same artifact
— but the artifact itself is singular. One attempt, one answer.

The 4-way ensemble pattern breaks that. Instead of one builder producing one
answer, one agent surface produces four independent answers in parallel. The
four are not coordinated with each other — each builder sees the spec, sees
no sibling attempts, writes its own solution, files its own PR. Only after
all four land does the next stage begin: the other active swarms (in the
2026-04-11 session, Claude Code device and Codex device) read the four
variants side by side, score them against the spec, pick one to merge, and
close the other three.

Read naively, this looks like coordination failure: "why did four agents all
do the same work?" That is exactly how the earlier swarm-ops retrospective
originally read it. The 2026-04-11 session had several clusters where two or
three surfaces independently produced overlapping fixes within short time
windows — #4108 vs #4120 on the pragma revert, the three parallel
`clippy needless_borrow` attempts that contributed to #4098, the
`fix/clippy-needless-borrow` unpushed branch that never became a PR. Every
single one was described in the first-pass retrospective as some flavor of
race or duplicate-dispatch waste.

Reading the commit graph more carefully makes it clear that at least some of
those clusters were intentional. They were ensemble samples. The surface
producing them — Codex web — was deliberately dispatching the same prompt
into multiple independent runs so the other surfaces could compare outputs.
The "waste" framing was wrong because the three rejected variants were not
wasted compute: they were **training data for the spec** and **evidence of
the design space**.

---

## The Machine Learning Analogy

Anyone who has trained a stochastic model has seen this structure before.
Classical Monte Carlo sampling for code generation (and for many RL-style
pipelines) looks like this:

1. Draw N independent samples from a stochastic model, conditioned on the
   same input.
2. Score each sample with a reward function.
3. Select the highest-scoring sample (or the top-k) for the next step.
4. Use the variance across the N samples to inform the posterior — high
   variance means high model uncertainty, low variance means the model has
   converged on an answer for this input.

Applied to code production by agent swarms, the structure maps across almost
directly:

| ML concept | Swarm equivalent |
|------------|------------------|
| Stochastic model | Agent surface (Codex web, Codex device, Claude Code device, etc.) |
| Sample | A PR from one dispatch |
| Input | The issue/spec/scout report |
| Reward function | Cross-surface PR review (standards + correctness + fit) |
| Best-of-N selection | Merging one PR, closing the others |
| Variance signal | Degree of divergence across the N variants |
| Posterior update | What the orchestrator learns about spec maturity |

The map is tight enough that it's worth carrying the ML intuition the rest of
the way: what's the reward function, what's the scoring protocol, how many
samples is the right number, and — the question this article exists to
answer — what do you do with the variance signal that classical best-of-N
selection throws away?

---

## The Variance Signal (The Most Novel Insight)

This is the piece that the first-pass swarm-ops retrospective missed.

When four independent implementations of the same problem come back, the
winning PR is not the most informative artifact. The most informative
artifact is the **shape of the disagreement between the four**. Three
qualitatively different regimes show up, each with a different operational
meaning:

### Regime 1: All four converge

All four PRs land on nearly the same implementation. Same files touched,
same approach, same edge-case handling, tests look structurally similar,
review finds no meaningful trade-offs between them. The "pick one" decision
becomes arbitrary — they're all equivalent up to formatting.

**Interpretation**: the problem was fully specified. Any competent builder
starting from that spec converges on the same answer because the spec
constrained the design space down to a point. This is the **stable regime**.

**Operational consequence**: the metric, fix, or scorecard baseline produced
by the winning variant can be ratcheted into a floor with **day-one
confidence**. You don't need to wait N cycles to observe stability across
independent attempts — you already have N=4 stability in a single batch.
That's a shortcut to floor-raising decisions that would otherwise take
multiple cycles of accumulated evidence.

Low exploration value beyond the verification itself. Don't ensemble next
time.

### Regime 2: Partial convergence (2–3 agree, 1–2 diverge)

Three of the four implementations look alike. One or two take a noticeably
different approach — different file structure, different helper extracted,
different edge case prioritized, different test shape, sometimes a different
semantic interpretation of an ambiguous sentence in the spec.

**Interpretation**: there's a subtle design choice the spec didn't pin down.
The majority didn't notice the choice existed (or silently made the same
default assumption); the minority made a different assumption and produced a
different artifact. Medium ambiguity.

**Operational consequence**: surface the disagreement to a plan-reviewer or a
human. The divergence itself is a **design option spec**, auto-generated by
the samples. The minority variant is often not wrong — it's exploring a
corner of the design space the majority didn't consider. This is where the
ensemble produces its highest marginal value: the outlier is a free
design-option A/B/C proposal nobody had to write by hand.

The ratchet baseline from this regime still usable, but the floor should be
raised more cautiously — you need to be sure the floor is defined in terms
that all four variants agree on, not in terms where the minority's
reinterpretation would make the number meaningless.

### Regime 3: All four diverge meaningfully

No two PRs look alike. Different files touched, different abstractions,
different diagnostic boundaries, sometimes different *problems* being
solved. One might refactor the call site, another might add a new layer,
another might patch the symptom, another might propose a config flag.

**Interpretation**: the problem itself is under-specified. The spec didn't
identify the actual design decision the builder was supposed to make, so
each builder made that decision independently and reasonably, and the
decisions diverged because there was no anchor. High ambiguity.

**Operational consequence**: **do not ratchet anything yet**. Do not even
merge one of the four as the "winner" — the merge would lock in a design
choice that was supposed to be made by a human or a plan-reviewer, not by
whichever sample happened to be picked first. Kick the problem back to
plan-review with the four variants attached as "here are four reasonable
interpretations of your spec, please pick one or write a new spec that
selects between them." The ensemble's job here is to make the ambiguity
visible, not to resolve it.

### The property-based testing analogy

The variance signal is worth framing one more way. Property-based testing
(quickcheck, proptest, Hypothesis) varies **the input** to a function and
checks that some property holds across all inputs — the variance in the
input finds edge cases in the code that hand-picked tests would miss.

Ensemble spawning varies **the implementation** of a spec and checks that
some property holds across all implementations — the variance in the
implementation finds edge cases in the spec that hand-picked spec reviews
would miss.

The two techniques attack the same class of problem (latent ambiguity in a
specification) from opposite directions. PBT's input-variance finds gaps in
what the code defends against. Ensemble-variance finds gaps in what the
spec nails down.

---

## Concrete 2026-04-11 Examples

Three clusters from the 2026-04-11 session fit somewhere on the ensemble
spectrum. Each is cited against GitHub state directly.

### #4108 vs #4120 — the pragma revert pair

This is the cleanest example. Both PRs correct the same false-premise
pragma propagation bug tracked in issue
[#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100), which
was opened after the research-verifier on
[PR #4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) caught
that `BEGIN { use strict; }` does not propagate strict to file scope —
contrary to the shared-blind-spot belief multiple reviewers had already
signed off on.

- [PR #4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108):
  *fix(pragma): keep phase-block pragmas lexically scoped*. **Merged.**
  Touches `PragmaTracker`, removes the `strict_warnings` phase-block override,
  replaces the false-premise tests with behavior-spec and integration
  coverage for block-local semantics.
- [PR #4120](https://github.com/EffortlessMetrics/perl-lsp/pull/4120):
  *fix(pragma): correct phase-block pragma scoping to match Perl lexical
  semantics*. **Closed.** Touches the same files, removes the same
  `PhaseBlock` body-scan arm, rewrites the same three integration tests with
  inverted assertions, adds six new BDD tests, adds the `perl -e`
  verification as a doc comment.

Both PRs were independently derived. Both passed the same `perl -e`
verification. Both made the same corrective edit to
`strict_warnings.rs`. Both preserved the correct pieces of
[PR #4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052) (the
`state_for_offset(usize::MAX)` fix) intact.

The first-pass swarm-ops retrospective read this as a duplicate-dispatch race:
two operators independently produced the same revert within a ~20-minute
window, and only one could merge. That framing isn't wrong, but it misses the
point. **The fact that two independent derivations converged on
functionally equivalent fixes is itself the evidence that the corrected
understanding is stable under variance.** If two surfaces starting from the
same `perl -e` reproducer produced the same revert, any third surface would
too. That's a Regime 1 signal: the corrected spec was fully constrained, and
the winning PR can be trusted on day one.

The ensemble here was almost certainly larger than the two that became PRs —
the other variants probably existed as WIP branches that never pushed, or as
Codex web dispatches that failed their internal checks and self-aborted. Two
out of four samples reaching the PR layer is consistent with typical
best-of-N filtering.

### The #4089 + `fix/clippy-needless-borrow` + #4098 cluster

A messier ensemble. At least three surfaces attempted variants of the
post-merge clippy drift after
[PR #4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052) and
[PR #4088](https://github.com/EffortlessMetrics/perl-lsp/pull/4088) landed
consecutively on master.

- [PR #4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089): the
  Windows extended-length-prefix fix for
  [#4085](https://github.com/EffortlessMetrics/perl-lsp/issues/4085), a
  parallel symptom that showed up in the same test-execution surface.
  Merged.
- `fix/clippy-needless-borrow`: an unpushed local branch on one of the
  worktree-contaminated machines — a third variant that never made it to a
  PR at all. Its existence is noted in the swarm-ops retrospective; it's
  evidence that a surface attempted the fix and silently lost the race.
- [PR #4098](https://github.com/EffortlessMetrics/perl-lsp/pull/4098): the
  variant that actually merged. Fixes the `needless_borrow` clippy error
  that appeared after #4052 and #4088. Small, targeted, landed hot as a
  hotfix via ops.

Three surfaces, three variants, one merged. The unpushed branch is the
ensemble's archived loser — a sample that existed long enough to count as a
data point but not long enough to become a review artifact. The variance
between the surviving samples was low enough to be unobservable from the
outside (the fix is three lines), which would put this cluster in Regime 1
if all the drafts were recoverable. The best evidence we have is that the
three surfaces converged on the same mechanical fix, which is consistent
with a fully-specified problem and a day-one-stable ratchet on clippy-clean
master.

### Backlog drainage during strategic work (non-ensemble)

Not every parallel PR cluster in the session was an ensemble. It's worth
calling out one cluster that wasn't, for contrast.

While the Claude Code swarm was doing strategic and research work on the
#4062 metric-stack umbrella, Codex web landed
[PR #4093](https://github.com/EffortlessMetrics/perl-lsp/pull/4093)
(`workspace/configuration` reverse-request) and
[PR #4002](https://github.com/EffortlessMetrics/perl-lsp/pull/4002) (P0
test-side idiom burn-down) in parallel. Both merged cleanly, both were
distinct in scope, neither had any other surface attempting a variant.

This is **concurrent non-overlapping productivity**, not ensemble sampling.
Two different problems, two different single-threaded pipelines, running at
the same time. No variance signal to extract because there was nothing to
vary across. The ensemble pattern is a specific technique for a specific
class of work; most of the 2026-04-11 PR volume was plain concurrent work,
and that's fine — trying to force every merge into the ensemble frame would
be over-fitting.

### Evidence from commit co-authorship

The session's commit graph carries a secondary signal that reinforces the
ensemble framing: commits co-authored by more than one surface. The
2026-04-11 master history has several commits with co-author lines attributing
work to both `EffortlessSteven` and `codex`, with the Claude Code device
surface appearing as the committer. That pattern is what you'd expect if one
surface (Codex web) produced the branch and a different surface (Claude Code
device) reviewed, picked it, and pushed the merge. The co-authorship is the
swarm-level equivalent of the reward function being applied to a sample drawn
from a different model. The detailed tracing of which commits came from which
surface is filed in
[docs/forensics/2026-04-11-three-swarm-cooperation.md (or
equivalent)](../forensics/); this article treats the tracing as a given and
focuses on the pattern.

---

## Cost-Benefit Math

The ensemble pattern costs 4x compute per ensemble-worthy problem. That cost
is real and it compounds fast — applied indiscriminately to every PR it
would explode the compute budget while producing almost no new information
on routine work. The benefit is concentrated on a narrow class of high-stakes
or high-ambiguity work where the variance signal is worth the multiplier.

### Benefit

- **Strong convergent evidence**: four independent derivations reaching the
  same answer is qualitatively stronger than one derivation reviewed four
  times. The lenses are independent in a way multi-pass review cannot be,
  because each sample starts from a fresh context rather than reasoning
  against an existing artifact.
- **Free variance map**: the divergence pattern across the N is produced at
  zero marginal cost once you're paying the 4x. Every ensemble produces a
  design-space map as a side effect.
- **Loss avoidance on high-stakes PRs**: a single-surface mistake that
  would have merged as the only attempt becomes a 1-of-4 minority vote,
  which is far easier to catch in review. The ensemble collapses the
  class of errors where "only one person tried it, and they happened to
  get it wrong" into "four people tried it and three got it right."
- **Training signal for future spawning prompts**: observing which variant
  structures tend to win, over many ensembles, is input to prompt
  engineering for the next batch. The losing variants are not discarded
  data — they're labeled negative examples.

### Cost

- **4x compute**: not just 4x LLM tokens but 4x agent-slot occupancy, 4x
  worktree activity, 4x review bandwidth from the selecting surfaces.
- **Review coordination overhead**: picking between four variants is a
  harder review task than approving one, because the reviewer has to do
  comparative scoring instead of pass/fail. That extra bandwidth is a real
  cost even when the picking protocol is cheap.
- **Archival overhead**: the losing variants need somewhere to go. If
  they're closed immediately their variance signal is lost. If they're
  left open they clutter the PR list. Either choice has a cost.

### When to use

The pattern is justified when the 4x cost is absorbed by the value of the
variance signal plus the loss-avoidance:

- **High-stakes corrections**: reverts, security fixes, correctness issues
  where a wrong answer has a large blast radius. The #4100 revert chain is
  a paradigm case.
- **Novel problems where the spec isn't obvious**: the first scorecard for
  a new metric, the first test for a newly-discovered failure mode, a fix
  for an issue where the scout found multiple candidate root causes.
- **Anything flagged by a research-verifier for external-semantics
  uncertainty**: if the verifier needed to run a reference implementation
  to check the premise, the corrected spec is probably worth ensemble
  sampling because it's already in the class of "shared blind spot likely."
- **Floor-metric candidates**: any metric that's a candidate for
  immediate ratchet-model inclusion, where day-one stability would be
  valuable and the alternative is waiting N cycles to observe.

### When NOT to use

- **Routine builder work with tight spec**: if the spec is concrete and
  mechanical ("replace `.or_insert_with(Vec::new)` with `.or_default()` in
  these three files"), four builders will produce four identical diffs and
  you've paid 4x for zero information.
- **Docs-only PRs**: the variance between four doc drafts is high by
  default and the picking protocol is subjective. Single-surface with
  review is cheaper and the quality ceiling is the same.
- **Anything where the 4x compute overhead exceeds the expected value of
  variance information**: if the cost of a wrong answer is small, and the
  cost of a re-attempt is small, ensemble the problem cheaply with N=1 and
  retry on failure.

---

## Relation to Layered Verification

perl-lsp's pipeline is already structured around the principle that each
pipeline stage is a distinct lens on the same artifact, and each lens
catches a distinct class of failure. The scout catches missing work. The
plan-reviewer catches bad approaches. The research-verifier catches false
external claims. The builder catches incomplete tests. The standards reviewer
catches banned patterns and scope creep. The deep reviewer catches logic
bugs and vacuous assertions. Ops catches CI regressions on master. Wisdom
catches patterns that repeat across cycles.

The 4-way ensemble is **another lens in the same stack**, but it's lensing a
different axis:

| Stage | What it samples | What it catches |
|-------|-----------------|-----------------|
| Scout | Problem space | Missing or mis-filed work |
| Plan-reviewer | Approach space | Wrong root cause, brittle design |
| Research-verifier | External-semantics claim space | False premises from textbook answers |
| Builder | Implementation fidelity | Tests that don't exercise the fix |
| Reviewer / reviewer-deep | Artifact quality | Standards, scope, logic, edge cases |
| Ops | Post-merge quality | CI regressions, merge conflicts |
| **4-way ensemble** | **Implementation space** | **Spec ambiguity via variance sampling** |

Every other stage samples one point in implementation space and evaluates it
against multiple lenses. The ensemble samples multiple points in
implementation space and uses their variance as the signal. That's why it
catches a class of errors the other stages structurally can't: the class
where "the plan was right, the implementation would have worked, but the
design had more than one reasonable answer and picking the first one was
arbitrary." You cannot detect that with any amount of single-sample review,
because the single sample you have is, by construction, one of the
reasonable answers. You need to see the alternatives to know they existed.

---

## Relation to the Ratchet Model

[#4105](https://github.com/EffortlessMetrics/perl-lsp/issues/4105) proposes a
4-layer ratchet model that sits on top of the layered scorecard framework in
[#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062). Ratchets
need a stability guarantee before they can raise a floor: a metric must not
slip below its current baseline, and a new baseline can only be locked in
after enough independent runs agree that the metric has actually moved. The
standard way to establish that stability is to observe the metric across
N cycles and wait for variance to collapse.

The 4-way ensemble provides the stability signal **directly and
immediately**. If four independent implementations of the same metric, fix,
or scorecard converge on the same baseline, the result is multiply-verified
on day one. You don't need to wait N cycles to see whether the floor holds
under independent attempts — you already have N independent attempts in the
batch. **One ensemble = a day-one N=4 stability check.**

Under Regime 1 convergence, the ratchet floor can be raised immediately.
Under Regime 2, the floor can be raised in the pieces all four variants
agreed on, with the disagreement surfaced separately. Under Regime 3, the
floor should not be raised at all until the spec is tightened.

That's a shortcut the ratchet model should absorb explicitly. It turns the
4x compute cost of an ensemble into operational leverage on floor-raising
decisions that would otherwise accumulate evidence slowly.

---

## Structural Recommendations

These are minimal — observational recommendations to make the pattern
legible, not new process rules. The point of this article is to name the
pattern, not to mandate its use.

- **Formalize the trigger**: a label (`needs-ensemble`, or similar) on the
  triggering issue, or a marker in the issue body, so the spawning surface
  knows which issues to sample and the reviewing surfaces know which PR
  clusters to expect. Today it's implicit in Codex web's internal dispatch
  and invisible from the outside, which is why the first-pass retrospective
  mis-framed the pattern.
- **Standardize the picking protocol**: decide in advance which surface
  reviews the N variants, how they score, and how the winner is declared.
  In the 2026-04-11 session the picking happened ad-hoc and the archival
  pattern was inconsistent (#4120 closed explicitly, the
  `fix/clippy-needless-borrow` branch silently abandoned).
- **Preserve the losers as learning signal**: don't close the losing PRs
  immediately. Leave them open-but-labeled for a wisdom agent to analyze
  what differed between the variants and why. The difference patterns are
  training data. A short grace period (one wave? one day?) is enough to
  capture the signal without cluttering the active PR list.
- **Don't ensemble everything**: the 4x cost is only justified for work
  that's high-stakes, high-ambiguity, or a ratchet-floor candidate. Most
  routine work should stay single-surface.

Explicitly **not** recommended here: tooling to automate ensemble dispatch.
Whether to ensemble-spawn is a product judgment call and the decision
criteria are not mature enough yet to automate. The right first step is to
name the pattern, observe it more carefully over subsequent sessions, and
see whether the variance-as-meta-metric framing holds up empirically before
building anything around it.

---

## Open Questions

A few things this article cannot answer from a single session's worth of
observation:

- **What's the optimal N?** The 2026-04-11 evidence is consistent with N=4
  but also consistent with N=3 or N=5. Is there diminishing marginal value
  beyond some N, and if so where? Does the curve depend on the class of
  problem (tighter for mechanical fixes, looser for design choices)? A
  single session is not enough data to draw this curve.
- **Which surface decides what gets ensembled?** In the 2026-04-11 session
  the ensemble dispatch came from Codex web and the picking came from the
  other two surfaces. Is that the right asymmetry, or can any surface
  request an ensemble of any other surface? What happens if two surfaces
  both try to ensemble each other simultaneously?
- **How is tie-breaking scoped?** When the four variants disagree in
  Regime 3, the "pick one" decision is not really a selection — it's a
  design-option fork (Option A / B / C / D). Does the same human-in-the-
  loop protocol that applies to plan-review design forks on issues like
  [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) apply
  here, or does the ensemble fork need a different escalation path?
- **Does ensemble size affect ratchet confidence linearly?** If N=4 gives
  day-one floor-raising under Regime 1 convergence, does N=8 give
  twice-as-strong day-one confidence, or is the marginal benefit
  sub-linear? Intuition says sub-linear (the second four samples are
  correlated with the first four via the shared prompt), but the intuition
  is untested.
- **Can the variance signal be computed automatically?** A diff-based
  similarity metric across the N variants would give a Regime classifier
  as a byproduct of the picking protocol. Today the regime is inferred by
  hand from reading the variants; tomorrow it could be a numeric score on
  the triggering issue.

None of these need to be answered before using the pattern. They're
articulated here so the next session that uses the pattern can collect
the data needed to answer them.

---

## Cross-References

- [SWARM_OPERATIONS_2026_04_11.md](SWARM_OPERATIONS_2026_04_11.md) — the
  first-pass retrospective this article corrects and extends. The
  duplicate-dispatch framing in that document should be read as the
  pre-ensemble-framing view; the ensemble framing here is the refinement.
- [docs/project/wisdom/2026-04-11-session-learnings.md](../project/wisdom/2026-04-11-session-learnings.md)
  — project-level wisdom retrospective on the same session, covering the
  underselling of capability, the four unwired-measurement failure modes,
  and the three truth surfaces.
- [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062) —
  layered scorecard model, the umbrella the metric-stack work ladders up
  to. This article's examples come from PRs in that umbrella's orbit.
- [#4105](https://github.com/EffortlessMetrics/perl-lsp/issues/4105) —
  4-layer ratchet model. The section above on ratchets is the direct
  operational link between ensemble convergence and floor-raising.
- [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) —
  pragma revert tracking issue. The #4108 / #4120 convergence is this
  article's cleanest ensemble example.
- [PR #4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108) and
  [PR #4120](https://github.com/EffortlessMetrics/perl-lsp/pull/4120) —
  the merged-and-closed pair that illustrate Regime 1 convergence.
- [PR #4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) —
  the near-miss that seeded the revert chain; closed without merge after
  the research-verifier caught the false premise.
- [PR #4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089),
  [PR #4098](https://github.com/EffortlessMetrics/perl-lsp/pull/4098) —
  the Windows path / clippy drift cluster, a messier ensemble whose
  losing variant stayed on a local branch.
- [PR #4093](https://github.com/EffortlessMetrics/perl-lsp/pull/4093),
  [PR #4002](https://github.com/EffortlessMetrics/perl-lsp/pull/4002) —
  concurrent non-ensemble productivity in the same session, included for
  contrast so "every parallel PR is an ensemble" is not the takeaway.
- [PR #4125](https://github.com/EffortlessMetrics/perl-lsp/pull/4125) —
  the swarm-operations retrospective this article extends.
- [PR #4127](https://github.com/EffortlessMetrics/perl-lsp/pull/4127) —
  single-incident forensic case study of the #4090 false-premise cascade;
  complementary to this article's pattern-level view.

---

*The point of naming patterns is to make them visible. Before the
2026-04-11 session had a name for this one, the ensemble PRs looked like
duplicate-dispatch waste and the first-pass retrospective recorded them
that way. After the name, the same PRs look like Monte Carlo samples with
a variance signal nobody was reading. Nothing about the underlying
behavior changed. What changed is that the next session can decide,
explicitly, whether to use the pattern or not — and can start collecting
the data needed to answer the open questions. That's what a named pattern
buys.*
