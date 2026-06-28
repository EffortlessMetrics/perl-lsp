# Orchestration Roles

> **This is a contract doc.** Agents load it to know their tier, constraints,
> and required output shape. Keep it tight. For design rationale see
> [docs/reference/ORCHESTRATION_DOCTRINE.md](../reference/ORCHESTRATION_DOCTRINE.md).

---

## Core Principle

> **Do expensive work only after cheap agents have made the work obvious.**

Classification, fact-checking, and de-duplication are haiku work. Architecture
conflicts, ambiguous root causes, and adversarial release review are sonnet or
opus work. Routing the wrong tier to a task wastes budget and dilutes signal.

---

## Conveyor Diagram

```
Issue
  |
  v
[classification]     ← haiku: fast triage, evidence-required
  |
  v
[adversarial        ← haiku: accuracy, research, oppositional, diaboli,
 verification]            architecture, maintainer-issue
  |
  v
[scoped plan]        ← haiku: spec-planner, red-tdd
  |
  v
[builder]            ← sonnet: implement, test, verify, PR
  |
  v
[independent        ← haiku: green-tdd, reviewer, maintainer-pr, refactor-planner
 verification]       ← sonnet: green-refactor, reviewer-deep
  |
  v
[CI proof]           ← haiku: green-ci, pr-responder
  |
  v
[merge / close      ← haiku closer with merge-base proof
 / defer]
  |
  v
[cleanup]            ← haiku: wisdom, label hygiene
```

---

## Multi-Pass Rule

High-wrong-cost claims require **independent passes before action**:

| Claim class | Why it is high-cost-wrong | Required pass count |
|-------------|--------------------------|---------------------|
| Superseded / already-landed / duplicate-of-merged | Triggers false close (29-PR class, 2026-06-06) | 2 independent agents |
| Release-readiness | Triggers premature release or wrong version | 2 independent agents |
| Sync / rebase needed | Triggers unnecessary force-push or history rewrite | 2 independent agents |
| Security invariant change | Triggers unsafe boundary regression | 2 independent agents |
| Already-fixed | Triggers close of still-open issue | 2 independent agents |

A single agent's assertion — even with evidence — does not satisfy the
multi-pass rule. Two *independent* agents must both reach the same conclusion
before action is taken.

---

## Role Catalog

### Scout

**Purpose:** Broad discovery — find the problem, file a roughly-right spec.
Honest about uncertainty; plan-reviewers correct.

**Model tier:** haiku

**Constraints:**
- Read-only by default; no commits, no merges, no closes
- Output JSON or short tables; no prose conclusions without evidence
- Flag uncertainty explicitly; "roughly right > confidently wrong"
- May spawn sub-searches; may not spawn builders

**Required output schema:**
```json
{
  "claim": "string — the finding",
  "evidence": ["file:line or URL"],
  "confidence": "high | medium | low",
  "recommended_action": "string",
  "blocked_by": ["issue or PR number, or empty"]
}
```

---

### Verifier (accuracy, research, oppositional, diaboli, architecture, maintainer)

**Purpose:** Challenge the scout's spec before build starts. Each sub-role
targets one axis: mechanical facts, external semantics, approach soundness,
existence justification, structural fit, or project alignment.

**Model tier:** haiku

**Constraints:**
- Read-only; no commits, no merges, no closes
- Each verifier reads the previous verifiers' outputs (sequential within gate)
- One routing decision per pass: sign-off OR bounce — never both
- No conclusions without cited evidence

**Required output schema:** same as scout `{claim, evidence, confidence,
recommended_action, blocked_by}` plus a `verdict` field:
```json
{
  "verdict": "PASS | NEEDS_CORRECTION | BLOCK",
  "routing_label": "accuracy-reviewed | needs-plan-review | ...",
  "claim": "...",
  "evidence": [...],
  "confidence": "...",
  "recommended_action": "..."
}
```

---

### Builder

**Purpose:** Implement the plan. Check out the impl branch (spec + red tests),
make the failing tests green, verify, open PR.

**Model tier:** sonnet

**Constraints:**
- One PR objective — do not bundle unrelated cleanup
- Fix forward on small gaps; bump back if structural
- Run `just pr-fast` before pushing; never skip hooks
- Clean up worktree after PR is open
- Evidence required for any claim in the PR body

**Required output schema:**
```json
{
  "changed_files": ["path"],
  "tests_run": "cargo test -p <crate> output summary",
  "ci_status": "passing | failing | unknown",
  "known_gaps": ["description or empty"],
  "cleanup_done": true,
  "pr_url": "https://..."
}
```

---

### Reviewer

**Purpose:** Standards check (haiku pass) then correctness check (sonnet deep
pass). The haiku reviewer fixes banned patterns, scope drift, and formatting
directly on the branch. The deep reviewer validates logic, edge cases, and
regressions.

**Model tier:** haiku (standards); sonnet (deep / correctness)

**Constraints (standards / haiku):**
- Fixes pushed directly to PR branch; no "LGTM-only" passes
- One routing decision per pass
- No conclusions without cited diff evidence

**Constraints (deep / sonnet):**
- Focused validation — one PR, one pass
- Correctness only: edge cases, regressions, logic errors
- Evidence required; speculative findings noted as such

**Required output schema:** verdict + findings list:
```json
{
  "verdict": "APPROVE | REQUEST_CHANGES",
  "findings": [
    {"file": "path", "line": 42, "severity": "bug|style|suggestion", "claim": "..."}
  ]
}
```

---

### CI Watcher

**Purpose:** Verify CI is genuinely green on the current HEAD SHA — not a
stale result from an earlier push. Fix mechanical CI failures.

**Model tier:** haiku

**Constraints:**
- Read-only signal collection; may push fixes for mechanical failures (fmt, clippy)
- Never declare CI green from a cached label — query live SHA
- One routing decision: `ci-green` OR `needs-ci-fix`

**Required output schema:**
```json
{
  "head_sha": "40-char commit hash",
  "checks_passed": 17,
  "checks_failed": 0,
  "checks_pending": 0,
  "verdict": "GREEN | FAILING | PENDING",
  "routing_label": "ci-green | needs-ci-fix"
}
```

---

### Closer

**Purpose:** Close issues that are superseded, already landed, or duplicate of
a merged PR. Governed by [CLOSE_PROOF_POLICY.md](CLOSE_PROOF_POLICY.md).

**Model tier:** haiku (verification pass); sonnet only when content-overwritten
analysis requires synthesis

**Constraints:**
- **Never close without merge-base proof** — see CLOSE_PROOF_POLICY.md
- Must satisfy multi-pass rule for all high-wrong-cost claim classes
- Port-before-close: content must reach a canonical surface before source closes
- Wrong closes trigger reopen-or-reland with trail documented

**Required output schema:**
```json
{
  "closed_item": "issue or PR number",
  "canonical_landed_artifact": "PR URL or file path",
  "merge_base_proof": "git merge-base --is-ancestor <commit> <canonical-main> output",
  "comment_url": "https://github.com/.../issues/N#issuecomment-M"
}
```

---

### Cleanup

**Purpose:** Worktree pruning, stale label hygiene, wisdom consolidation,
memory recalibration after a merge batch.

**Model tier:** haiku

**Constraints:**
- Read-only for label decisions; may prune stale worktrees and stash entries
- Wisdom consolidation: captures learning into durable artifacts, not inline comments
- Never delete a worktree with uncommitted work

**Required output schema:** summary table of actions taken:
```json
{
  "worktrees_pruned": 3,
  "labels_cleaned": ["list"],
  "wisdom_entries_added": 1,
  "alerts": ["any anomaly worth routing"]
}
```

---

### Release Captain

**Purpose:** Oversee the release pipeline — version bump, changelog, tag,
publish. Adversarial self-review of release artifacts before any tag lands.

**Model tier:** sonnet (synthesis + adversarial review); opus **only** for
architecture conflicts or ambiguous root causes that would affect the release
decision. Opus is rare — the cost is only justified when a decision cannot be
made confidently at sonnet tier.

**Constraints:**
- Evidence required at every step — no "looks good" without diff evidence
- Two-pass release-readiness check (multi-pass rule)
- `just semver-check` must pass before any version bump
- Tag is the point of no return: never tag without green CI on the release commit

**Required output schema:**
```json
{
  "version": "0.N.M",
  "changelog_entry": "URL or inline summary",
  "ci_sha": "40-char commit hash",
  "semver_check": "PASS | FAIL",
  "tag": "v0.N.M or null",
  "release_url": "https://... or null"
}
```

---

## Opus: When and Only When

Opus is reserved for three narrow cases — it is expensive and its use should
be deliberate:

1. **Architecture conflicts** where sonnet agents disagree and a tiebreaker
   judgment is needed that will affect multiple downstream PRs.
2. **Ambiguous root cause** in a production incident where sonnet analysis has
   stalled or produced contradictory conclusions.
3. **Adversarial release review** where the release captain needs an
   independent adversarial pass before tagging a major version.

Opus does **not** produce code. It produces decisions. A decision from opus
should unlock a clear next action for a sonnet or haiku agent.
