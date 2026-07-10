# Modern Claude Operating Model

> **PR1 artifact of the CLAUDE.md modernization migration.** This doc captures the
> target operating model that the thinned root [`CLAUDE.md`](../../CLAUDE.md) routes
> into. It does not change agent behavior, CI, labels, or the pipeline defined in
> [PIPELINE_GATES.md](../reference/PIPELINE_GATES.md) — it names the levels, truth
> hierarchy, and delegation discipline that already-existing doctrine implies, so the
> router doc can stay short without losing any invariant.
>
> This is a sibling to the repo-role contract in
> [docs/swarm/operating-model.md](operating-model.md) (which repo — `perl-lsp-swarm`
> vs `perl-lsp` — a change targets) and orthogonal to it: that doc answers "which
> repo," this doc answers "which level, which truth, who does the work."

## Why this doc exists

perl-lsp's orchestration model (see [ORCHESTRATION_DOCTRINE.md](../reference/ORCHESTRATION_DOCTRINE.md)
and [MAINTAINER_AGENT_DOCTRINE.md](../reference/MAINTAINER_AGENT_DOCTRINE.md)) has
consolidated from many single-purpose agents into fewer, longer-lived agents traversing
gate-shaped checkpoints (see the *Orchestration model* section of root CLAUDE.md). That
consolidation raises a question the gate table alone doesn't answer: **at what
granularity does an agent's context stay valid, and what should it trust when sources
disagree?** This doc answers both.

**The control plane is the binding constraint.** Codegen is cheap; the CI/merge
control plane (ripr, Codecov-patch, the serial fmt/clippy meta-gate, main-green) is
the bottleneck — and the bottleneck **migrates upward** as throughput improves
(codegen → compile → CI → merge → API → reconciliation → reviewer: once one layer
stops being the constraint, the next one up becomes it). Treat infra investment
(cached builds, idempotent bulk ops, current-main preflight, durable agent receipts,
API-aware write queues) as product velocity, not support work — it is usually the
actual bottleneck, not the code being generated.

---

## Three levels

Work at perl-lsp happens at three nested levels. Confusing them is a recurring failure
mode — a session-goal agent that thinks it owns a multi-PR mission scope-creeps; a
program-level agent that thinks it owns one session's transcript loses continuity across
compaction.

### 1. Program (multi-PR mission)

A program is a mission that spans many PRs, potentially many sessions, and outlives any
one agent's context window. Program state lives in
[`.perl-lsp/goals/`](../../.perl-lsp/goals/) — machine-readable TOML manifests (see
`active.toml`, `compiler-program.toml`, `lsp-freshness.toml`) that record objective,
end-state, claim boundaries, lane ownership, and status-doc pointers. A program is
*not* tracked in any single agent's memory or conversation — it is tracked in the
manifest, which any agent (fresh or long-running) can re-read to re-establish context.

### 2. Work item (one issue, one PR, one merge)

A work item is the unit the [7-gate pipeline](../reference/PIPELINE_GATES.md) operates
on: one GitHub issue through spec, build, review, and merge. Work-item state lives on
GitHub (issue/PR labels, comments) and optionally in `.spec/<issue#>-<slug>/` files
(checklist.md, acceptance.md, context.md — see
[SPEC_TEMPLATE.md](../reference/SPEC_TEMPLATE.md)). A work item is scoped to "one
change, one proof, one PR" (see *Work discipline* below) — it should never silently
grow into a second, unrelated PR's worth of change.

### 3. Session goal (one PR, one board-transition)

A session goal is the narrowest level: what one agent invocation is trying to accomplish
right now, bounded to **one PR or one board-transition** (e.g., "get PR #3591 from
`in-review` to `merge-ready`"). Session goals are deliberately narrow because a goal
evaluator (whether a human reviewer or an automated check) only ever sees the
transcript of *this* session — it cannot verify intent that spans sessions. A session
goal that tries to span multiple PRs is unverifiable by construction: there is no
single transcript that proves the larger claim.

**Rule of thumb:** if you can't state the session goal as "PR #N: get label X applied"
or "issue #N: produce a builder-ready spec," it's actually a program-level goal
disguised as a session goal — push it up to `.perl-lsp/goals/` instead of trying to
carry it in one session's context.

---

## Truth hierarchy

When sources disagree, higher wins. This is the general form of
[LIVE_SIGNALS_VS_LABELS.md](../reference/LIVE_SIGNALS_VS_LABELS.md)'s CI-specific rule,
extended to every kind of state an agent might consult:

| Rank | Source | Why it outranks the rest |
|------|--------|---------------------------|
| 1 (highest) | Current `origin/main` + live GitHub PR/check state (`statusCheckRollup`, `mergeStateStatus`) | Directly queryable ground truth — nothing can be truer than what GitHub reports right now for the current HEAD SHA. |
| 2 | Active lane manifest (`.perl-lsp/goals/*.toml`) + generated status boards (`docs/project/status/*.md`) | Machine-maintained, regenerated on a known cadence — stale by at most one regeneration cycle, never hand-edited. |
| 3 | Machine receipts + accepted baselines (CI run receipts, `parser-corpus-baseline.json`, closure receipts) | Bound to a specific SHA at time of capture; authoritative for *that* SHA, may be stale for HEAD. |
| 4 | Specs / ADRs / contracts / policy (`.spec/`, `docs/adr/`, `docs/reference/PARSER_CONTRACTS.md`, `.ci/policies/*.toml`) | Human-authored intent — correct until superseded, but can drift from what's actually merged. |
| 5 | CLAUDE.md + scoped rules (nested `CLAUDE.md`, `.claude/agents/*.md` frontmatter) | Durable operating instructions — high-trust but reviewed on a slower cadence than code. |
| 6 | Auto-memory + `CLAUDE.local.md` | Locally accumulated, unreviewed, uncommitted — useful priors, not proof. |
| 7 (lowest) | Conversation handoffs (a prior agent's summary, a chat message claiming state) | Self-report is the least verified layer — see the *Closure discipline* principle in root CLAUDE.md: a passing component test (or a claim) never proves the system. |

**Practical implication:** never let a lower-ranked source override a higher one. If a
conversation handoff says "PR #3591 is merged" but live GitHub says it's still open,
GitHub wins — full stop, no averaging. If `CLAUDE.local.md` has a stale note about a
required check that `.ci/policies/required-checks.toml` no longer lists, the policy
file wins.

---

## Work discipline

- **One accountable writer per PR.** Multiple agents may contribute review/comments,
  but exactly one agent (or one human) owns the write path for a given PR's code at any
  moment. Concurrent uncoordinated writers is how [#682/#1432-class contamination](../../docs/learnings/README.md)
  happens.
- **Production writes happen in a worktree.** Never edit the main checkout directly
  (see the `agent-preflight` isolation check). This is non-negotiable infrastructure,
  not a style preference.
- **Finish or disposition same-lane active work before starting another branch.** Don't
  abandon a half-done PR to start a new one in the same lane — either land it, park it
  with an explicit note, or close it. Silent abandonment is invisible to the truth
  hierarchy (nothing at rank 1–3 records "I meant to come back to this").
- **One change, one proof, one PR.** A PR's diff should map to one coherent change with
  one behavioral proof (test, receipt, or both) — not a bundle of unrelated fixes that
  are individually hard to review and impossible to bisect.
- **Never weaken a test or ratchet for green.** A red gate is signal. Loosening the
  assertion, skipping the test, or lowering a ratchet threshold to make CI pass
  converts real signal into false green — see *the control plane is the binding
  constraint* in root CLAUDE.md and the corpus-ratchet discipline in
  [PIPELINE_GATES.md](../reference/PIPELINE_GATES.md).

---

## Delegation model

perl-lsp routes work across models and structures by the *shape* of the task, not by
default preference:

- **Haiku** — search, mechanical verification, external-fact-checking, narrow review.
  Cheap, fast, well-suited to bounded lookups: accuracy-scout (file:line facts),
  research-verifier (Perl/LSP/crate-API facts against an oracle), reviewer (banned
  patterns, formatting), green-ci (CI freshness). See the Gate 1–5 haiku roster in
  [PIPELINE_GATES.md](../reference/PIPELINE_GATES.md).
- **Sonnet** — plan, implement, synthesize, refactor, deep-review. Reserved for work
  that requires holding a nontrivial design or correctness argument in context:
  plan-reviewer, builder, green-refactor, reviewer-deep.
- **Workflows** — broad independent fan-out or repeatable audits, where many
  independent angles need to run without cross-contaminating each other's context
  (e.g., the six-angle spec-builder workflow referenced from
  [SPEC_TEMPLATE.md](../reference/SPEC_TEMPLATE.md)).
- **Teams** — reserved for cases where workers must actually communicate or challenge
  each other mid-task (not just hand off sequentially). Most of the pipeline is
  sequential handoff, not a team — see *Sequencing within a gate* in root CLAUDE.md.
  Reach for a team only when the task genuinely needs live cross-talk, since teams cost
  more coordination overhead than a relay.

**Independent review approaches the seam from a different direction**, per the
*Adversarial review is seam-anchored* principle in root CLAUDE.md: the value of a
second pass is the different angle (what feeds this? what consumes this? what happens
on `None`/`Err`/empty?), not merely a separate agent instance with a clean context.
Re-aim a warm agent across angles rather than spinning up a new one for its own sake.

**Model routing note:** leave `CLAUDE_CODE_SUBAGENT_MODEL` **unset**. Each agent
definition's own `model:` frontmatter (haiku/sonnet/opus) encodes the delegation
decision above; a global subagent-model override defeats that per-agent routing and
silently makes every haiku-shaped task cost sonnet tokens (or vice versa).

---

## Capability-read-only rule

Review and audit workflows (accuracy-scout, research-verifier, oppositional-planner,
diff-auditor, and any ad hoc audit fan-out) must be **capability read-only**, not
merely *prompted* read-only. A prompt instruction ("don't edit files") is advisory —
a distracted or adversarially-steered agent can ignore it. A tool allowlist is
enforced by the harness.

Concretely: when spawning a review/audit agent or workflow, the effective tool set must
**exclude** `Edit`, `Write`, `NotebookEdit`, mutating `git` commands (`commit`, `push`,
`reset --hard`, `merge`), and GitHub write operations (label/comment/merge via `gh` or
`mcp__github__*`). This matters especially for **workflow subagents**, which by default
run in `acceptEdits` permission mode and **inherit the parent's tool allowlist** — if
the parent has write tools available, a workflow subagent spawned without an explicit
capability restriction inherits them too, silently turning an intended read-only audit
into a write-capable one. Restrict the allowlist explicitly at spawn time; do not rely
on the prompt alone.

---

## Receipts + PR cockpit

**Receipts** are machine-produced, SHA-bound, claim-bounded evidence — not narrative
summaries. A receipt names: the repo, the base+head SHA it was produced against, what
was actually run (command, not description), and what it does *not* cover. See the
*Closure discipline* closure-receipt shape in root CLAUDE.md
(`repo, base+head SHA, production_entrypoint, call_chain_verified,
independent_expected_behavior, remote_head_confirmed, user_visible_effect,
fallback_remaining, uncertainty`) — that shape generalizes to any "this is done" claim,
not just security/correctness ones.

**PR cockpit** — every PR body should carry these sections so a reviewer (human or
agent) can evaluate the claim without re-deriving it from the diff:

| Section | Answers |
|---|---|
| Intent | What is this PR trying to accomplish? |
| Controlling issue | Which GitHub issue does this close/address? |
| Scope | What files/crates does this touch, and why exactly those? |
| Non-goals | What does this PR deliberately *not* do (prevents scope-creep review comments)? |
| Change shape | Structural description of the diff (new file, refactor, behavior change, docs-only, etc.) |
| Behavioral proof | The test/receipt that demonstrates the change works — not "should work." |
| Receipts | Links or inline output of the machine receipts above. |
| Independent review | Who/what reviewed this from a different direction, and what they found. |
| What was not run | Explicit gaps — e.g., "full `ci-gate` not run locally; relying on CI." |
| Claim boundary | What this PR does NOT claim to prove (mirrors *uncertainty* in the closure receipt). |
| Risk & rollback | Blast radius if wrong, and how to revert. |
| Remaining work | Follow-ups this PR intentionally defers. |

This is the target shape; existing `/pr-create` output should move toward it
incrementally rather than being reformatted wholesale in one pass.

---

## State discipline

**GitHub/repo state is truth. Conversational checkboxes are not.** An agent's own
running tally of "done: 4/7" in its transcript is not verifiable by anyone outside that
session — see rank 7 (lowest) in the truth hierarchy above. If a state claim matters
beyond the current session, it must land as a GitHub label, a comment, a commit, or a
generated status doc — something rank 1–3 can independently confirm.

**Known harness bug — TaskList `completed` status does not persist.** As of this
writing, marking a `TaskUpdate` item `completed` does not reliably survive across
compaction/session boundaries. Do **not** treat the TaskList board as authoritative
state for cross-session tracking; it is a within-session scratchpad only. Rely on
GitHub (labels, issue/PR state) for anything that needs to survive past the current
transcript.

**Never store live state in memory.** Auto-memory and `CLAUDE.local.md` are for durable
*preferences and learned patterns* (see
[docs/learnings/](../learnings/README.md)), not for the current PR number, current head
SHA, current CI status, or current blocker list — those change every few minutes and
belong at truth-hierarchy rank 1–3, queried fresh each time, never cached in a
long-lived memory file where they will silently go stale.

**Meta labels** (self-descriptive, no dedicated table needed): `size/S` / `size/M` /
`size/L` record effort estimate; `swarm-discovered` records provenance ("found by an
automated sweep, not a human report"). Per-label live-vs-authoritative detail for every
other label (Sign-off, State, Routing) lives in
[LIVE_SIGNALS_VS_LABELS.md](../reference/LIVE_SIGNALS_VS_LABELS.md).

---

## Worktree / agent operational gotchas

Field-learned failures specific to running agents in git worktrees. None of these
change product behavior — they are how to avoid wasting an agent's turn on
infrastructure friction.

- **`gh pr checkout` can fail with exit 128 inside a worktree.** Use
  `git fetch origin pull/<N>/head:<branch>` instead to check out a PR's branch in a
  worktree.
- **Run cargo builds/tests in the foreground with an explicit long timeout.** A
  workflow/background agent that ends its turn waiting on a background build loses the
  completion notification and its work along with it.
- **If a long build is killed by a tool timeout, re-run the same command.**
  Incremental compilation resumes where it stopped — the prior work is not lost.
- **Under concurrent worktree builds, `sccache` can fail with a bare exit-1 on
  unrelated crates.** Set `RUSTC_WRAPPER=""` for agent builds to avoid this.

---

## See also

- [docs/reference/PIPELINE_GATES.md](../reference/PIPELINE_GATES.md) — the 7-gate
  model this doc's delegation guidance feeds into.
- [docs/reference/MAINTAINER_AGENT_DOCTRINE.md](../reference/MAINTAINER_AGENT_DOCTRINE.md) —
  the operating contract for consequential PR decisions (work PR by PR, verify from
  primary artifacts, never destructively batch).
- [docs/reference/ORCHESTRATION_DOCTRINE.md](../reference/ORCHESTRATION_DOCTRINE.md) —
  design rationale for the orchestration model this doc assumes.
- [docs/reference/LIVE_SIGNALS_VS_LABELS.md](../reference/LIVE_SIGNALS_VS_LABELS.md) —
  the CI-specific instance of the truth-hierarchy principle above.
- [docs/forensics/2026-06-25-closure-gap-the-recurring-defect.md](../forensics/2026-06-25-closure-gap-the-recurring-defect.md) —
  the incident record behind the closure-discipline and receipt language reused here.
- [docs/swarm/operating-model.md](operating-model.md) — the repo-role contract
  (`perl-lsp-swarm` vs `perl-lsp`) this doc is a sibling to.
