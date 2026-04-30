# 2026-04-25 — Verification Economics: Push Upstream

**Window**: Synthesis from the 2026-04-23 → 2026-04-25 three-day arc, distilled into a standalone economic principle
**Audience**: Future operators tuning the verification ladder, deciding where to add or retire a layer
**Purpose**: Codify the cost-ordering principle that determines the natural home for each risk-class, and the trajectory the methodology should track over time

This doc is a corollary of the defense-in-depth framing for the verification ladder, but operationally distinct enough to deserve its own page. Defense-in-depth says *use uncorrelated layers so failures in one don't propagate*. This doc says *order those layers by marginal cost, push each risk-class to the cheapest layer that can catch it, and accept that the upstream-most layer is "no code written yet."*

---

## Core principle

Every risk caught at deep-review is expensive but recoverable. Every risk *prevented* by upstream verification before any code exists is free.

The mature methodology pushes risk-classes upstream until they no longer reach the verification ladder. As more risks move upstream, the ladder thins — not because it's less valuable, but because it's catching residual cases. The endpoint isn't a thinned-out ladder; it's a ladder where each remaining layer has a unique residual catch surface that nothing upstream of it can address.

The corollary: when a new risk-class emerges, the question is not "which existing layer should catch this?" but "what's the cheapest layer that *could* catch this, and does that layer exist yet?"

---

## Layer ordering by marginal cost

Cheapest to most expensive. Cost includes both per-invocation token cost and downstream cost saved (a catch at layer N saves the cost of all layers 1..N-1 running on a doomed PR).

| # | Layer | Substrate | Per-invocation cost | What it uniquely catches |
|---|---|---|---|---|
| 1 | **Upstream research** | ChatGPT-Pro + GitHub repo connector | $0.05–$0.50 | "Wrong direction" risk at the intent layer, before code exists |
| 2 | **Spec-test-code three-way-match** | haiku, mechanical | ~$0.001 | API hallucination, spec-test drift before builder runs |
| 3 | **Verification ladder** (accuracy/research/oppositional/architecture/maintainer-issue/plan-review) | haiku, judgment-light | $0.01–$0.10 per pass | Structural drift, scope creep, project-fit issues |
| 4 | **Standards review** | haiku | ~$0.05 | Banned patterns, formatting, basic correctness |
| 5 | **Deep review** | sonnet | $0.50–$2.00 | Subtle correctness bugs, edge cases, performance issues |
| 6 | **Real-world testing** | manual / scheduled | hours of operator time | What synthetic tests can't see (real workspaces, real Perl projects) |

Every layer above the one that catches a risk is "wasted on this PR" — the PR was doomed, the upstream layers ran anyway. Every layer below the one that catches it is "saved on this PR" — the catch prevented N expensive downstream invocations. The cost-ordering exists because catching a risk at layer K saves all of K+1..6 from running on a doomed artifact.

---

## The "push it upstream" trajectory

Each risk-class has a natural home — the cheapest layer that can reliably catch it. Mature methodology converges on placing risks at their natural home. Examples observed across the 2026-04-23/24/25 arc:

| Risk class | Natural home | Currently caught at | Gap |
|---|---|---|---|
| API hallucination | Three-way-match (mechanical, layer 2) | Builder iteration → deep-review (layer 5) | Closing — three-way-match agent added 2026-04-25 |
| Wrong-direction features | Upstream research (layer 1) | Maintainer-issue review (layer 3) — late, spec already exists | Partial — ChatGPT-Pro use is inconsistent |
| Banned patterns | Standards review (layer 4) | Standards review | Closed — at natural home |
| Subtle algorithmic bugs | Deep review (layer 5) | Deep review | Closed — only layer that can see them |
| Branch contamination, .hermes leaks | Diff-audit (layer 4-equivalent) | Maintainer-PR (false positives), diff-audit (true positives) | Closing — work-id check added |
| Tooling friction (xtask cascade, sandbox timeout) | None — methodology dispatches at PR level | Master bit-rot scout (reactive, after damage) | Open — tooling-debt-scout agent added 2026-04-25 as first attempt |
| Stale-base PRs | Plan-review (layer 3) | Update-branch failures at merge time | Open — needs pre-build merge-base check |
| Cross-PR `.spec/` cumulation | Diff-audit subdir-name check | Diff-audit (added 2026-04-25 hardening) | Closing |

**Pattern**: every risk-class either sits at its natural home (closed), is in transit toward it (closing), or is being newly recognized (open). The methodology evolves by identifying open risks and building or extending the layer that catches them cheapest.

---

## The natural stopping point

Defense-in-depth doesn't push toward zero layers. It pushes toward minimum-sufficient-layers with maximum-diversity. The endpoint isn't "all risks pushed upstream and the ladder retired." It's: two or three layers with deeply uncorrelated failure modes, each catching its specific residual class cheaply.

A layer is correctly placed when:

1. The risk-class it catches has no cheaper home (no upstream layer can see it).
2. Its failure mode is uncorrelated with adjacent layers (a haiku-mechanical layer's blind spots aren't shared by a sonnet-judgment layer).
3. The marginal catch rate justifies the marginal cost (catches per dollar exceeds the next-cheapest alternative).

A layer is correctly retired when:

1. Upstream layers have absorbed its catch class.
2. Its remaining catches are duplicates of what downstream layers also catch.
3. Per-PR latency cost exceeds residual catch value.

Neither addition nor retirement is the default. The methodology is in steady state when each layer's catch surface is unique and minimum-sufficient.

---

## Practical implications for ladder evolution

**When adding a new layer**: ask what risk-class it uniquely catches that no upstream layer can. If the answer is "nothing — it just doubles up on what X catches," don't add it. The three-way-match layer (added 2026-04-25) passes this test: red-tdd ↔ builder API drift was being caught at deep-review (layer 5) at sonnet cost; three-way-match catches it at haiku cost (layer 2) before builder runs. The catch surface is unique to layer 2 because deep-review can see the bug but can't prevent the builder cycle.

**When considering retiring a layer**: ask whether upstream layers have absorbed its catch class. If yes, retire and reduce per-PR latency. If no, keep. No layer in the current ladder qualifies for retirement at session end 2026-04-25 — every layer's residual catch is unique. But the trajectory is monitored; if upstream research (layer 1) becomes consistent enough to absorb maintainer-issue's "wrong direction" catches, maintainer-issue can shrink to project-fit-only.

**When debating "should we tune layer X"**: ask whether the tuning reduces marginal cost or increases marginal catch. If neither, you're overfitting to a substrate that's about to change. Codex 5.5 + ChatGPT-Pro substrate (literal-yesterday, 2026-04-24/25) is meaningfully better than 5.4 baseline; tuning a downstream layer to compensate for a 5.4-era failure mode is wasted work if 5.5 doesn't exhibit it.

---

## Why this matters now

The substrate is improving. Codex 5.5 + ChatGPT-Pro generates PRs with measurably fewer hallucinations and narrower scope drift than 5.4 baseline. More risk-classes will be naturally caught upstream — at the prompt-generation layer or at the upstream-research layer — before any agent in the verification ladder sees them.

The downstream ladder should EXPECT to thin over the next few sessions. That's not a sign of degradation — it's the right direction. Catches per layer per session should drop while catch *quality* per remaining catch goes up (because trivial catches are absorbed upstream).

The risk in this transition: continuing to dispatch the full ladder out of habit when the early layers are catching nothing. Per-PR latency cost compounds. The orchestrator should periodically resample catch rates per layer per substrate-version and consider thinning when a layer's catch rate drops below its dispatch cost.

---

## The three-way-match agent as a worked example

Red-tdd was producing tests with hallucinated APIs at known rates. Per-wave fix counts from the 2026-04-23 arc:

| Cluster | Red-tdd hallucination fixes required |
|---|---:|
| G1a | 3 |
| G1b | 6 |
| Trend across waves | growing |

Three options were considered:

1. **Push to red-tdd** (add API-read step): done in `feedback_red_tdd_needs_api_read.md`. Partial coverage — red-tdd's API enumeration is best-effort and misses cases where the public surface has changed since the last red-tdd run.

2. **Push to spec-planner** (require API surface enumeration in `context.md`): DEFERRED as Fix B in `.spec/4513-red-tdd-api-read/`. Higher leverage long-term, but requires upstream tooling investment and changes the spec-planner contract.

3. **Add a new layer** (three-way-match between red-tdd and builder): done 2026-04-25.

The three-way-match layer is haiku-mechanical. Cost per check: ~$0.001. It reads the spec, the red-tdd tests, and the actual public API of the affected crates, then verifies all three agree on function signatures and types before the builder runs. Catches API hallucination before the builder spends time iterating against tests that can't compile.

The economic case:

| Alternative | Per-instance cost | Catch rate | Catches per dollar |
|---|---|---|---|
| Builder iteration → deep-review catches at layer 5 | $0.50–$2.00 | ~100% (deep-review catches all known cases) | ~0.5–2 catches/$1 |
| Three-way-match at layer 2 | $0.001 | ~100% (mechanical check, no judgment) | ~1000 catches/$1 |

Three orders of magnitude cheaper for the same catch rate. The layer's existence reduces deep-review's residual catch surface for this specific class — deep-review can now spend its $1.50 budget on the algorithmic bugs only it can see, instead of repeatedly catching API drift that should have been caught at layer 2.

Net: ladder gets thicker by one layer, but each prior layer's job gets simpler and cheaper. This is the right direction even though the layer count goes up — the metric is total cost per merged PR, not layer count.

---

## What this doesn't say

This framing does NOT say "always add more layers." Two specific anti-patterns to avoid:

1. **Adding a layer that doubles up** on an existing layer's catch surface. If standards-review already catches banned patterns at $0.05, adding a "banned-pattern-deep-check" sonnet layer at $1.50 doesn't add catches per dollar — it subtracts them.

2. **Adding a layer to compensate for a misconfigured upstream layer**. If red-tdd is producing bad tests because its prompt is wrong, fix the prompt (free) instead of adding a verifier (recurring cost). The three-way-match agent passes this test only because red-tdd's prompt has been iterated multiple times and the residual hallucination rate is structural, not prompt-fixable.

The corollary: every new layer should justify its existence by pointing at the specific residual class it catches that no upstream layer can. "It might catch some bugs" is not a justification.

---

## Cost-ordering vs. capability-ordering

The layers are not ordered by capability. Layer 5 (sonnet deep-review) is more capable than layer 2 (haiku three-way-match). But layer 2 is upstream because it's cheaper, not because it's more capable. The ordering is:

```
upstream  ←  cheaper, narrower scope, mechanical
downstream  ←  expensive, broader scope, judgment
```

A layer's capability sets its catch ceiling. Its cost sets its position in the order. The two are independent — a $0.001 mechanical check can prevent a $2.00 sonnet review from ever needing to run on a doomed PR, and that's the entire economic case for the ordering.

This also means the ladder is not a "weak-to-strong" cascade. Layer 5 isn't catching what layers 1-4 missed because it's stronger; it's catching what they missed because it's the only layer that can see that specific class of bug. If layer 5's residual catches start looking like things layer 4 *could* have caught at lower cost, that's a signal to tune layer 4, not to celebrate layer 5's catch.

---

## Cross-references

Sibling analyses from the 2026-04-23/24/25 arc:

- `2026-04-25-3day-arc-economics-and-learnings.md` — quantitative metrics for the arc
- `2026-04-25-orchestration-anatomy.md` — wave composition and operator-orchestrator dynamics
- `2026-04-24-extended-throughput-session-retrospective.md` — deep-review catalog (sections 4a–4g)
- `feedback_red_tdd_needs_api_read.md` — original red-tdd hallucination memory
- `feedback_research_verifier_roi.md` — 6.3% scout error rate caught by haiku verifiers
- `feedback_multigate_catches_cheap_model_drift.md` — gate ordering catches from Codex bursts
- `feedback_upstream_research_improves_pr_quality.md` — ChatGPT-Pro pre-planning ROI
- `feedback_prompt_generation_is_cheap_web_thread.md` — upstream prompt-gen as commodity layer
- `feedback_verification_is_sequential.md` — why ordering matters even within the ladder

---

## Applies to

- Any time evaluating where a new verification layer should live in the ordering
- Any time considering retiring an existing layer because upstream layers have absorbed its catches
- Any time deciding between fixing a prompt at an existing layer vs. adding a new layer
- Any time the substrate (Codex version, ChatGPT-Pro version, Anthropic model version) changes and the catch-rate distribution across layers shifts
- Any time per-PR latency cost feels high and the question is "which layer can I drop" — answer is "the one whose residual catches are also caught downstream at acceptable cost"
- Any time someone proposes a layer with the justification "it might catch some bugs" — answer is "name the specific residual class no upstream layer catches, or don't add it"
