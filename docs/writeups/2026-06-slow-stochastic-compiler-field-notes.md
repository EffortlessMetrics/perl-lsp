# Field Notes: The Slow Stochastic Compiler (2026-06 Campaign)

*Companion to [2026-06-agentic-maintenance-field-notes.md](2026-06-agentic-maintenance-field-notes.md).
Concepts referenced: [slow-stochastic-compiler](../concepts/slow-stochastic-compiler.md),
[stochastic-ready-pipelines](../concepts/stochastic-ready-pipelines.md),
[verify-the-instrument](../concepts/verify-the-instrument.md),
[triage-as-claim-audit](../concepts/triage-as-claim-audit.md),
[non-exhaustive-check-silent-drop](../concepts/non-exhaustive-check-silent-drop.md).*

---

## Thesis

Agentic engineering makes the hidden economics of engineering visible.

A conventional engineering team has the same costs as an agentic pipeline: reading time, rework from
imprecise specs, re-discovering the same edge cases that a previous developer hit, trusting dashboards
that measure the wrong surface, accepting CI overrides that accumulate as substrate debt. The difference
is that most of these costs are invisible in conventional practice — they appear as "slowdowns," "churn,"
"technical debt," or "unclear requirements." In an agentic pipeline, they appear as token counts, pass
failure rates, CI override counts, and learning entries.

The thesis of this writeup: **the same discipline that makes a human engineering organization work —
specification before implementation, independent verification, instrument repair rather than dashboard
trust, incident logging, durable correction — applies directly to a stochastic pipeline, made explicit
and measurable by the visibility the pipeline provides.**

The pipeline is not a novel paradigm. It is conventional engineering discipline adapted to stochastic
workers, stochastic evidence, moving branches, and uneven tool reliability.

---

## What the campaign did

The June 2026 autonomous campaign ran across approximately three weeks with the following concrete
outputs:

**variablesReference three-act resolution (#1219 → #1430 → #1450)**
The DAP `variablesReference` ID-space collision was one of the most expensive issues of the campaign.
It took three PRs to resolve because each PR fixed a real problem that revealed the next layer of the
same root cause. #1219 fixed the base-50000 collision; #1430 fixed the wire-codec band-overflow that
appeared when the fix was exercised under load; #1450 fixed the residue-disambiguation edge case that
#1430 introduced. The issue was not over-engineered across three PRs — each PR was the correct fix for
what was known at that point, and each subsequent PR was a new discovery. This is the correct model for
layered root causes: fix what you can see, build tests that reveal what you cannot, repeat.

**NodeKind category ratification (#1452)**
The `NodeKindCategory` enum was introduced to consolidate the classification logic that had been
scattered across three match arms in two crates. The PR also ratified the exhaustive-match invariant
for `NodeKind` consumers, so that future variant additions require all consumers to be updated before
the code compiles. This is a rung-1 fix on the shift-left ladder: zero marginal cost per future addition.

**Codecov substrate fix (#1453)**
Coverage CI had been running with `--lib` scope for several months, silently excluding integration test
lines. The fix was a one-line change to the CI invocation. The displayed coverage percentage shifted
after the fix — not because coverage changed, but because the instrument's scope changed. The substrate
tax was lifted. Every subsequent PR benefits from a coverage gate that measures the full surface.

**Master-break one-word fix (#1458)**
A duplicate function introduced in #651 broke master because the CI gate in use at merge time was
scoped narrowly enough to miss the duplication. The fix in #1458 was widening the gate scope by adding
`--all-targets` to the clippy invocation. One word. Permanent benefit. This is the highest-leverage
type of substrate fix: the cost is trivial, the amortization is unbounded.

**Hash-recovery parser fix (#1456)**
The parser's hash recovery logic had a silent drop for the case where a hash value is a block
expression. The fix required understanding the interaction between the block-start heuristic and the
hash-value parser. This was a case where the specification (what should the parser do with
`{ key => { ... } }`) required careful reasoning before implementation — a good example of why
plan-review exists upstream of the builder.

**NodeKind variant hazard catches (#1457/#1459)**
Three consumers of `NodeKind` used non-exhaustive patterns. When new variants were added, all three
silently dropped the new types. The green-tdd agent caught the gap by writing a test that asserted
semantic tokens for a hash subscript expression. The test failed; the consumers were converted to
exhaustive match. The hazard class was added to `hazard-class-invariants.md`.

**Backlog triage (~47 issues)**
Approximately 47 issues were triaged across the campaign. Distribution: ~18 already-fixed (the behavior
described was corrected by a subsequent merge), ~9 refuted (the behavior could not be reproduced on
current codebase), ~7 duplicates consolidated, ~8 real-bug-with-spec (filed acceptance criteria),
~3 deferred, ~2 broader-pattern (promoted to hazard classes). Most backlogs in active development have
a large proportion of phantom work; this one was no exception.

**Spec-debt reduction (~29 specs)**
A backlog of issues that had `needs-plan-review` but no corresponding `.spec/` directory was worked
through in batches. Each spec was written from the issue body, the plan-reviewer comment, and any
accuracy-scout corrections. This was spec catch-up work — reducing the gap between the issue count
and the builder-ready count. Framing: spec-debt is the same as technical debt; it accumulates when
new issues arrive faster than specs are written, and it compounds because builders without specs
produce more rework than builders with specs.

---

## Recalibrated lessons

Nine operating principles recalibrated during the campaign:

**1. Red means stop. Overrides are incidents, not doctrine.**
The verified treadmill-break exception (five conditions: human authorization, instrument failure
identified, independent verification, follow-up issue, release notes do not cite broken instrument)
is the only valid path past a red check. When overrides become routine, the signal degrades.
The repair is the instrument, not the override.

**2. The lead's judgment is the final arbitration layer.**
Agents vote. Maintainer-pr and reviewer-deep agents flag concerns. The lead decides. This is not
collective deliberation — it is structured escalation. The lead's job is to make the call that
the agents cannot make: "this risk is acceptable for this release" or "this pattern is now
disallowed." Agents produce the evidence; the lead consumes it.

**3. Triage is a claim audit, not a scheduling exercise.**
Each issue is a claim. Triage establishes which claims are true (real bug, spec needed), which are
false (refuted, already-fixed), which are duplicated, and which reveal a class (broader-pattern,
promote to hazard-class). The output is not a shorter list — it is a list with higher evidence quality.

**4. Specs reduce builder rework, not just ambiguity.**
A builder without a spec produces rework at a rate proportional to the spec's ambiguity. Spec-debt
(the gap between issue count and builder-ready count) is technical debt: it accumulates, it
compounds, and it is paid by every builder pass on an under-specified issue. Reducing spec-debt is
a force multiplier on all subsequent build effort.

**5. Adversarial-by-default is stochastic-ready judgment, not paranoia.**
In a pipeline where every artifact has a reliability profile, treating every artifact as evidence
(not ground truth) is calibration, not hostility. The stochastic-ready posture asks: "what is the
cost of acting on this artifact if it is wrong?" High-stakes artifacts (CI status, merge decisions)
require independent verification. Low-stakes artifacts (internal doc descriptions) do not.

**6. CI-cost and agent-cost are the same design problem.**
A substrate defect adds to both. An instrument repair reduces both. The design question for any
check is: "at what layer is this class of defect caught most cheaply?" The shift-left ladder
answers this; the cheapest-sufficiently-reliable rung is the correct rung.

**7. Token economics are operability budget.**
Token cost per pass is the agent-side equivalent of CI wall-clock per run. Both are real costs in
the same budget. Designing for cheap traversal by future agents (greppable contracts, indexed
concept docs, durable learning entries, precise file ownership) reduces the token budget just as
building for the maintenance team reduces the human budget.

**8. Re-create is a salvage threshold, not a preference.**
Re-create when branch state is more expensive to understand than to recreate. Untangle when history
contains useful reasoning. The artifacts that matter — the spec, the tests, the patch, the proof,
the learning — can be extracted from a tangled branch; the contaminated mechanics do not need to
be preserved.

**9. Apply cheap Rust static checks eagerly; make expensive checks earn their keep.**
Exhaustive `match`, `cargo check --all-targets`, focused clippy lints: these are rung-1 and rung-2
on the shift-left ladder. Apply them at the first recurrence of a class. Do not wait for the third
incident to justify the one-time fix. Expensive checks (deep review, full CI suites, mutation
testing) earn their keep by catching classes that cheaper checks cannot catch; they should not be
used as substitutes for cheap checks.

---

## Recurring isomorphism: non-exhaustive check at three altitudes

The most structurally interesting pattern of the campaign was the isomorphism between:

- Code-level: `if let` on one enum variant silently dropping new variants (#1457/#1459)
- CI-level: `--lib` scope silently excluding integration targets (#1282/#1453)
- Process-level: multiple PR attempts on one issue without explicit ownership signal

All three share the same structure: a check that does not enumerate all cases, and proceeds silently
as if it did. The counter-move is identical at all three altitudes: make the check exhaustive (match
arm, `--all-targets`, explicit enumeration in spec) so that new cases produce signal rather than
silence.

This isomorphism is useful because it means the same concept document (`non-exhaustive-check-silent-drop.md`)
covers all three altitudes. Agents reading about a code-level issue and agents reading about a CI issue
are reading about the same pattern. The concept index reduces token cost for future agents by providing
a single, greppable entry point for the class.

---

## Honest debts

**Undocumented overrides**: Two CI overrides were accepted during the campaign without complete five-
condition documentation. Both had human authorization and an identified instrument failure, but the
independent-verification and follow-up-issue conditions were documented post-hoc rather than pre-merge.
This is not ideal. The correct procedure is pre-merge documentation of all five conditions.

**Ripr gate complexity**: The ripr tool's suppression-application logic (coverage gap suppression)
has been patched twice in this campaign (#1346, #1349) without a structural redesign. The patches
are correct; the underlying complexity is not reduced. A structural redesign is tracked but not yet
scheduled.

**Documentation confidence inheritance**: Several concept documents written in this campaign cite
PRs and behaviors that were verified against current-codebase state at time of writing. As the
codebase evolves, specific PR citations will remain accurate (PRs do not change) but function-path
citations may drift. This is expected; it is the documented limitation of docs that cite concrete
paths.

**Convergence without deduplication**: The campaign closed ~47 issues, but did not fully deduplicate
the spec-debt backlog. Issues that describe the same root cause from different angles may have been
separately spec-ed. The next triage pass should check for spec duplicates and consolidate.

---

## The framing shift: the compiler passage

The 2026-06-agentic-maintenance-field-notes.md writeup used the framing "the pipeline compiles
vague intent into PRs through stochastic stages." The same campaign, reframed through the
slow-stochastic-compiler lens:

> The human operator adjusts the compiler. The operator does not write each assembly instruction.
> The operator sets branch policy, merge economics, risk appetite, what counts as done, which
> invariants are absolute, when to stop. When the compiler's model of reality drifts — when agents
> hallucinate a file path, merge against a stale branch, or treat a measurement failure as a
> behavior failure — the operator recalibrates the operating model. This is the human's highest-
> leverage contribution: not reviewing each output, but keeping the compiler's ground truth aligned
> with the repository's actual state. The preferred names for this mode are "human-calibrated
> autonomous execution" and "operator-guided stochastic compilation." Human-in-the-loop implies
> approval gates; this is something different — it is the operator who tunes the flags and repairs
> the buggy passes, while the workers (agents) run the pipeline.

This framing makes one thing explicit that the earlier field notes left implicit: the operator's
job is calibration, not implementation. The scarce work is not writing code — it is choosing
invariants, shaping specs, assigning proof obligations, verifying results, and correcting the
operating model when it drifts. That work moves up-stack as implementation becomes cheaper. Refs #1425.

---

## The operating style, named

Call it **evidence-weighted autonomous engineering** (equivalently, **stochastic-ready engineering**). It is not "weird things with agents" — it is a stochastic engineering system run with explicit controls. Its principles:

1. Claims are not truth until evidenced.
2. Specs are build inputs, not after-the-fact docs.
3. Tests must fail for the right reason before they prove anything.
4. CI is important but fallible.
5. Human judgment calibrates the system model (operator-guided stochastic compilation, not HITL).
6. Parallel builds are good; merges need pacing.
7. Recurring friction belongs in the substrate, not per-PR toil.
8. Learnings become repo assets.
9. Cheap static prevention is preferred; expensive bespoke gates must earn their keep.
10. Release claims only include merged, proven scope.

### The operating loop

```
1. Intake claim
2. Haiku scouts: premise, prior art, hazard classes, affected contracts, test grid, blast radius
3. Spec packet: behavior, hazards, contracts, API shape, test grid, blast radius
4. Red-TDD: test fails before fix, failure reaches the bug path, invalid-red rejected
5. Sonnet builder: one scoped slice, no broadening
6. Deep review: confirm known hazards covered, hunt novel risks, verify PR body vs diff
7. CI / proof: required checks, raw artifacts if failing, no blind debugging
8. Merge: when green, no mid-CI rebase, admin exception only as incident-backed treadmill-break
9. Learn: repo learning + portable concept if reusable + spec update if forward-looking + issue if mechanical follow-up remains
```

The code improves, but the deeper asset is that the system gets better at improving itself: claims → specs → tests → PRs → reviews → learnings → stronger specs.
