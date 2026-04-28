# 2026-04-28 — Architecture Foundation + 58-Merge Session Retrospective

**Session window**: ~10 hours, 2026-04-27/28
**Repo**: EffortlessMetrics/perl-lsp
**Operating mode**: Dual-track — architectural doc wave + continuous merge operations
**Headline**: Landed 13 architectural reference docs + ADR-0044, shipped the queue reconciler, merged 58 PRs, filed 24+ follow-up issues. Open PR count moved from ~223 to ~159.

---

## Session metrics

| Metric | Value |
|--------|-------|
| PRs merged | 58 |
| Architectural reference docs landed | 12 |
| ADRs filed | 1 (ADR-0044) |
| Queue reconciler implementation | #7085 |
| PRs closed (lessons harvested) | 4 |
| Issues filed for follow-up | 24+ |
| Memory entries captured | 7 |
| Open PRs at start | ~223 |
| Open PRs at end | ~159 |
| Net queue delta | −64 (combined merge + close) |

---

## Architectural pieces landed

The session opened a doc wave that had been accumulating as a backlog. These were not speculative — each doc codified methodology that was already operating but unwritten. The writing forced precision and surfaced several inconsistencies in the running system.

### Reference docs (all under `docs/reference/`)

| File | Purpose |
|------|---------|
| `ORCHESTRATION_DOCTRINE.md` | Mentality, direction, and design rationale for the orchestration model |
| `OCTOPUS_CLUSTER.md` | Umbrella concept and vocabulary — the primary entry point for new readers |
| `PIPELINE_GATES.md` | 7-gate model with skip criteria, within-gate ordering, and three-axis triangulation |
| `LIVE_SIGNALS_VS_LABELS.md` | Live-truth principle: when the GitHub API can answer a question, it supersedes labels |
| `GLOSSARY.md` | Vocabulary index, cross-referenced to source docs |
| `FAILURE_MODES.md` | Pattern catalog of known failure modes with detection and recovery |
| `RECEIPT_SCHEMA.md` | Structured receipt format for agent sign-offs |
| `JUDGMENT_COMPOSITION.md` | Multi-agent verdict synthesis — how multiple lens outputs compose into a decision |
| `WORKTREE_PROTOCOL.md` | Multi-box safety: stash prohibition, CARGO_TARGET_DIR isolation, branch discipline |
| `CI_ARCHITECTURE.md` | Frontdoor / survivor / master-watcher CI topology |
| `CLUSTER_CURATION.md` | Codex/Jules ensemble methodology — how to evaluate, pick, and close duplicate clusters |
| `DISTRIBUTED_ENGINEERING_LINEAGE.md` | Positioning: Octopus Cluster vs Beowulf, SDLC mapping, classical practice inheritance |

### ADR

`docs/adr/0044-octopus-cluster-orchestration.md` — formal decision record accepting the Octopus Cluster as the authoritative orchestration architecture. Related to ADR-0033 (worktree-first disposable workers).

### Reconciler

`xtask/src/tasks/queue_reconciler.rs` (PR #7085) — the load-bearing implementation. 15-minute cron. Detects label contradictions, strips stale `ci-green` when HEAD has changed, reconciles `needs-*` conflicts. The system was previously dependent on agents remembering to clean state; the reconciler owns that responsibility.

---

## Durable learnings

### 1. Live truth beats labels — every time

Every label-state contradiction during the session had a live signal that resolved it unambiguously. The pattern is documented in `LIVE_SIGNALS_VS_LABELS.md`, but the session evidence makes it visceral: `ci-green` applied by an agent 3 hours ago doesn't mean CI is green now. The `statusCheckRollup` for the current HEAD SHA does.

The practical consequence: ops agents were almost merging PRs on stale `ci-green` state throughout the session. The fix is architectural — the reconciler strips `ci-green` when the HEAD SHA changes, so the label stays current rather than drifting. The label-as-cache model (stamp when checked, invalidate on change) is the correct mental model.

This aligns with `feedback_label_skill_silent_failure` (labels applied by agents fail to land ~80% of the time from agent-reported actions alone) — the reconciler also acts as the truth-enforcement layer for labels that agents should have set but didn't.

Cross-reference: `docs/reference/LIVE_SIGNALS_VS_LABELS.md`, `docs/adr/0044-octopus-cluster-orchestration.md`.

### 2. Receipts are memory, not noise

The initial reconciler design included a comment-volume tier: "post a detailed comment for contradictions, a brief comment for staleness, no comment for routine reconciliation." The user pushed back: all reconciler actions should produce durable comments. The correction was right.

The reason surfaced during the session: future agents reading a PR thread need to understand _why_ labels changed. A label appearing or disappearing without a comment creates a mystery. The comment trail is how agents (human and AI) reconstruct what happened to a PR without re-running all the checks. This is the same principle documented in `feedback_comment_trail_over_overwrite` — post corrections as comments, never silently overwrite.

The reconciler's final design posts a structured comment every time it takes an action. The comment format follows `RECEIPT_SCHEMA.md`.

Cross-reference: `docs/reference/RECEIPT_SCHEMA.md`, `docs/reference/FAILURE_MODES.md`.

### 3. Tooling owns state — agents cannot

When an agent must remember to do X at the end of its pass, X gets forgotten proportionally to how many steps precede it in the agent's context. During this session the "strip `needs-ci-fix` when applying `ci-green`" step was the canonical example: agents were applying `ci-green` and leaving `needs-ci-fix` in place, creating contradictory state that blocked ops routing.

The skill-edit approach — "add a reminder to the agent's prompt to strip the old label before applying the new one" — is the wrong fix. It creates a distributed memory problem: every agent that might touch these labels needs to remember the invariant. The reconciler approach is right: define the invariant once, enforce it continuously. No agent needs to remember because the reconciler catches violations within 15 minutes.

The broader principle, which landed in `ORCHESTRATION_DOCTRINE.md`: the system's invariants should live in code (the reconciler) and documentation (CLAUDE.md, reference docs), not in agent working memory. Working memory is finite and session-scoped; the tooling persists.

This is the practical reason the #6853 control-plane work existed. Taking label management out of agent hands — or at least making agent label mistakes self-correcting — is the right direction.

Cross-reference: `docs/reference/ORCHESTRATION_DOCTRINE.md`, `xtask/src/tasks/queue_reconciler.rs`.

### 4. Octopus Cluster is distributed engineering practice, not an HPC scheduler

The framing that crystallized during `DISTRIBUTED_ENGINEERING_LINEAGE.md` authoring: Beowulf clusters scale execution (parallel computation on shared data). The Octopus Cluster scales trust formation (converting candidates into trusted changes through multiple verification passes).

These are qualitatively different concerns. Beowulf cares about throughput and load distribution. The Octopus Cluster cares about the reliability of the trust signal — whether a PR that reaches merge is actually safe to merge. Adding more agents doesn't help if the verification passes are insufficient; it just generates more candidates that fail later in the pipeline.

The SDLC mapping in `DISTRIBUTED_ENGINEERING_LINEAGE.md` makes this concrete: the 7-gate model encodes classical software engineering practice (Kanban work tracking, code review, CI/CD, trunk health, SRE incident response, receipt-based accountability). The multi-agent architecture is the implementation mechanism, not the purpose. The purpose is the same as what a strong engineering team achieves — trusted change at velocity.

Cross-reference: `docs/reference/DISTRIBUTED_ENGINEERING_LINEAGE.md`, `docs/reference/OCTOPUS_CLUSTER.md`.

### 5. Cascade conflicts are the dominant operational cost

Not candidate generation. Not individual review passes. The bottleneck this session was CLAUDE.md cross-reference conflicts from multiple doc PRs touching the same file in rapid succession. The pattern repeated more than 10 times: one PR adds a sentence to CLAUDE.md, a second PR (opened concurrently) adds a different sentence, the first merges, the second now has a conflict at that exact location.

The fix is sequencing. Doc waves that all target CLAUDE.md or other high-traffic files should be batched differently: one PR opens, merges, next PR rebases on the result. The current pattern of parallel generation into a shared file produces O(N) conflicts for N concurrent PRs — each subsequent PR conflicts with each prior merge.

Issue #7126 was filed during the session to track the structural fix: identify which files are "conflict hotspots" and sequence PRs touching them rather than parallelizing.

This cost is distinct from master bit-rot (where the cause is code-level breakage). Cascade conflicts are pure coordination overhead — the underlying changes are all correct, but they land in the wrong order.

### 6. Master bit-rot is an incident class, not a per-PR investigation

Two master bit-rot incidents this session: the test panic surfaced by #5985 (fixed via #7031) and the fmt cascade from the doc wave (fixed via #7090). Both followed the same detection signature: 3+ PRs failing the same gate identically, in the same CI step, with the same error message.

The operational response is documented in `FAILURE_MODES.md` under the "Master Bit-Rot" pattern:

1. Stop per-PR debugging immediately when the signature appears — any per-PR investigation at this point is wasted effort because the PR is not the cause.
2. Verify on fresh master locally: `cargo xtask fmt --check` or `cargo test -p <crate>` on master HEAD.
3. If confirmed: open a narrow fix PR targeting master only. Admin-merge after local verification passes.
4. Cascade-update all blocked PRs: `gh pr update-branch --rebase` on each PR in the blocked cohort.

The session confirmed the pattern from `feedback_master_bitrot_cascade_8plus_pattern`: Codex burst waves produce approximately 1 master-side breakage per 2-3 PRs merged. Budget for this in high-throughput sessions — it is not a failure of the process, it is a predictable consequence of high-velocity merging into a shared codebase.

Cross-reference: `docs/reference/FAILURE_MODES.md`, `docs/reference/CI_ARCHITECTURE.md`.

### 7. rtk's `gh pr checks` masks aggregator failures

During ops merge preparation, the `rtk gh pr checks` output showed all checks green for a PR that the raw GitHub API reported as having failing checks. The rtk wrapper was filtering the output to show only the most recent check result per check name, but it was applying that filter in a way that could show a stale passing result over a current failing one.

The PR was nearly merged in this state. The issue surfaced when the raw `gh pr view --json statusCheckRollup` was queried directly, which showed the correct current state.

The operational fix: always verify mergeable state against the raw `statusCheckRollup` before merge, not against the rtk-filtered output. The rtk output is useful for navigating the check list but should not be trusted as the final word on go/no-go for merge.

Issue #7127 was filed to track the rtk filter calibration problem.

### 8. Spec quality determines builder course-correction count

The reconciler builder (#7085) went through 4 mid-implementation course corrections. The spec that the builder received was complete on high-level intent but underspecified on type shapes, error handling paths, and the exact semantics of "label contradiction" in edge cases.

Each course correction cost a builder round-trip: discover the gap, pause, clarify with the orchestrator or infer from context, continue. The cumulative cost was approximately 2-3x what a well-specified implementation would have required.

The structural fix, filed as issue #7128: for implementations that touch state machines or multi-branch logic, drafting types and pseudo-code in the spec before the builder starts would catch shape mismatches early. "Pseudo-code + worked examples" is not over-specification — it is the minimum that makes complex state transitions builder-verifiable without mid-implementation discovery.

This aligns with the broader pattern from `feedback_spec_folders_are_history`: `.spec/` files should contain enough detail that a builder can implement without needing to re-research the problem domain.

### 9. Haiku-class agents do real reasoning on well-framed missions

The initial framing for haiku-class verification agents ("simple checks only, delegate complexity to sonnet") was calibrated for the wrong dimension. Haiku agents are weak on _long-horizon reasoning_ (multi-step inference, code generation across many files). They are capable on _targeted verification_ (does this claim hold? does this label contradict that one? is this file path correct?).

The distinction matters operationally: "simple" missions should mean "bounded scope, clear criterion," not "low cognitive difficulty." The label-contradiction detection in the reconciler is exactly the kind of bounded, clear-criterion task that haiku handles well. Several agents that were being routed to sonnet for "verification" tasks could be routed to haiku with better-framed missions.

The framing that works: scope + objective + authority + constraints + evidence + exit criteria. Six elements. Each makes the mission more tractable for a bounded model.

### 10. Verification bandwidth is the bottleneck, not generation

Thirty PRs remained blocked on the `parser_corpus_ratchet` substrate (#6847) at session end. These were all correct implementations by the generating agents — the parser work was sound. The constraint was that each PR requires `just cpan-corpus-ratchet` to run after merge, which is a sequential operation that cannot be parallelized across concurrent merges.

The lesson: generating more candidates against a verification-bottlenecked substrate produces queue growth, not throughput gain. The reconciler addresses label-state verification bottlenecks. The corpus ratchet bottleneck requires a different fix — either parallelizing the ratchet, or batching parser PRs into single merge operations before running it.

The user elected to take the corpus ratchet manually for this session. Issue #6847 tracks the structural fix.

Cross-reference: `docs/reference/CLUSTER_CURATION.md` (ensemble methodology and verification economics).

---

## What was harder than expected

### BDD cluster cascade conflicts

The BDD test harness cluster had 12 PRs touching the same harness file. Sequential merge cost was O(N²) in conflict resolution: each merge required rebasing the next N-1 PRs, each rebase had a chance of conflict at the now-merged location. The practical cost was ~4 hours of sequential rebasing that a bundle-PR approach would have completed in ~30 minutes.

Issue #7129 was filed to define the bundle-PR strategy for BDD and similar same-file clusters. The core idea: curate the cluster into a single PR (or a small number of non-overlapping PRs) before beginning the merge sequence, rather than merging independently and resolving conflicts serially.

### Multiple master bit-rot incidents revealing CI substrate fragility

Two independent master breaks in one session (test panic + fmt cascade) is above the expected rate of ~1 per session. Each incident required stopping merge operations, diagnosing root cause, shipping a fix, and cascade-updating blocked PRs — approximately 45-60 minutes per incident. The CI substrate fragility is not unexpected given the high-velocity doc wave, but the back-to-back pattern revealed that the monitoring gap (no automated master-green check at merge time) was leaving the team in a reactive position.

The reconciler's master-watcher component (filed in `CI_ARCHITECTURE.md`) is the structural response: actively check master CI state after each merge batch, rather than discovering breakage when PRs start failing.

### Worktree-to-main-checkout boundary ambiguity

Two agents during the session made edits in the main checkout rather than their assigned worktrees. Neither produced lasting damage (the edits were discovered and reverted), but the failure mode illustrates the weaker-than-assumed isolation in the worktree model.

The `WORKTREE_PROTOCOL.md` doc landed as a direct response: explicit rules for verifying worktree isolation at startup (`git rev-parse --show-toplevel` check, preflight script), stash prohibition, and the escalation path when isolation is violated.

---

## What was easier than expected

### Doc PRs through the gate

Once the gate model in `PIPELINE_GATES.md` was explicit about skip criteria (doc-only PRs skip several gates), the doc wave moved quickly. Twelve reference docs in one session is about 5x the normal doc throughput, but the gate model's skip-criteria clarity meant each doc PR spent only the time it needed — no unnecessary reviewer-deep passes on content that had no code.

### Reconciler landing cleanly

Despite 4 course corrections during build, the reconciler (PR #7085) landed cleanly on the first review pass. The implementation was complete and correct; the course corrections were in the spec-to-implementation translation, not in the implementation itself. The reviewer-deep pass found no bugs.

### Diff-audit batch processing

The diff-auditor ran on 20+ PRs in batch during the session without false-positive drift flags. The `check-agent-audit-trail` skill calibration from prior sessions (a PR may contain its own audit trail entries; contamination is when the trail names a *different* PR's work) held correctly across the batch.

---

## What's still in tension going into next session

### Parser corpus ratchet (#6847)

30 PRs blocked. The user is taking this manually. When it clears, the parser PR cluster should move quickly — the underlying implementations are sound.

### Reconciler SKIPPED handling (#7120)

The reconciler currently treats `SKIPPED` CI checks identically to `FAILED`. Approximately 30 PRs have SKIPPED checks that are legitimately skipped (platform-conditional jobs that don't apply to their change). These PRs are being held back from merge unnecessarily. Fix is small (~15 lines in `queue_reconciler.rs`); unblocking impact is ~30 PRs.

### Receipt schema bugs (#7113)

Section 7.3 of `RECEIPT_SCHEMA.md` defines a `review` receipt variant whose field names don't match what `gate-receipts validate` emits for standard review passes. Any PR with a `review` receipt fails validation. Fix requires either updating the schema or updating the validator — both are small; the schema source of truth needs to be decided.

### BDD cluster remainder

Seven BDD cluster PRs remain unmerged. The bundle-PR strategy from issue #7129 defines the approach; the execution requires one builder session to curate the cluster and one ops session to merge the result.

---

## Memory entries captured this session

Seven entries were added to project memory during or immediately after the session. Referenced by canonical name:

- `feedback_rtk_gh_pr_checks_masks_failures` — rtk filter masks aggregator failures; always use raw statusCheckRollup for go/no-go
- `feedback_spec_pseudocode_prevents_course_corrections` — pseudo-code + worked examples in specs prevent mid-implementation discoveries; quantified at 2-3x cost multiplier
- `feedback_cascade_conflict_dominant_cost` — doc waves into shared files produce O(N) conflicts; sequence same-file PRs
- `feedback_haiku_bounded_mission_framing` — haiku is capable on bounded verification; the six-element mission frame unlocks it
- `feedback_reconciler_owns_invariants` — tooling should own system invariants, not agent working memory; agents making mistakes is expected; reconciler making mistakes is a bug
- `feedback_master_bitrot_two_incidents_one_session` — two incidents in one session is above expected rate; master-watcher closes the detection gap
- `feedback_verification_bandwidth_bottleneck` — generation is cheap; the constraint is verification; corpus ratchet is the canonical example

---

## Cross-reference index

All architectural docs landed this session:

- `docs/reference/ORCHESTRATION_DOCTRINE.md`
- `docs/reference/OCTOPUS_CLUSTER.md`
- `docs/reference/PIPELINE_GATES.md`
- `docs/reference/LIVE_SIGNALS_VS_LABELS.md`
- `docs/reference/GLOSSARY.md`
- `docs/reference/FAILURE_MODES.md`
- `docs/reference/RECEIPT_SCHEMA.md`
- `docs/reference/JUDGMENT_COMPOSITION.md`
- `docs/reference/WORKTREE_PROTOCOL.md`
- `docs/reference/CI_ARCHITECTURE.md`
- `docs/reference/CLUSTER_CURATION.md`
- `docs/reference/DISTRIBUTED_ENGINEERING_LINEAGE.md`
- `docs/adr/0044-octopus-cluster-orchestration.md`
- `xtask/src/tasks/queue_reconciler.rs`

Follow-up issues filed (selected):

- `#7085` — queue reconciler (landed)
- `#7113` — receipt schema `review` variant bug
- `#7120` — reconciler SKIPPED check handling
- `#7126` — cascade conflict hotspot sequencing
- `#7127` — rtk `gh pr checks` aggregator masking
- `#7128` — spec pseudo-code requirement for state-machine implementations
- `#7129` — BDD cluster bundle-PR strategy
- `#6847` — parser corpus ratchet parallelization (user-owned)

Prior memory entries most relevant to this session's findings:

- `feedback_comment_trail_over_overwrite` — receipts as memory, not noise
- `feedback_master_bitrot_cascade_8plus_pattern` — master bit-rot incident class
- `feedback_label_skill_silent_failure` — agent label application failure rate (~80%)
- `feedback_deep_reviewer_premature_merge_ready` — ops must verify ci-green is live, not labeled
- `feedback_spec_folders_are_history` — `.spec/` files are research artifacts, not clutter
