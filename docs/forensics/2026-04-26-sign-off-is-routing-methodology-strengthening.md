# Sign-Off IS Routing — Methodology Strengthening

**Window**: 2026-04-26 session (Saturday morning UTC, ~02:00–06:00 EDT)
**Audience**: orchestrator, anyone evolving sign-off agents (reviewer, maintainer-pr, refactor-planner, deep-reviewer, diff-auditor, green-ci, etc.)
**Purpose**: capture the #6780 incident, the methodology-strengthening fix shipped in PR #6808 + #6811, and the "every PR potentially problematic" disposition that prevents recurrence.

---

## The incident — PR #6780

PR #6780 (`fix(vscode-extension): remove redundant onStartupFinished`) was merged 2026-04-26 05:17 UTC carrying two substantive blocking bugs:

1. **Wrong language reference**: `docs/performance-activation-fix.md` referenced `onLanguage:idris` instead of `onLanguage:perl` (cargo-cult from another extension's docs)
2. **Missing manifest change**: PR title claimed a vscode-extension fix but the diff only added the doc — the actual `vscode-extension/package.json` was never modified

The standards reviewer agent caught both at 04:46 UTC and posted a "NEEDS BUILDER FIX" comment with explicit blockers. **But it ALSO applied `review-reviewed`** simultaneously with `needs-builder-fix`. The conflicting labels confused the merge gate, and a manual merge proceeded carrying the unfixed bugs.

Cleanup PR #6809 (correcting `idris` → `perl` and applying the manifest change) merged 05:31. Then PR #6812 added the lazy debug activation (`onDebugResolve:perl` + `onDebugInitialConfigurations`) and purged stale `onCommand:*` activation tests across 3 test files — the principled completion of the original fix.

---

## Root cause — sign-off and routing are not separate categories

The methodology had been treating sign-off labels (`<gate>-reviewed`) and routing labels (`needs-*`) as two label categories that could in principle coexist. That framing allowed the #6780 contradiction (`review-reviewed` AND `needs-builder-fix` simultaneously) because the labels weren't structurally exclusive.

The user's reframing during the session was the load-bearing fix:

> "signoff is signoff. It's signoff *or* bouncing back, not both."
>
> "Across all agents. like, signoff is one of the routing decisions."

The principle is **one routing decision per pass**, with sign-off as one of the options. Each agent's pass terminates with exactly one outcome:

- **Gate clean** → apply `<gate>-reviewed` (and only that)
- **Mechanical fix applied** → push fix; post-fix is clean → apply `<gate>-reviewed`
- **Bounce back (blocker found)** → apply the appropriate `needs-*` label (and ONLY that, NOT the sign-off)

The mutual exclusion is structural — they're the same decision with different outcomes — not a separate "no overlap" rule layered on top.

---

## What landed

### PR #6808 — agent definitions + CLAUDE.md

- New CLAUDE.md key principles: master-green directive, sign-off-as-routing rule, no-needs-*-on-merge gate, external-PRs-same-gates rule
- `reviewer.md` — sign-off-vs-routing rule applied universally across all sign-off agents
- `ops.md` — master-green protocol (workspace-wide CI verification before merge)
- `diff-auditor.md` — cross-PR source-file contamination check (extends prior `.hermes/`-only watch) + master-green guard

### PR #6811 — 4 key skill playbooks

The principle in agent definitions doesn't enforce itself at runtime — the skill playbooks the agents invoke at decision points must enforce it too.

- `ops-check-queue.md` — filter requires `mergeStateStatus = CLEAN` and excludes any active `needs-*` label
- `ops-merge-batch.md` — pre-merge requires CLEAN + no `needs-*` + workspace-wide CI SUCCESS; new step 8 re-verifies master green after each batch of 3
- `reviewer-decide.md` — operating principle codified at top; new explicit "Blocker found → send back WITHOUT sign-off" branch
- `diff-audit-comment.md` — operating principle codified; pre-comment checks for cross-PR contamination + master-green guard

### PR #6812 — vscode-extension cleanup

The principled completion of the original #6780 fix:

- Deleted `docs/performance-activation-fix.md` (always the wrong shape — proposed `onCommand:*` events that VS Code 1.74+ doesn't require, dropped Gherkin + walkthrough activation)
- Added `onDebugResolve:perl` + `onDebugInitialConfigurations` for cold-start debug flows (replacement for the lost `onStartupFinished` coverage)
- Purged stale `onCommand:*` activation event tests across 3 test files (configuration.test.ts, commands.test.ts, podPreview.test.ts)
- Final activationEvents shape: 5 lazy events covering language + walkthrough + debug, no startup, no per-command

---

## The "every PR potentially problematic" disposition

The user's clarification on what makes the methodology work:

> "By giving proper agentic oppositional review and etc. that enables us to properly improve the code"

The principle isn't "be suspicious for the sake of suspicion" — it's that **agentic oppositional review IS the quality mechanism**. Every gate substantively looking for problems is how the methodology catches issues that would otherwise slip through. The #6780 case is the textbook failure: a gate that found problems then signed off anyway lost its function as a gate.

The corollaries for any sign-off agent:

- The right default posture is "I'm going to find something concrete to flag" — not "approved unless something obvious is wrong"
- "Clean / nothing to flag" should be a rare verdict, not the modal one
- Sign-off means: "I substantively looked, applied my gate's specific checks, and the PR survived the scrutiny" — not "I ran a quick scan and didn't trip"
- Mechanical box-checking outputs (✅ banned patterns ✅ title format ✅ scope) without any concrete observation = the gate didn't actually do its job

This disposition is now codified in the diff-auditor and reviewer skills (PR #6811) and propagates to other sign-off agents by reference.

---

## Reconciliation of historical contradictory state

The reviewer-as-routing rule applied retroactively required cleaning up PRs that were already in the contradictory state. A reconciliation sweep processed 19 PRs that had `review-reviewed` AND `needs-builder-fix` simultaneously:

- All 19 stripped of post-builder sign-off labels (review, maintainer-pr, refactor-planner, green-refactor, deep, ci-green, diff-audited, merge-ready) where the `needs-builder-fix` indicated the gate hadn't actually cleared
- Pre-build verification labels (accuracy-reviewed, research-reviewed, etc.) preserved — those are issue-side state, not affected by post-build gate contradictions
- `needs-*` routing labels preserved as authoritative state

Pattern observed: ~10 of 19 contradictory PRs were also fresh-root strand cases (no merge-base with master) — separate operational issue documented in the fresh-root forensics.

---

## Anti-patterns this prevents going forward

- **The #6780 pattern**: gate finds blockers, applies sign-off anyway, manual merge proceeds carrying bugs
- **Phantom merge-readiness**: PRs with all sign-off labels but active `needs-*` reach ops as merge candidates
- **Stale sign-offs surviving fixes**: when a builder addresses a `needs-*`, the prior gate sign-offs persist and the PR appears more reviewed than it is
- **Soft-merge-bypass**: an operator skim of labels misses that one says "stop"

---

## How the principle applies to specific agents

| Agent | Sign-off label | Bounce label |
|-------|---------------|--------------|
| accuracy-scout | `accuracy-reviewed` | (issue-side; correction comments) |
| research-verifier | `research-reviewed` | (issue-side; flag false claims) |
| oppositional-planner | `oppositional-reviewed` | (issue-side; raises challenges, doesn't bounce) |
| advocatus-diaboli | `diaboli-reviewed` (with BUILD/DEFER/CLOSE verdict) | (issue-side; verdict in comment) |
| architecture-reviewer | `architecture-reviewed` | (issue-side; comment) |
| maintainer-issue | `maintainer-issue-reviewed` (with ALIGNED/DEFERRED/OUT OF SCOPE) | (issue-side; verdict in comment) |
| plan-reviewer | `plan-reviewed` (sets `builder-ready`) | (rejects approach via comment + iteration) |
| spec-planner | `spec-reviewed` | (issue stays builder-ready until spec ready) |
| red-tdd | `red-tdd-reviewed` | (escalates if spec doesn't admit tests) |
| spec-test-code-match | `spec-match-reviewed` | `needs-red-tdd-fix` OR `needs-spec-fix` |
| green-tdd | `green-tdd-reviewed` | `needs-builder-fix` |
| reviewer | `review-reviewed` | `needs-builder-fix` |
| maintainer-pr | `maintainer-pr-reviewed` | `needs-builder-fix` |
| pr-responder | `pr-responded` | `needs-builder-fix` |
| refactor-planner | `refactor-planner-reviewed` (+ `green-refactor-reviewed` if no plan) | (recommends, doesn't bounce) |
| green-refactor | `green-refactor-reviewed` | (executes plan, doesn't bounce) |
| reviewer-deep | `deep-reviewed` | `needs-builder-fix` |
| green-ci | `ci-green` | `needs-ci-fix` |
| diff-auditor | `diff-audited` | `needs-diff-fix` |
| ops | (merges; doesn't sign off) | (refuses merge if `needs-*` present) |

For PR-side gates, sign-off and `needs-*` are exclusive. For issue-side gates, the verdict is captured in the comment and the sign-off label means the verdict was rendered (even if BUILD/DEFER/CLOSE varies in content).

---

## Related forensics + memory entries

- `2026-04-25-defense-in-depth-verification-architecture.md` — the gate ladder context this principle operates within
- `2026-04-25-forensics-as-prompt-fragments-architecture.md` — how the principle propagates via in-repo prompt material
- `feedback_label_skill_silent_failure.md` — orchestrator-direct vs. agent-reported label apply success rates (~100% vs. ~20%)
- `feedback_take_judgment_on_verdicts.md` — how to synthesize across multiple lens-layers
- PR #6808 commits — methodology rules
- PR #6811 commits — skill playbook enforcement
- PR #6812 commits — the principled cleanup of the originating #6780 incident

---

## Applies to

Reference this doc when:
- Spawning any sign-off agent (the rule applies universally)
- Auditing PRs in contradictory label state (the reconciliation pattern)
- Onboarding a new sign-off agent class (extend the table above + apply the same exclusion rule)
- Reviewing a PR that "got through" with bugs after multiple sign-offs (likely a new instance of the #6780 pattern; check label history)
- Designing the next merge-time enforcement layer (the principle suggests CI gates that fail when both labels coexist would be the natural next step)
