# ChatGPT Session 2 Analysis — Pipeline Design Critique

*External analysis of the perl-lsp swarm methodology pipeline. Session date: 2026-03-21.*
*Session 1 covered article corrections and story arc. Session 3 covered promotion paths and validator trust.*
*This session focused on the pipeline as a mechanical system — what it is, what it does well, what it still needs.*

---

## Context

The session presented the full pipeline design (Scout → Plan-Review → Build → Review+Improve → Green → Merge → Wisdom), the Era 7 s1 evidence (51+ merged PRs, 4/4 plan-review corrections, every reviewed PR improved), and asked for structural critique.

The analysis is captured here because external review consistently finds framings internal scouts miss.

---

## 1. "The Pipeline Is Real Now, Not Aspirational"

**The observation:** Every previous external analysis of this project described the pipeline aspirationally — "you could do X," "if you implement Y." This session was different. The evidence is in.

- 51+ PRs merged in Era 7 session 1
- 4/4 scout specs corrected by plan-review (wrong file, wrong function, already-fixed issues, wrong scope — all caught)
- Every reviewed PR received pushed improvements, not just comments
- Plan-review caught one issue that would have sent a builder to fix the wrong function entirely (1-line fix vs. risky refactor of wrong code)

**The framing shift:** "You have moved from building a methodology to having one. The pipeline isn't the plan. The pipeline is the operating record."

**What this means for documentation:** Claims like "plan-review adds value" should now be stated as past-tense evidence: "plan-review corrected 100% of scout specs in session 1." Numbers with dates, not claims about intended behavior.

---

## 2. Plan-Review Is Doing Real Correction (100% Correction Rate)

**The finding:** 4/4 is not a sample — it's the entire population for that session. That's not a success rate. That's a structural property.

The four corrections:
1. Scout: "C-style for is broken" → Plan-review: already fixed, issue closed (saved a wasted builder)
2. Scout: "qr<...> broken" → Plan-review: parser correct, scoped to tests-only
3. Scout: "fix parse_hash_or_block" → Plan-review: real bug was in calls.rs (different function)
4. Scout: "2 corpus files" → Plan-review: actually 8 files, every file path and function name was wrong

**The structural implication:** Scouts (haiku) are not capable of being reliably accurate at file paths and root cause. This is not a scout quality problem — it's a model tier limitation. The architecture accounts for it correctly: scouts are honest about uncertainty, plan-reviewers correct.

**The quotable line:** "You have a correction layer that works. The question is whether it's always invoked."

---

## 3. Deep Review Is Doing Real Improvement (Every PR Got Better)

**The finding:** Reviewer agents pushed fixes directly to PR branches and improved every PR they touched. This is not "review theater" — these are real bugs removed before merge.

Evidence from Era 7 s1:
- Regex hover: 6 bugs fixed by reviewer, tests 5→11
- Go-to-test navigation: 7 improvements, tests 39→47
- Test::More hover: signature fixes, tests +21
- Special vars: 7 issues fixed, tests +19
- Diagnostic code audit: ALL_CODES fixed, tests 80→143

**The structural implication:** The improvement reviewer role (distinct from the correctness reviewer) is doing something qualitatively different. It's not checking whether the code is right — it's upgrading the PR to a higher standard than the builder targeted.

**The quotable line:** "Every PR that went through improvement review shipped better than what the builder wrote. That's not a failure of builders — it's the pipeline working."

---

## 4. "Cheap Tokens, Expensive Control" — The Core Economic Insight

**The insight:** The naive model of AI-native development is that tokens are the cost center. Generate more code faster = more value. This is wrong.

The real cost structure:
- Token cost: near-zero per PR at current prices
- Control cost: human attention deciding what to merge, what to trust, what to reject

The pipeline is a **control cost reduction machine**, not a code generation machine. Every stage that catches an error before merge reduces the human attention needed to verify the output.

- Scout catches wrong problem → plan-reviewer corrects → builder doesn't waste tokens on wrong fix
- Plan-reviewer corrects root cause → builder doesn't implement wrong location
- Reviewer finds bugs → they don't reach CI → CI doesn't fail → human doesn't investigate
- Green gate → merge confidence → human doesn't re-verify

**The economic model:** The value of each stage is measured in human attention saved, not tokens consumed. A 2-minute plan-review that prevents a 30-minute builder on the wrong file saves 28 minutes of human-equivalent cost, even if the tokens cost the same.

**The quotable line:** "Code is cheap. Control is not. The pipeline is a control infrastructure — and that's its economic justification."

---

## 5. The Accuracy-Scout as the Missing Structural Layer

**The gap identified:** The pipeline has correction at the plan-review stage. But there is no systematic accuracy verification at the boundary between plan-review and builder, or between builder and reviewer.

Current flow:
```
Scout (rough) → Plan-Review (corrects) → Builder (implements) → Reviewer (improves) → CI (gates)
```

The problem: if a plan-reviewer fills a gap incorrectly (not catches a scout error — introduces a new one), the builder implements it without challenge. If a builder makes a silent error (wrong variable, off-by-one, missed edge case), the reviewer catches it — but only if the reviewer has enough context to know what "correct" looks like.

**The accuracy-scout concept:** A lightweight verification pass between stages that asks: "Does this match what the previous stage said it would do?" Not a full review — a structural match check.

- Between plan-review and build: "Does the issue description match what the plan-reviewer updated it to say?"
- Between build and review: "Does the diff match the spec in the issue?"
- Between review and green: "Do the test names and coverage match what was claimed?"

**Why this matters:** It closes the loop. Right now, the pipeline has stages but the stages don't verify each other's outputs mechanically. The accuracy-scout makes the handoffs verifiable.

**Status:** This is a structural gap. Not currently implemented. Recommend issue filing.

---

## 6. Label-Driven State Machine as the Mechanical Improvement

**The observation:** The current pipeline state is encoded in human memory and agent conventions. Labels exist (`builder-ready`, `needs-plan-review`, `needs-review`) but they are applied manually and not enforced as gates.

**The proposal:** Make labels the state machine. Every stage transition requires a label change. No agent proceeds without the correct incoming label. No stage completes without setting the correct outgoing label.

Proposed label sequence:
```
needs-scout → needs-plan-review → builder-ready → needs-review → needs-improvement → ready-for-ci → merged
```

Each label is:
- **Set by:** the agent completing the previous stage
- **Consumed by:** the agent entering the next stage
- **Verified by:** a hook or gate check before the agent is dispatched

**Why this matters:** Right now, a builder can be dispatched on an issue that has never seen plan-review. That's a control failure. With a label gate, dispatching a builder on a `needs-scout` issue is mechanically impossible — the orchestrator can't route it.

**What it prevents:**
- Builders on unreviewed specs (the "research first" anti-pattern that produced 0 PRs)
- Reviewers on draft PRs that aren't ready
- Merges of PRs without green CI
- Double-dispatch of the same issue to two builders

**Status:** Label convention exists informally. Formal state machine enforcement is a gap. Recommend: xtask command that validates label transitions before dispatch.

---

## 7. "Stages Are Mandatory, Rerunnable, and Version-Bound" — The Core Rule

**The insight:** Three properties make a pipeline stage trustworthy:

1. **Mandatory** — there is no path to the next stage that skips this one
2. **Rerunnable** — if the stage fails, you can rerun it without side effects
3. **Version-bound** — the stage output is tied to the input version (a review of v1 is not valid for v2)

Current state of each stage:
- Scout: rerunnable (yes), mandatory (no — builders often dispatched without it), version-bound (no)
- Plan-review: rerunnable (yes), mandatory (enforced by convention only), version-bound (no — a plan-review from week ago may be stale)
- Build: rerunnable (yes, worktrees are isolated), mandatory (yes — no merge without a PR), version-bound (yes — PR is tied to a branch SHA)
- Review: rerunnable (yes), mandatory (convention only), version-bound (partial — reviewer looks at current branch)
- Green: rerunnable (yes), mandatory (yes — merge blocked without it), version-bound (yes — CI runs on specific SHA)

**The gap:** Scout, plan-review, and review are not version-bound. A plan-review written against a stale spec is still labeled `builder-ready`. A review written before a builder's late amendment is still treated as valid.

**The fix:** Every stage output should record the input SHA/version it was written against. Reuse of a stage output after the input has changed should require re-running the stage.

**Status:** SHA-verification exists at the green/merge stage only. Needs extension upstream.

---

## 8. The Definitive Pipeline Design — 9 Stages

Synthesizing the current design with the gaps identified above:

```
[1] Scout       haiku    → Discovers, files rough spec. Honest about uncertainty.
[2] Plan-Review sonnet   → Corrects scout, fills gaps, outputs builder-ready issue.
[3] Build       sonnet   → Implements spec. Fixes forward. Worktree isolated.
[4] Draft PR    ops      → PR created as draft. Label: needs-review.
[5] Review      haiku    → Correctness check. Pushes fixes to branch.
[6] Improve     sonnet   → Quality upgrade. Pushes improvements to branch.
[7] Green       CI       → SHA-verified gate. Fail = back to step 5.
[8] Merge       ops      → Batch of 3. Squash. Ratchet corpus if parser.
[9] Wisdom      sonnet   → Retrospective. Memory update. Pattern capture.
```

Properties each stage must have (current gaps noted):
- Stages 1-2: need version-binding (gap)
- Stages 1-4: need label enforcement (partial)
- Stage 3: needs accuracy-scout handoff check (gap)
- Stages 5-6: need to be distinct (currently sometimes collapsed)
- Stage 7: SHA-verified (implemented)
- Stage 9: exists but not always invoked (convention gap)

---

## 9. Best Quotable Lines

These lines emerged from the session and are worth preserving for article/talk use:

**On the pipeline being real:**
"You have moved from building a methodology to having one. The pipeline isn't the plan. The pipeline is the operating record."

**On tokens vs. control:**
"Code is cheap. Control is not. Every stage of your pipeline is a control cost reduction, not a code generation step."

**On plan-review:**
"You have a correction layer that works. The question is whether it's always invoked."

**On review:**
"Every PR that went through improvement review shipped better than what the builder wrote. That's not a failure of builders — it's the pipeline working."

**On stage properties:**
"Stages are mandatory, rerunnable, and version-bound. Right now yours are mandatory-ish, rerunnable, and version-unbound upstream of CI."

**On the label state machine:**
"Labels aren't just metadata. They're the state machine. If you enforce them mechanically, you close the control loop."

**On accuracy:**
"The pipeline catches errors at each stage. But stages don't verify each other's outputs. That's the missing structural layer."

---

## 10. What Still Needs Filling In

These are open questions or gaps identified in the session that don't have complete answers yet:

### Label Invalidation
When a PR is amended after review, should the review label be revoked automatically? If yes, how? Options:
- Hook that removes `needs-improvement` label on push after review
- Reviewer re-runs on diff since last review tag
- No invalidation — reviewers check diff explicitly

Currently unresolved. Default behavior is no invalidation (stale reviews can pass as valid).

### Receipts as Stage Outputs
The concept of a "receipt" — a verifiable artifact that a stage completed — exists informally (memory files, CI results). A formal receipt would be:
- Immutable (can't be edited after creation)
- Content-addressed (tied to input SHA)
- Machine-readable (can be queried by orchestrator)

Currently receipts are PR comments and memory files — human-readable but not machine-queryable.

### Version-Binding Upstream of CI
CI runs are SHA-verified (built into GitHub Actions). But plan-review and build stage outputs are not. If a plan-review says "the bug is in calls.rs line 47" and then someone else merges a PR that changes calls.rs line 47, the plan-review is now stale — but still labeled `builder-ready`.

Fix would be: plan-review includes a `based-on: <SHA>` field. Builder checks SHA before starting. If stale, requests re-review.

### Accuracy-Scout Implementation
The accuracy-scout concept (stage 2.5 and 4.5 in the pipeline) needs a concrete prompt design:
- What does it read? (previous stage output + current state)
- What does it check? (structural match, not quality)
- What does it output? (pass/fail + specific mismatch if fail)
- When is it invoked? (always? only on high-risk changes?)

Currently no implementation exists.

---

## Session Meta-Note

This analysis was generated from a conversation where the swarm pipeline was presented and critiqued externally. The key value of external analysis is not that it finds errors internal scouts missed — it's that it names things. "Cheap tokens, expensive control" is not a new insight; it was already encoded in "code is cheap, trusted change is not." But naming the economic model explicitly makes it easier to apply.

The session confirmed: the pipeline is real, the evidence is solid, and the remaining gaps are mechanical (enforcement, version-binding, accuracy verification) rather than conceptual (the concepts are correct).

---

*Captured: 2026-03-21. Related: feedback_chatgpt_article_corrections.md, feedback_chatgpt_session3_analysis.md, feedback_pipeline_fix_forward.md, project_era7_session1.md*
