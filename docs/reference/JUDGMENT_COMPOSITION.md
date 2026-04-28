# Judgment Composition — Synthesizing Multi-Agent Verdicts

**Status**: Active methodology (introduced 2026-04-27)
**Related**: [PIPELINE_GATES.md](./PIPELINE_GATES.md) | [LIVE_SIGNALS_VS_LABELS.md](./LIVE_SIGNALS_VS_LABELS.md) | [ORCHESTRATION_DOCTRINE.md](./ORCHESTRATION_DOCTRINE.md) | [CLAUDE.md](../../CLAUDE.md)

---

## The Problem

Each gate in the pipeline deploys multiple agents. Each agent produces a verdict. When verdicts disagree, the orchestrator must synthesize — decide what is actually true — before routing the PR forward or back.

The wrong mental model: treat verdicts as votes, count sign-offs, let the majority decide.

The right mental model: treat each agent as a **lens with a specific viewing angle**. The picture you need is what the lenses compose into together, not a tally of how many lenses passed.

---

## The Lens Metaphor

A microscope and a telescope are both valid optical instruments. They cannot be averaged. A result that looks correct under one is not contradicted by the other simply being present — each reveals a different slice of reality.

Verification agents work the same way:

| Agent | Viewing angle |
|-------|--------------|
| reviewer | Standards correctness — banned patterns, scope, formatting |
| maintainer-pr | Project vision — does this fit perl-lsp's direction and user base? |
| reviewer-deep | Logic correctness — does the code actually work? edge cases? regressions? |
| diff-auditor | Artifact cleanliness — is the diff coherent, scope-matched, cross-PR-clean? |
| green-tdd | Test coverage — what did the builder's tests miss? |
| refactor-planner | Structural quality — what could be simplified or extracted? |

Each agent sees something the others do not. A reviewer can sign off on scope while reviewer-deep finds a logic bug. That's not a contradiction — those are different planes of the same PR.

**Synthesis means**: combine what each lens reveals into a coherent picture of the PR's state. It does not mean count how many agents passed.

---

## Composition Rules

### Rule 1: Check whether each lens's premise still holds

A verdict is a claim attached to a premise. The premise is what the agent was actually looking at.

If another lens invalidates the premise, the verdict no longer carries its claimed weight.

**Example**: accuracy-scout signs off that "the file path in the spec is correct." Later, plan-reviewer rewrites the spec to target a different file. The accuracy-scout's sign-off still exists — but its premise (the original file path claim) has been superseded. Do not count the label as evidence that the new file path is correct.

**Check**: Before treating a passing verdict as evidence, ask — "was this agent looking at the same artifact I'm evaluating right now?"

### Rule 2: A failing lens whose concern is addressed elsewhere should not bounce

A bounce is expensive. It routes the PR back through a build or fix cycle. Only bounce when the failing lens's concern is real and unaddressed.

If another gate, a prior fix, or a different agent's pass already addresses the concern the failing agent raised, the concern is addressed — even if the failing agent's label was never updated.

**Example**: green-tdd files `needs-builder-fix` because a test was flaky. The builder fixes the flakiness, pushes, and CI goes green. The `needs-builder-fix` label was stripped by the reconciler. reviewer-deep runs and finds the same test is now stable. The bounce concern is addressed. Do not re-bounce on stale evidence from a now-resolved state.

**Check**: Before acting on a bounce, verify the concern is present in the current state of the PR — not just in a prior agent comment.

### Rule 3: Give weight to specificity

Generic concerns ("this might have edge cases") are weaker evidence than specific findings ("line 47 in `scope_analyzer.rs` does not handle the case where `$self` is `undef`").

When two agents disagree — one passing, one bouncing — specificity is the tiebreaker:

- **Concrete finding (file:line, reproducible scenario)**: treat as strong evidence; investigate directly
- **Generic concern ("this seems risky")**: treat as a prompt to verify, not as a bounce trigger by itself

**Check**: Does the bouncing agent have a concrete finding or a generic concern? Concrete findings require resolution. Generic concerns require investigation.

### Rule 4: A passing lens whose premise is invalidated should not tip the verdict

The inverse of Rule 1. If conditions changed such that an agent's approval no longer applies to the current state, do not count that approval as evidence.

This is especially relevant for CI-related labels. A `ci-green` label from a push three days ago does not mean CI is currently green. Query the live signal.

For non-CI labels (where no live signal exists), apply the GitHub timeline rule: the later-applied label takes precedence. But also apply the premise check: did conditions change in a way that makes the earlier verdict irrelevant regardless of timeline?

---

## The Three-Axis Triangulation

Gate 4 is where synthesis is most complex. Six agents, three axes, one routing decision.

From [PIPELINE_GATES.md](./PIPELINE_GATES.md) Gate 4:

**Axis 1 — Right thing** (matches user/issue intent):
- reviewer: scope and issue alignment
- maintainer-pr: fit with perl-lsp's direction and user base

**Axis 2 — What the codebase needs** (architecture and structural quality):
- refactor-planner: simplification and reuse opportunities
- architecture-reviewer (if Gate 2 was thin): microcrate layering fit

**Axis 3 — Right way** (correctness, idiom, regression safety):
- reviewer-deep: logic, edge cases, regressions
- diff-auditor: diff coherence, artifact cleanliness
- green-tdd: test coverage gaps

**Key synthesis rule**: A PR that passes two axes but fails the third does not pass Gate 4. The axes are not scored — they are all required.

A PR where reviewer and maintainer-pr both pass (Axis 1 clear) but reviewer-deep finds a logic bug (Axis 3 failing) is not "2/3 done." It is not ready. Fix the logic bug.

A PR where reviewer-deep and diff-auditor pass (Axis 3 clear) but maintainer-pr finds scope drift away from the issue spec (Axis 1 failing) is not ready either. The fact that the code is correct is not sufficient if it's the wrong code.

Synthesize by answering: "Which axes have unresolved failures?" Route based on the answer, not on the count of passed agents.

---

## Anti-Patterns

### Anti-pattern 1: Voting

Counting sign-offs as if they were votes toward a passing threshold.

"Five agents signed off, one bounced — the majority wins."

Why it fails: a single concrete finding from reviewer-deep that the code silently truncates data at a module boundary is not outweighed by five clerical passes. The severity and specificity of the finding matter; the count does not.

### Anti-pattern 2: Thumb on the Scale

Biasing synthesis prompts toward a predetermined verdict.

"My bias is toward merging this one — can you confirm it looks okay?"

Why it fails: contaminated synthesis prompts produce contaminated verdicts. The orchestrator's role is to present prior verdicts neutrally and synthesize honestly. Priming the synthesis agent poisons the output, particularly when the genuine finding is the one the orchestrator's prior instinct would dismiss.

See memory entry `feedback_no_thumb_on_scale_in_prompts` — this failure mode was explicitly captured.

### Anti-pattern 3: Last-Wins Drift

Letting the most recent verdict unconditionally override all prior verdicts.

"The latest agent said it's fine, so we're good."

Why it fails: a downstream agent that doesn't look at the upstream agent's concrete finding — because they were run out of order, or because the spec they received omitted the prior finding — can produce a passing verdict that ignores real evidence. "Latest" is not "most informed."

The correct question is not "which verdict was posted most recently?" but "which verdict has the most relevant premise for the current PR state?"

### Anti-pattern 4: Premise-Blind Acceptance

Accepting a passing verdict without checking whether its premise still applies.

This is the accumulation failure: labels pile up across reviews, pushes, and fixes, and eventually a cluster of stale "pass" labels creates an illusion of comprehensive review. Each label was genuine when applied. The problem is that the artifact they evaluated has changed.

Defense: for each gate-critical sign-off, note the HEAD SHA it was applied on. If the PR has had pushes since, re-verify the critical lenses on the new HEAD.

---

## Worked Examples

### Example 1: Two passes, one concrete bounce — the bounce wins

**Scenario**: PR #6748, `split_qualified_name` fix.

reviewer signs off: scope matches issue, no banned patterns.
maintainer-pr signs off: fix aligns with perl-lsp's qualified-name handling direction.
reviewer-deep bounces: finds that the fix handles `Foo::Bar` but not `Foo::Bar::Baz` — the two-segment assumption is hardcoded, and Perl's module hierarchy allows arbitrary depth.

**Synthesis**: The bounce has a concrete finding (hardcoded segment count assumption, specific function, reproducible scenario). The two passing lenses were not looking at this dimension — reviewer checks standards, not logic depth; maintainer-pr checks project fit, not edge-case completeness.

The bounce's premise is not invalidated by the passes. The passes' premises are not invalidated by the bounce. All three verdicts are valid; they cover different planes.

**Correct routing**: bounce. Route back to builder with the specific finding: "handle arbitrary segment depth."

**Incorrect routing**: merge because 2 > 1.

---

### Example 2: Six passes, one questionable bounce — investigate the premise

**Scenario**: A PR adding new LSP hover documentation for local variables.

All six Gate 4 agents run. Five sign off. diff-auditor bounces with "scope drift — this PR modifies `scope_analyzer.rs` in addition to the hover provider."

Investigating: the modification to `scope_analyzer.rs` is a two-line fix the builder made to expose the local variable resolution result that the hover provider needed. Without it, hover cannot access the data. It was mentioned in the spec's context files as a required prerequisite, but diff-auditor's prompt did not include the spec files.

**Synthesis**: diff-auditor's premise is partially invalidated. The scope drift concern is based on "this file wasn't in the issue title" — but the spec and the builder's commit message explain why it was touched. The touch was necessary, not incidental, and it was in scope per the spec.

**Correct routing**: read the diff-auditor's specific finding against the spec. If the `scope_analyzer.rs` change is genuinely minimal (the two lines described) and serves only the hover provider's data access need, the bounce premise doesn't hold under the other lenses' context. Sign off and proceed.

**Incorrect routing**: bounce automatically because diff-auditor said bounce, without checking whether the concern holds.

**Important**: "investigate the premise" means actually read the finding and the spec. It does not mean dismiss the bounce because most agents passed.

---

### Example 3: Two lenses looking at the same thing differently — reconcile via the actual finding

**Scenario**: reviewer and reviewer-deep disagree on whether a data transformation is correct.

reviewer (standards pass): "The `.map()` chain looks idiomatic and follows project patterns."
reviewer-deep (logic bounce): "The `.map()` chain reverses iteration order on the input, which produces wrong output when the input is not already sorted."

**Synthesis**: Both agents looked at the same code. They reached different verdicts because they applied different lenses. reviewer was checking idiom; reviewer-deep was checking behavior.

These lenses are not equal weight on the question of "does this code produce correct output?" reviewer-deep's lens is the authoritative one for that question. reviewer's lens is authoritative for the question "is this idiomatic?"

**Correct routing**: reviewer-deep's concern is the binding one for the correctness question. Investigate directly — does the `.map()` chain actually reverse order? If yes, it's a bug. Route back with the specific finding. reviewer's pass is still valid (the idiom is correct — the bug is in the caller's assumptions, not the pattern).

**Incorrect routing**: average the two verdicts, pick the majority, or treat them as the "same question from different angles" and let the more recent one win.

---

## The Reconciler's Role

The reconciler (`xtask/src/tasks/queue_reconciler.rs`) handles label-state contradictions mechanically:

- For labels with live ground truth (CI state): grounds the verdict in live CI, not label history
- For labels without live ground truth: uses GitHub timeline (later-applied wins)
- Strips stale labels automatically
- Does not make judgment calls

**The reconciler is not a synthesizer.** It resolves mechanical contradictions. It cannot assess whether a reviewer-deep finding is addressed by a different lens's pass. It cannot judge whether a diff-auditor bounce's premise is still valid after the builder's explanation in the commit message.

Judgment composition is orchestrator-level work. The reconciler produces a clean label state; the orchestrator reads that clean state and applies synthesis rules to decide what to route next.

See [LIVE_SIGNALS_VS_LABELS.md](./LIVE_SIGNALS_VS_LABELS.md) for what the reconciler does and does not own. See [OCTOPUS_CLUSTER.md](./OCTOPUS_CLUSTER.md) for the broader reconciliation dividend.

---

## Summary Checklist

When synthesizing multi-agent verdicts:

1. **What did each agent actually look at?** (premise check — not just what label they set)
2. **Have conditions changed since the verdict was applied?** (SHA drift, spec rewrites, prior bounces resolved)
3. **Is the bouncing agent's concern concrete (file:line) or generic ("seems risky")?**
4. **Which axis of the three-axis triangulation is failing?** (right thing / codebase needs / right way)
5. **Am I presenting prior verdicts neutrally, or am I priming toward a predetermined outcome?**
6. **For CI-pair labels: query live CI, don't trust the label.**

When in doubt: read the actual finding, not the verdict label. A verdict is a summary. The finding is evidence.

---

*Methodology grounded in memory entries: `feedback_take_judgment_on_verdicts`, `feedback_no_thumb_on_scale_in_prompts`, `feedback_verification_ladder`. Session history: PR #6748, #5543, #6780.*
