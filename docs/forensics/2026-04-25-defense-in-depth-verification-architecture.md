# Defense-in-Depth as the Verification Ladder's Architecture

**Window**: 2026-04-25 synthesis from session conversation
**Audience**: orchestrator, anyone evolving the verification ladder, anyone proposing/retiring agent classes
**Purpose**: reframe the verification ladder as defense-in-depth and pull out the operational consequences

---

## Why this doc exists

The verification ladder (research → oppositional → architecture → maintainer → standards → maintainer-PR → diff-audit → deep-review) gets described variously as "checks," "gates," "stages," or "review passes." None of these names capture its actual structure. The ladder is **defense in depth** — multiple independent layers where any single layer's failure does not compromise the system, and where coverage comes from layer composition rather than layer perfection.

Once the frame clicks, several operational decisions get clearer: when to add a layer, when to retire one, why the layer-diversity argument for activating idle plans is independent of throughput, and what the ladder's natural endpoint looks like.

---

## The frame

Defense in depth is a familiar pattern from security and reliability engineering. Its load-bearing properties:

1. **Each layer is judged by what it uniquely catches**, not by its overall catch rate. A layer's value is its marginal contribution to coverage given the layers upstream of it.
2. **Layers should be independent**. Correlated failures across layers defeat the model — N layers sharing a wrong assumption fail together and present as a single layer.
3. **The goal is minimum sufficient layers with maximum diversity**, not maximum layers. Adding redundancy at one layer is wasteful if other layers already cover that failure mode.
4. **Order matters by marginal cost**. Cheap layers should catch what they can early; expensive layers reserved for residuals only they can catch.

Applied to the verification ladder, each of these has direct operational consequences.

---

## Consequence 1 — Each layer's value is measured by what it uniquely catches

This is the principled answer to "is layer X still pulling its weight?" The wrong question is "what fraction of bugs does layer X catch?" The right question is "what fraction of bugs does layer X catch *that the layers upstream of it missed*?"

Examples:
- If upstream research (ChatGPT-Pro + repo connector) prevents 80% of "wrong direction" cases and architecture-review prevents another 15%, then maintainer-issue is judged on the residual 5% it catches that the other two missed — not on its total agreement rate with reality.
- As more risk-classes get pushed upstream, the downstream layers' jobs get *harder and more specialized*, not less important. The ladder's apparent "thinning" is risks moving to cheaper layers, not the ladder weakening.

**How to apply**: when proposing to retire a layer, frame the proposal as "upstream layers now absorb its catch class" with evidence, not as "this layer is low-value." When proposing to add a layer, frame it as "this catches a class no upstream layer can," not as "more coverage is better."

---

## Consequence 2 — Removing a layer is safe iff upstream absorbed its catch class

This gives a principled retirement criterion. Reactive layer-addition (something slipped → add a gate) is the dominant pattern; principled retirement (its catch class moved upstream → reduce per-PR latency) is the missing dual.

Worked example: spec-test-code three-way-match (added 2026-04-25) catches API hallucination and grid drift before the builder runs. If, over time, this layer reliably catches what diff-auditor catches today for the same class, diff-auditor's role can be reduced or merged into something else. The audit hasn't been done yet — but the framework for doing it (compare what each layer uniquely catches over a sample of N PRs) is now articulable.

**How to apply**: every quarter or so (or whenever per-PR latency becomes a concern), audit each layer for its marginal catch rate against samples of N PRs. Layers whose marginal catch is near zero are retirement candidates. Don't retire blindly — verify the catch class is genuinely absorbed elsewhere, not just unmeasured.

---

## Consequence 3 — Correlated failure across layers is the actual risk

The classic defense-in-depth failure mode is when N layers share an assumption that's wrong, fail together, and present as a single layer. This matters operationally because the methodology's coverage gaps are systemic and concentrated at the failure-mode layer, not distributed across PRs.

**The current ladder's correlated-failure surface**: most downstream layers run on Anthropic models (Sonnet for deep-review, Haiku for the ladder agents). If Sonnet has a blind spot for some Perl construct (say, indirect method syntax, or some heredoc edge case, or some quote-like operator parsing nuance), then deep-review and the ladder agents will all miss it the same way. The ladder's catch rate stays high *for things Sonnet is good at* while quietly dropping toward 0 for things Sonnet is bad at.

The blind spot is invisible from inside the methodology because the same model produces the verification verdicts that would have to flag it.

**The mitigation: layer diversity by model family, not just by role.**

This is the structural argument for activating the idle plans (GLM Coding Plan, Fireworks Firepass, Minimax Token Plan, OpenCode Go) that exists *independently of throughput*. Even at current per-repo volume — where adding throughput isn't urgent — decorrelating downstream layers across model families catches a class of bugs the all-Anthropic ladder structurally cannot see.

Concrete proposal: dispatch architecture-review to GLM, deep-review to Sonnet, oppositional-planner to a third family. Each layer's failures are now uncorrelated with the others' failures. A bug Sonnet would miss has a chance of being caught by GLM (or vice versa).

The throughput argument for multi-plan is secondary — adding a second Anthropic instance multiplies throughput but doesn't add diversity. Adding GLM/Fireworks/Minimax/OpenCode at different ladder layers adds both.

---

## Consequence 4 — Layer ordering by marginal cost

Cheap layers should catch what they can early; expensive layers are reserved for residuals only they can catch. This shapes:

- **What goes where**: API hallucination belongs at three-way-match (mechanical, ~$0.001), not at deep-review (interpretation, ~$0.50-$2). Banned patterns belong at standards review, not at deep-review. "Wrong direction" features belong at upstream research, not at maintainer-issue review.
- **The push-upstream principle**: every risk-class has a "natural home" at the layer that catches it cheapest. Mature methodology pushes risk-classes upstream until they no longer reach the ladder. Detailed in the companion doc on verification economics.
- **The natural endpoint**: defense-in-depth doesn't push toward zero layers. It pushes toward minimum-sufficient-layers with maximum-diversity. Endpoint shape: two or three layers with deeply uncorrelated failure modes, each catching its specific residual class cheaply. NOT "all risks pushed upstream and the ladder retired."

---

## What this doesn't mean

Some implications people might draw from "defense in depth" that don't apply here:

- **Not "more layers are always better."** Diminishing returns hit fast. Adding a fourth haiku layer that catches near-zero incremental bugs adds latency without coverage. Diversity beats redundancy.
- **Not "every layer must be uncorrelated from every other."** Some correlation is acceptable; perfect independence is impossible. The aim is to minimize systemic blind spots, not to chase pure orthogonality.
- **Not "layers should be sequential."** Many layers can run in parallel (architecture-review and oppositional-planner have no ordering dependency). The ladder is conventionally drawn as sequential because it fits a column in a routing diagram, not because the layers have intrinsic ordering.

---

## How to apply

When evaluating a proposed agent class:
- Does it catch a risk-class that no upstream layer catches? If yes, evaluate cost. If no, don't add it.
- Does it run on a different model family from existing layers in the same broad role? If yes, that's an extra reason to add it (diversity). If no, weigh the throughput vs. diversity trade.

When evaluating a proposed retirement:
- Does an upstream layer now absorb this layer's catch class? Verify with sample audit (~10 recent PRs), don't just assume.
- Would per-PR latency reduce by N% if removed? Quantify.

When evaluating a recurring miss class (bugs that repeatedly slip through):
- Is the failure mode correlated across layers (same model family missing the same thing)? Check by re-running the same PRs through a different family.
- Is the failure mode at a granularity no layer dispatches at (sub-PR, infrastructure-level)? Then it's a Conway's-law blind spot, not a ladder gap. See the methodology blind spots doc.

---

## Worked example: three-way-match agent added 2026-04-25

The agent catches API hallucination and grid drift between red-tdd commits and the builder. Defense-in-depth analysis:

- **What it uniquely catches**: red-tdd's hallucinated API references against the actual workspace surface. No upstream layer (research, oppositional, etc.) checks this; downstream layers (standards, deep-review) catch it expensively if at all.
- **Independence**: runs as haiku, mechanical row-walking. Failure modes are different from interpretation-heavy layers (it can fail on parser ambiguity, but not on judgment errors).
- **Marginal cost**: ~$0.001 per check, runs once between red-tdd-reviewed and builder dispatch.
- **Catch class moved**: red-tdd's API-shape misses (G1a: 3, G1b: 6, growing) used to surface at builder time as `cargo check` failures, then required builder/red-tdd round trips to fix. Now caught upstream of builder.

Net: ladder gets thicker by one layer, but each downstream layer's job gets simpler. Defense-in-depth math says this is a net improvement iff:
- (cost of three-way-match) < (cost of downstream-layer round trips it eliminates)

At ~$0.001 vs. ~$0.05-$0.50 per round trip, the math holds even at low catch rates.

---

## Substrate sensitivity

The defense-in-depth analysis above assumes a specific substrate (Codex 5.5 upstream, Anthropic-mostly downstream, ~current per-repo volume). The conclusions remain valid as the substrate moves but the *specific calibrations* (which layers catch what, marginal costs, retirement candidates) need re-verification when substrate shifts.

Notable upcoming substrate shifts to monitor:
- Codex versions beyond 5.5 (further upstream quality changes the catch-class distribution)
- Activation of GLM/Fireworks/Minimax/OpenCode (changes the diversity calculus)
- Three-way-match agent maturity (changes diff-auditor's residual catch class)

When substrate shifts, the right move is not to redesign the ladder but to re-audit which layer catches what, then make principled add/retire decisions.

---

## Related forensics + memory entries

- `2026-04-25-multi-axis-scaling-design.md` — the layer-diversity argument as a non-throughput reason to activate idle plans
- `2026-04-25-verification-economics-push-upstream.md` — the corollary about layer ordering by marginal cost
- `2026-04-25-methodology-blind-spots-conways-law.md` — risks at granularities no layer dispatches at
- `feedback_take_judgment_on_verdicts.md` — how to synthesize across multiple lens-layers
- `feedback_no_thumb_on_scale_in_prompts.md` — keeping layer outputs uncontaminated during synthesis
- `feedback_deep_review_bug_catch_roi.md` — ROI evidence for the most expensive layer

---

## Applies to

Reference this doc when:
- Spawning a synthesis agent (plan-reviewer) that has to weigh multiple verification verdicts
- Proposing a new agent class (asks: does this layer catch a class no upstream layer catches?)
- Considering retiring a layer (asks: has its catch class moved upstream?)
- Debating whether to activate idle plans (asks: throughput, diversity, or both?)
- Investigating a recurring miss class (asks: is this a correlated-failure blind spot or a granularity blind spot?)
- Onboarding a new operator who needs the ladder's structure explained
