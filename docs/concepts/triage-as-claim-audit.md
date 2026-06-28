# Triage as Claim Audit

*Portable concept. Grounded in perl-lsp. See also: [slow-stochastic-compiler](slow-stochastic-compiler.md), [stochastic-ready-pipelines](stochastic-ready-pipelines.md), [shift-left-ladder](shift-left-ladder.md), [model-conformance](model-conformance.md), [human-corrects-substrate](human-corrects-substrate.md).*

---

## The distinction

Conventional triage asks: "which items should we work on, and in what order?" It treats the backlog as a task list and triage as a scheduling problem.

A stochastic pipeline generates a different problem. Issues arrive from scouts (human and automated), bug reports, external contributors, and stale half-finished investigations. Each issue is a **claim** — that a function behaves incorrectly, that a feature is missing, that a test is flaky, that a file path is wrong. Claims can be refuted, already fixed, duplicated across multiple issues, or genuine bugs requiring a spec.

In this environment, triage is not scheduling. It is a **claim audit**: working through the backlog to establish which claims are true, which are false, which are already resolved, and which are duplicates of each other. The output is not a ranked task list — it is a backlog with higher evidence quality. Future agents reading it find claims that have been tested against reality, not raw observations from the time of filing.

---

## The six outcomes

Good triage produces one of six outcomes per issue. These are exhaustive — every issue resolves to exactly one.

**1. Refuted**
The claim is false. The described behavior is not present in the current codebase. Evidence: reproduction attempt failed, the code path in question does not exist, the behavior described contradicts what a direct test shows. Close with documentation of the refutation and the evidence.

**2. Already fixed**
The behavior was real at filing time but has been corrected by a subsequent change. Evidence: a PR that directly or incidentally fixed the described behavior, a test that now passes that would have failed when the issue was filed. Close with the PR reference and a note on what fixed it.

**3. Duplicate**
The claim is real but another issue (open or fixed) already tracks it. The correct action is to link the issues and close one, not to resolve it twice. Two issues describing the same root cause from different angles should be consolidated to avoid divergent specs and redundant builder effort.

**4. Real bug with spec**
The claim is real, reproducible, and not tracked elsewhere. The triage outcome is to file a spec: write the acceptance criteria, the expected behavior, the test shape that would demonstrate the fix, and the blast radius. Apply `needs-plan-review`. The issue is now builder-ready after the plan-review pipeline completes.

**5. Deferred**
The claim is real but out of scope for the current roadmap, or blocked on a dependency that is not ready. The triage outcome is to label it deferred with a documented reason — not to silently leave it in the backlog with no signal.

**6. Broader pattern**
The claim reveals a class of issue rather than a single instance. The triage outcome is to promote the class to a hazard-class invariant (see hazard-class-invariants.md), file a spec-planner task to add the hazard row to relevant acceptance criteria, and close the individual issue as captured-in-class.

---

## What good triage leaves behind

Good triage produces **clean artifacts**, not a shorter list. For each closed issue:

- A documented disposition with evidence (not just a close action)
- For already-fixed: the PR reference and a sentence on what fixed it
- For refuted: the reproduction attempt and the evidence that contradicts the claim
- For duplicate: explicit links between all related issues and the surviving canonical issue
- For real-bug-with-spec: a spec packet in the issue body or linked `.spec/` directory
- For deferred: a documented reason and a tag that makes the deferral greppable
- For broader-pattern: a reference to the hazard-class entry that captured it

Future agents reading any of these issues should be able to determine in one read whether the issue is live, closed with cause, or superseded.

---

## 2026-06 campaign example

During the June 2026 autonomous campaign, approximately 47 issues were triaged over several sessions. The distribution across outcomes:

- ~18 already-fixed (identified by comparing issue claims against recent merged PRs)
- ~9 refuted (reproduction attempts failed on current codebase)
- ~7 duplicates (consolidated into canonical tracking issues)
- ~8 real-bug-with-spec (filed acceptance criteria, applied needs-plan-review)
- ~3 deferred (out of current roadmap scope)
- ~2 broader-pattern (promoted to hazard-class invariants)

The primary cost driver was **accuracy-scout passes** to verify whether named functions existed at named paths. Roughly 40% of issues named functions that had been renamed or moved since filing. A secondary cost driver was **reproduction attempts**: ~25% of claimed behaviors could not be reproduced on the current codebase without any obvious fix in the git log, suggesting the claim was imprecise rather than correct-but-fixed.

This distribution is typical. Most backlogs in active development have a large proportion of already-fixed and refuted items. Triage that does not check these classes produces a task list full of phantom work.

---

## Triage discipline

Five practices that keep triage from becoming phantom-work generation:

**Reproduce or disprove before spec-ing.** Write a test or run the described scenario before filing a spec. If the behavior cannot be reproduced, the claim is refuted. If it can, the reproduction attempt is the first evidence in the spec. Never write a spec for a claim that has not been checked against the current codebase.

**Cite evidence, not impressions.** "This looks like it might be a problem" is not a triage outcome. "The function `parse_block` at `crates/perl-parser/src/block.rs:142` returns `None` when given an empty input, where the acceptance criteria require `Some(Block { stmts: vec![] })`" is a triage outcome. The evidence must be specific enough for a future agent to verify or refute it without re-investigating from scratch.

**Write for future agents.** The triage comment is read by builders, reviewers, and orchestrators who do not have the triage session's context. Assume they will read only the issue body and the triage comment. Everything they need to understand the disposition must be in those two artifacts.

**Check related claims.** Before closing an issue as already-fixed, check whether related issues make the same claim. If they do, close them together with the same evidence. A PR that fixes issue #A often also fixes #B, #C, and #D — but the connection is only visible if triage looks for it.

**Update once.** Triage should leave each issue in a stable state. An issue that is triaged but then re-opened because the evidence was weak, re-triaged, re-closed, and then re-opened again is a sign that the initial triage did not produce a clean artifact. Better to leave an issue open with a note that reproduction was inconclusive than to close it and have it re-opened.

---

## Relation to other patterns

- **Slow stochastic compiler** (`slow-stochastic-compiler.md`) — triage is a pass in the pipeline: it transforms raw claims into evidence-backed artifacts. Like any pass, it can be wrong. The next pass (accuracy-scout, plan-reviewer) catches triage errors, not the triage agent itself.
- **Stochastic-ready pipelines** (`stochastic-ready-pipelines.md`) — issue claims are artifacts with reliability profiles. Triage is the process of applying confidence weights and routing to verification when stakes are high.
- **Shift-left ladder** (`shift-left-ladder.md`) — triage is upstream of spec and builder. Catching phantom work at triage (refuted, already-fixed) saves the full builder cost. Catching class patterns at triage (broader-pattern) prevents recurring builder rework.
- **Model conformance** (`model-conformance.md`) — when multiple issues make conflicting claims about the same behavior, triage is the process of finding the outlier and resolving the conflict against the current codebase.
- **Human corrects substrate** (`human-corrects-substrate.md`) — when triage is producing a high proportion of phantom work (the claims are systematically wrong), the problem is usually a substrate issue: a scout that files issues without checking current codebase state, or an accuracy layer that is not running. The human corrects the substrate, not individual triage outcomes.

---

## Specs need an owner state

A spec is an asset only if it is consumed. Every spec should carry an explicit owner state: `build-ready` | `blocked` | `deferred` | `refuted` | `superseded` | `contract-only`. A spec with no owner state is unconsumed inventory, not acceleration. Specs are cheap to produce early — but cheap-to-produce is not the same as free-to-leave-unreconciled. Pair early spec production with a periodic reconciliation sweep so spec debt does not silently become a shadow backlog.
