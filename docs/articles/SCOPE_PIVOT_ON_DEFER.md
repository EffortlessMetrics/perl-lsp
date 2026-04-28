# Scope-Pivot on DEFER: Reading Agent Verdicts as Advisory, Not Authoritative

**Date**: 2026-04-19
**Session**: Wave G1 collapse (5 PRs merged, 74 → 49 published crates)
**Cross-references**: [VERDICT_OVERRIDE_PATTERNS.md](../contributing/VERDICT_OVERRIDE_PATTERNS.md), [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md)

---

## TL;DR

A verification agent that returns DEFER is almost always saying "at the proposed scope, this is too risky/premature/noisy right now." That "at the proposed scope" qualifier is implicit and easy to miss. If you shrink the scope, the defer-rationale frequently evaporates — the agent wasn't asked to consider smaller alternatives, so it didn't.

During the 2026-04-19 Wave G1 collapse session on perl-lsp, two diaboli DEFER verdicts (on issues #4497 and #4499) were reversed by the orchestrator after re-examining premises at reduced scope. Both PRs merged cleanly the same day. Had both DEFERs been honored blindly, the session would have ended at 64 published crates instead of 49 — a 30-40% productivity loss from two unnecessary deferrals.

This article names the pattern — **scope-pivot on DEFER** — and argues that AI verification-agent verdicts deserve a specific kind of critical reading. The agents are conservative-by-design, which is correct behavior; the orchestrator's job is to test whether the conservatism still binds when the problem is re-framed smaller.

---

## Why Verdicts Feel Authoritative (But Aren't)

In a well-designed verification pipeline, each agent is specialized: accuracy-scout checks file paths, research-verifier checks external claims, oppositional-planner challenges approach, advocatus-diaboli judges existence, architecture-reviewer checks structural fit, maintainer assesses project-vision alignment, and deep-reviewer is the final correctness gate. Each returns a labeled verdict. Labels accumulate. When all layers sign off, the orchestrator routes to builder or ops.

The failure mode is treating a DEFER/CLOSE verdict at any layer as a blocker instead of input. Verdicts are advisory. The orchestrator owns the routing decision, and any verdict returned at any layer is a hypothesis — bound to the scope, timing, and premises available to that agent when it ran. Change the scope and you change the hypothesis.

Two illustrative cases from the 2026-04-19 session:

## Case 1 — `#4497`: `cargo public-api` diff ratchet

**Original scope:** ratchet the public API surface of all 74 surviving published crates after the microcrate collapse.

**Diaboli verdict:** DEFER until post-Wave-G3.

**Diaboli's rationale:**
- Baseline files: 74 text snapshots of `cargo public-api --simplified` output.
- During G-wave collapse, each wave (G1, G2, G3) absorbs crates. Absorbed crates disappear from the baseline list.
- Expected regenerations through the collapse program: 15+ baseline files per wave × 3 waves = 45+ mechanical baseline regenerations.
- That churn pollutes PR history and trains reviewers to ignore the gate.

The defer-rationale was **valid at the 74-crate scope.** The orchestrator's first instinct was to honor it.

**Scope pivot:** read the scope again. The issue body said "ratchet the surviving public crates." What counts as "public"? Users of perl-lsp as a library consume five facade crates: `perl-lsp-rs`, `perl-parser`, `perl-uri`, `perl-dap`, `perllsp`. The other 69 published crates are internal implementation support that users don't bind against. Ratcheting their API surface is noise, not signal.

**Facade-only re-scope (5 crates):**
- 5 baseline files, not 74.
- Facades are **designed not to change during satellite collapse** — that's the whole point of the facade/core split landed in Wave D and Wave F.
- Expected baseline regenerations through G1/G2/G3: 0 (the absorbed crates are all behind the facade, and the facade's public surface is the stable contract).
- The churn concern evaporates. The gate is never false-positive.

**Outcome:** orchestrator reversed the DEFER in writing on the issue, routed back to plan-reviewer with the revised scope. Plan-reviewer refined the spec. Builder shipped PR #4504. Merged same day.

## Case 2 — `#4499`: offline manifest-lint `xtask publish-manifest-check`

**Original scope:** six checks — keyword count ≤ 5, no wildcard dependencies, `license` field present, `description` present, `repository` present, allowlist drift.

**Diaboli verdict:** DEFER.

**Diaboli's rationale:**
- Overlap with `cargo package` — keyword count, wildcard deps, description, and repository are already caught by `cargo package`.
- Allowlist-drift overlaps with an existing Python `scripts/publish-topo.py --check-drift` invocation in `.github/workflows/publish-dry-run.yml`.
- During active G-wave collapse, the allowlist-drift check would fire on every expected PR because the publish allowlist is changing.
- "~80% of real failures caught" claim from the issue body is overstated; real catch rate is closer to 33% (two silent failure modes: allowlist drift, missing LICENSE).

Again, valid at the original scope.

**Scope pivot:** ask what the two silent failures are that `cargo package` doesn't catch.
- **Allowlist drift** — a published crate is listed in `xtask/published-crate-baseline.txt` but missing from `[workspace.metadata.publish.allow]` (or vice versa). `cargo package` doesn't know about these files; the existing Python check catches it but only on nightly.
- **Missing LICENSE** — `cargo package` only enforces license at publish-time. If a new crate lands in the allowlist without a LICENSE, CI doesn't notice until release day.

**Re-scope (2 checks, plus consolidation):**
- Consolidate the existing Python drift check into xtask (net LOC reduction — removing the Python invocation).
- Add LICENSE-present check (real silent failure mode).
- Drop the four checks `cargo package` already handles.
- The allowlist-drift-during-G-wave concern **also** evaporates: serial-merge collapse PRs update both files together; drift is internally consistent per PR.

**Outcome:** orchestrator reversed the DEFER, routed back to plan-reviewer with the revised scope. Builder shipped PR #4505. Merged same day.

## The Pattern

1. Agent returns DEFER on an issue at scope *S*.
2. Orchestrator asks: "does this defer-rationale still hold at reduced scope *S'* ⊂ *S*?"
3. If **no** (rationale depends on features of *S* not in *S'*): reverse the DEFER in writing, re-scope the issue, route back to plan-reviewer.
4. If **yes** (rationale is structural, independent of scope): honor the DEFER.

The pivot is not about disagreeing with the agent. The agent was answering a specific question — "should this ship as filed?" — with the information it had. The orchestrator is asking a different question — "is the smallest version of this shippable?" — with broader context. Both answers can be correct; they're different answers.

## Why This Works With Conservative Agents

Verification agents should be conservative. A DEFER is cheap to recover from; a merged broken PR is not. The conservative bias is the feature, not the bug.

But the asymmetry of cost creates a specific failure mode: the agent's incentive is to DEFER when in doubt. Doubt is common. A pipeline that routes agent DEFER verdicts directly to issue-blocking creates a slow-path dominance — any issue the agents aren't fully confident in accumulates DEFER and waits.

The scope-pivot tool is exactly designed for this asymmetry. It says: before we accept the DEFER slowdown, let's check whether a smaller version is already confident-enough to ship. Often it is.

## Boundary: DEFER vs CLOSE

Not every verdict is a candidate for scope-pivot. A close-adjacent example from the same session: **#4498** (per-crate registry dry-run). Research-verifier established that the core premise was false — `cargo publish --dry-run` is entirely local and doesn't contact crates.io. No amount of scope-pivoting fixes a broken premise.

Diaboli's verdict on #4498: CLOSE. Orchestrator action: close #4498, file a new issue (#4499) with the correct premise ("consolidate existing Python drift check + add LICENSE-present"), thread a link from #4498 to #4499 so the broken-premise catch becomes a teaching artifact.

CLOSE on broken premise → file a new issue with the right premise. Don't repurpose the old issue; the comment trail has teaching value.

DEFER on valid-but-scoped concerns → scope-pivot, route back to plan-reviewer on the same issue.

## Recording the Override

Whenever the orchestrator reverses a verdict, the reversal must be written on the issue as a comment. Future agents (and future humans) need to see the explicit trail: which verdict was reversed, what scope change triggered the reversal, which specific rationale from the original verdict evaporated under the new scope. This is the difference between *judgment* and *silence*. Judgment is auditable.

The two reversal comments from 2026-04-19 are visible on #4497 and #4499. Both explicitly name the original DEFER rationale and show why the re-scope removed it. Six months from now, when someone asks "why didn't we honor the diaboli verdict here?", the answer is in the PR history, not in someone's memory.

## What This Doesn't Mean

- **Not**: every DEFER is wrong. Some are structural. Honor those.
- **Not**: the pipeline is broken. The pipeline worked — diaboli made conservative-and-correct verdicts at the scope they were asked to evaluate.
- **Not**: run the agents less. They caught real bugs throughout the session (see [per-layer ROI article](VERIFICATION_LADDER_PER_LAYER_ROI.md)).
- **Yes**: when a DEFER shows up, check whether scope-pivot eliminates the rationale before waiting days or weeks.

## Related

- [VERDICT_OVERRIDE_PATTERNS.md](../contributing/VERDICT_OVERRIDE_PATTERNS.md) — operator playbook (this article's how-to companion)
- [../forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) — full session retrospective
- [DOWNSTREAM_CATCHES_UPSTREAM.md](DOWNSTREAM_CATCHES_UPSTREAM.md) — companion pattern (downstream layers catch upstream-layer errors)
- [LAYERED_VERIFICATION.md](../project/protocols/layered-verification.md) — theory
