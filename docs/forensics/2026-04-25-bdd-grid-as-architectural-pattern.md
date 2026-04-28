# The BDD Grid as Architectural Pattern

**Window**: 2026-04-25 synthesis from session conversation
**Audience**: spec-planner, red-tdd, three-way-match agent, builder, anyone evolving spec/test/code workflow
**Purpose**: capture the insight that the spec-test-code grid does for the methodology layer what microcrate architecture does for the codebase layer — same architectural pattern, different domain. Implications for agent dispatch granularity, mechanical-vs-interpretive verification, and pipeline shape.

---

## The pattern

Microcrate architecture works in this codebase because every crate has:
- A bounded surface that fits in working memory
- Explicit dependency direction (one-way, machine-checkable via layer-check)
- A single concern per crate
- Verification that's mechanical (cargo metadata, layer-check, type system) rather than interpretive

The BDD grid (the per-feature spec-test-code cross-reference, instantiated in this repo as `.spec/<wave>/acceptance.md`) does the same thing for the methodology layer:
- Each grid row has a bounded surface (one assertion + named code-side ref + named test-side ref)
- The grid has explicit dependency direction (spec authors → tests → code, not the reverse)
- Each row covers a single concern
- Verification is mechanical (does the named file exist? does the named symbol resolve? does the named test exist?) rather than interpretive

Same architectural pattern at a different layer. The codebase layer eliminates "does this crate's API drift from its consumers?" via layer-check; the methodology layer eliminates "does this test drift from this spec?" via grid-walk.

---

## Why the pattern matters

The pattern is load-bearing because **mechanical verification scales; interpretive verification doesn't**. A reviewer can hold one microcrate in their head; they cannot hold the whole codebase. A three-way-match agent can walk grid rows mechanically; it cannot judge "do these tests really cover the spec?" at the prose level reliably.

Both layers exist because they replace human/agent interpretation with structured cross-reference checks. The cost is up-front: someone has to write the cross-reference (via spec-planner or via crate organization). The payoff is recurring: every subsequent verification is cheap and deterministic.

This is the same trade-off the methodology makes everywhere it can — see also: labels-as-state-machine, forensics-as-prompt-fragments, the verification ladder's principled add/retire criterion. The pattern is "make the structure explicit and machine-readable so verification can be cheap and mechanical."

---

## What the grid gives that prose specs don't

A prose spec describes intent. Three artifacts (spec, tests, code) interpreted by three different agents drift from each other through interpretation. The grid eliminates the interpretation surface by naming the cross-references explicitly:

| Grid row carries | What this lets the agent do mechanically |
|---|---|
| Assertion text | Identify the unit of work |
| Code-side reference (file:line, symbol, file path) | Verify the named code element exists / will exist |
| Test-side reference (test file name, test function, helper) | Verify the named test element exists / will exist |
| Sanctioned-new-surface marker | Distinguish "API doesn't exist yet (red-TDD)" from "API doesn't exist and isn't supposed to (hallucination)" |

A row where any of these three columns is missing is *itself a finding* — the spec hasn't done its work. A row where all three exist but don't agree is also a finding — drift detected.

The acceptance.md format already used in `.spec/<wave>/acceptance.md` instantiates this pattern as a checklist where each `[ ]` row carries the (assertion, code-side ref, test-side ref) triple inline. Not a literal table — structurally equivalent. The three-way-match agent walks rows, not columns.

---

## Consequence 1 — Verification becomes mechanical, not interpretive

Without the grid, a reviewer asking "does this implementation match the spec?" is doing prose interpretation. Two reviewers can disagree honestly. The reviewer's mental model has to span the spec, the tests, and the code at once.

With the grid, the same question decomposes into mechanical checks:
- For each grid row, does the named code-side reference exist (or get added in the diff)?
- For each grid row, does the named test-side reference exist and assert what the row says?
- For each diff hunk, is it covered by some grid row?
- For each test reference, is the API it calls either resolvable on master or sanctioned by the grid as new surface?

None of those is a judgment call. All run on haiku in seconds. The mechanical-verification budget for "is this feature done?" goes from minutes of sonnet interpretation to seconds of haiku grep-and-resolve.

This is the same property microcrates give the codebase layer: structural decomposition of an interpretation problem into a cross-reference problem.

---

## Consequence 2 — Red-TDD's job becomes constrained, not creative

Without the grid, red-TDD reads the spec and *interprets* what tests should exist. This is creative work. It's also the source of the API hallucination class (`feedback_red_tdd_needs_api_read.md`: G1a 3 fixes, G1b 6 fixes, growing) — when red-TDD interprets the spec into tests, it interprets API shapes too, and gets them wrong.

With the grid, red-TDD reads the grid and *implements* the test side of each row. The test names are named in the row. The API shapes are sanctioned by the row (or referenced as existing on master). Red-TDD can't add a test that doesn't have a grid row, and can't skip a test that does.

The "red-tdd writes tests against imagined APIs" failure mode collapses if the grid names the API surface upfront — because either the grid sanctions the API (in which case red-TDD writes against the sanctioned shape) or it doesn't (in which case the API is supposed to exist on master, and red-TDD reads master to confirm the actual shape).

Memory entry `feedback_red_tdd_needs_api_read.md` and the deferred Fix B in `.spec/4513-red-tdd-api-read/` (require spec-planner to enumerate API surfaces in `context.md`) are partial solutions to this. The grid pattern is the structural solution: enumerate the API surface in the *grid*, not in a separate `context.md` field, so the grid-walk verification fires automatically.

---

## Consequence 3 — Builder context narrows

The builder under prose specs knows roughly what to implement. The builder under a grid knows *exactly* which code locations the spec covers — because the grid says so. "Did the builder change something the spec didn't cover?" becomes mechanically detectable: any diff hunk outside grid-named locations is either out-of-scope or needs a new grid row first.

This is much tighter than diff-auditor's current scope check (which is "does this look coherent" — judgment call). With the grid, scope drift detection is a `grid-named-locations ⊇ diff-hunk-locations` set check.

The builder's mental model also narrows: the work is "make these grid rows pass" rather than "build this feature." A grid row is a self-contained work unit — one assertion + one named test + one named code location. Smaller blast radius per builder operation, fewer places where the builder has to make judgment calls.

---

## Consequence 4 — Drift detection becomes structural

When the spec changes, the grid changes. Rows whose test-side or code-side no longer matches flag automatically. When a code refactor moves a function, the grid row's named code location is wrong and the next three-way-match catches it. When a test gets renamed, the grid row's test-side reference is broken.

Drift is now a *failed cross-reference*, not a hard-to-spot inconsistency between three artifacts you have to interpret. This converts a recurring "deep-review caught X weeks-after-merge" pattern into a "three-way-match flagged X before merge" pattern.

The cost-shift is dramatic: deep-review costs ~$0.50-$2 per pass; three-way-match costs ~$0.001. A drift caught upstream costs three orders of magnitude less than a drift caught at deep-review.

---

## Consequence 5 — The grid is the agent dispatch unit

This is the proposal that follows from the architectural pattern.

A grid row is a self-contained work unit:
- Spec assertion (what)
- Named test (how we'll know it's done)
- Named code location (where it lives)

Three lines of context. Fully self-contained. Suggesting that builder agents could be dispatched at row-granularity, not at PR/issue-granularity.

Implications:
- **Much smaller blast radius per agent operation**: a builder dispatched at one row can only touch the named code location. Nothing else.
- **Massive parallelism**: N rows in a spec can have N builders running simultaneously, each on its own row, each with bounded context.
- **Easier verification per operation**: three-way-match against one row is trivial; against an entire spec is more work.
- **Failure isolation**: one row's builder failing doesn't block other rows. Per-row retry/fix is cheaper than whole-spec retry.

The current methodology dispatches at issue/PR granularity because that's the existing scaffolding. Row-granularity dispatch would require:
- A row-aware builder agent (input: one row, output: code change covering that row)
- A row-aware three-way-match (input: one row + diff, output: pass/fail for that row)
- A row-completion tracker (status: row N of M done)
- A row-aggregator (when all rows pass, propose a single PR)

None of these exist yet. They're proposable infrastructure. The architectural pattern says they would compose cleanly with what's already here, and the parallelism gain is potentially substantial for large specs.

---

## Why this is the same pattern as microcrate architecture

The two patterns share their core property: **decompose an interpretation problem into a cross-reference problem**.

Microcrates:
- Without: monolithic codebase, "does this change break consumers?" requires global reasoning
- With: each crate has a bounded surface; consumers are explicit; layer-check verifies dependency direction mechanically

BDD grid:
- Without: prose spec + tests + code, "does this feature work?" requires interpreting all three artifacts together
- With: each grid row has a bounded triple; cross-references are explicit; three-way-match verifies cross-reference resolution mechanically

Both substitute structural decomposition for global interpretation. Both pay an up-front cost (microcrate organization, grid authoring) for recurring downstream verification savings.

The methodology is using the same architectural pattern at every layer where it can:
- Codebase layer: microcrates
- Methodology layer: BDD grid
- State layer: labels-as-state-machine
- Knowledge layer: forensics-as-prompt-fragments
- Verification layer: defense-in-depth ladder

The consistent application of the pattern is itself a methodology property — and it predicts where new infrastructure should go (anywhere global interpretation is currently required, decompose it into cross-references).

---

## What this changes about the spec-planner role

If the grid is the architectural keystone, spec-planner's job is **grid authoring**, not document writing. The acceptance.md is a checklist; the checklist.md is operational; the context.md is supporting prose. The grid IS the spec, in the load-bearing sense.

Implications:
- Spec-planner output quality should be measured by grid completeness and resolution (do all rows have all three sides? do they resolve mechanically?), not by prose quality.
- A spec without a grid is half-done and should be sent back, the same way a parser-core change without a layer-check pass would be sent back.
- New spec-planner agents (or revisions to the current one) should generate the grid as their primary deliverable, with prose as derived/supporting material.

The deferred Fix B in `.spec/4513-red-tdd-api-read/` (require API surface enumeration in `context.md`) is a step toward this but in the wrong place — the API surface belongs in the grid, not in a separate context field, so the grid-walk fires it automatically.

---

## How to apply

When designing a new spec-process artifact, ask: does this enable mechanical cross-reference verification, or does it require interpretation? If interpretation, find the cross-reference decomposition before adding the artifact.

When evaluating spec-planner output, check the grid first: every behavioral assertion has both a code-side and test-side named reference; every reference resolves; every reference unique to this spec is sanctioned as new surface.

When considering new agent classes, prefer ones that walk the grid mechanically over ones that interpret prose. The three-way-match agent is the worked example of this preference.

When proposing dispatch granularity changes, evaluate whether row-granularity composes cleanly with the existing scaffolding (it does for builders, would for verifiers; less obvious for end-to-end runs that need cross-row state).

---

## Related forensics + memory entries

- `2026-04-25-defense-in-depth-verification-architecture.md` — the verification ladder context that three-way-match plugs into
- `2026-04-25-verification-economics-push-upstream.md` — why mechanical layers (like grid-walk) belong upstream of interpretive ones
- `feedback_red_tdd_needs_api_read.md` — the partial-solution memory entry that the grid pattern subsumes
- `.spec/4513-red-tdd-api-read/` — the deferred Fix B that should be reframed as a grid-authoring requirement
- `.claude/agents/spec-test-code-match.md` — the agent that walks the grid mechanically

---

## Applies to

Reference this doc when:
- Authoring or evaluating a spec-planner output (check grid completeness)
- Spawning a three-way-match agent (the agent walks the grid; this doc explains why the grid is the source of truth)
- Considering a new spec/test/code verification agent class (prefer mechanical row-walking)
- Debating whether row-granularity dispatch is worth the infrastructure cost
- Onboarding a new spec-planner or red-tdd implementer who needs the architectural rationale
