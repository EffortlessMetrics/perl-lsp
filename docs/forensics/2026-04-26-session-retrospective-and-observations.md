# Session Retrospective + Agentic Development & Repo Observations

**Window**: 2026-04-26 session, ~02:00–06:00 EDT (~4 hours active orchestration)
**Audience**: orchestrator (next session), maintainers, anyone studying agentic development as methodology
**Purpose**: capture quantitative session outcomes, qualitative observations on agentic development, and repo-state observations distinctive to this session.

---

## Session-end metrics

### Merged this session: 17 PRs

**Master fixes (4)** — narrow fmt + clippy fixes that unblocked ~30 PRs each:
- #6789 incremental_checkpoint.rs fmt
- #6803 perl-pragma comprehensive_unit_tests.rs fmt
- #6807 semantic-analyzer fuzz tests fmt
- #6810 perl-parser doc_lazy_continuation clippy

**Methodology (3)** — codified directives in the methodology layer:
- #6808 agent definitions + CLAUDE.md (master-green directive, sign-off-as-routing, no-needs-on-merge, external-PRs-same-gates)
- #6811 4 key skill playbooks (ops-merge-batch, ops-check-queue, reviewer-decide, diff-audit-comment)
- #6812 vscode-extension #6780 cleanup (lazy debug activation + stale onCommand:* test purge)

**Earlier ops-batch merges (10)** — drained queue of clean PRs:
- #6351, #6333 (perl-pragma test/refactor)
- #5422, #5415, #5410, #5403, #5399, #5369, #5359 (mixed: docs, AI fixes, semantic, xtask, perl-lsp-rs)
- #5728 (semantic goto-definition)

### Cherry-pick recoveries: 6 fresh-root strands → replacement PRs
- #5513 → #6804, #5509 → #6805, #5502 → #6806 (early session)
- #6051 → #6837, #6138 → #6838, #6129 → #6839 (late session)

### New issues filed: 5
- #6791-#6794 tooling-debt (xtask fmt error message, sandbox timeout, UX Regression Gate trigger gap, CARGO_BUILD_JOBS=1 phantom timeouts) — all from prior session, dispatched in this session
- #6802 perl-ci-hygiene PR Smoke `--lib` mismatch (binary-only crate gap)

### Queue snapshot at wind-down
- 306 open PRs (down from session-start of ~317)
- 17 merge-ready (UNKNOWN/UNSTABLE pending master CI cycle on 639a311dc)
- 50 needs-ci-fix (most likely stale-base; cascade-update will clear majority)
- 21 needs-builder-fix
- 1 needs-diff-fix (was 8 at session start — strong drain)
- 22+ fresh-root stranded PRs flagged `structural-blocker` (large pending cherry-pick recovery batch)

---

## What was unusual this session

### 1. Methodology debt got paid down

For most sessions, the methodology layer is a quiet substrate that everyone uses but nobody touches. This session shipped THREE explicit methodology PRs (#6808, #6811, plus the related #6812 cleanup). The trigger was a single failed gate (the #6780 incident) revealing a structural gap.

The pattern: real incidents reveal latent methodology weaknesses. The user's reframing ("sign-off is one of the routing decisions") didn't exist as a written rule before this session — it was elicited by the incident. The methodology PRs codified the elicited rule so the gap doesn't recur.

The lesson: methodology debt accumulates silently until an incident exposes it. Sessions that pay it down have outsized leverage on future sessions.

### 2. The false-signal-to-real-signal ratio inverted from prior assumption

Prior calibration treated "3+ PRs failing identically" as a strong master-bit-rot signal. This session's data: 3 false positives for every 1 real positive. The aggregator-level view (statusCheckRollup, label queries) makes false signals look identical to real ones.

Detection has to verify on fresh master locally before triggering a master-fix builder. The cost is one local `cargo xtask fmt --check` (~10 sec); the savings is hours of premium-agent waste on speculative fixes.

### 3. Fresh-root strand pool is much larger than estimated

Earlier session estimated 4-5 stranded PRs. This session's stuck-PR scout enumerated 22 explicit fresh-root strands flagged `structural-blocker`. Plus inference from the 11 "operator-decision" leftovers suggests another ~15-30 likely candidates in the broader queue.

Cherry-pick recovery is the only fix (per `feedback_fresh_root_master_rebuild.md`) — and it's substantive work per PR (~15 min each). At ~22 explicit strands × 15 min = ~5.5 hours of cherry-pick work pending. That's a meaningful queue-drain cost that the next session should plan for explicitly.

### 4. Agent-reported labels vs. orchestrator-direct labels held the ~80% silent-fail-vs-100% gap

Multiple sweeps this session explicitly verified label landing post-application. Pattern continues to hold:
- Orchestrator-direct `gh pr edit --add-label X` lands 100% of the time
- Agent-reported "I added label X" lands ~20% of the time (per prior calibration)

The label-drift fix sweep that ran earlier in the session unblocked 30 PRs by retroactively applying labels that agents had reported but didn't actually land. This is a recurring methodology cost; structural fix would be the agents' label-apply step verifying via `gh pr view --json labels` and retrying on miss.

### 5. The methodology PR review pattern surfaced

Twice during the session, my methodology PRs (#6808, #6811) were locally reverted in the user's working tree (visible via system reminders saying "this change was intentional, don't revert it"). Both times the PR was actually merged on remote — the local revert was IDE/review state, not a merge rejection.

The pattern: the user reads my methodology edits in the IDE, may try local reverts to see what reverts to, then merges if satisfied. This produces apparent contradictions in real-time (system says "reverted" while remote shows merged) that I had to learn to interpret correctly.

The lesson: don't trust local-state system reminders as authoritative for remote PR state. Always verify on remote via `git fetch + git show origin/master:<file>` or `gh pr view`.

---

## Agentic development observations

### Defense in depth works as designed

The verification ladder caught real bugs at multiple stages this session:
- **Standards review** caught wrong-language `idris` reference + missing manifest change in #6780 (though the agent then bypassed itself by applying contradictory labels — the methodology gap, not the gate's catch capability)
- **Deep review** caught logic bug in #5881 (claimed feature not delivered) and `f64::EPSILON` floating-point tolerance issue in #5416
- **Diff audit** caught cross-PR contamination in #5870 (2043 of 2063 lines) and BOM corruption in #5416
- **Refactor planner** caught duplication in #6347 (3 byte-identical 4-line blocks) and self-refactor opportunities the builders had already addressed
- **Maintainer-PR review** caught scope drift in #6786 (auto-regen status docs hand-edited)

Hit rate across sweeps was high enough to validate the investment. Sonnet deep-review specifically continues to catch correctness bugs at near-100% rate per `feedback_deep_review_bug_catch_roi.md`.

### The "every PR potentially problematic" disposition produces concrete observations

Standards review wave 2 produced 12/15 clean verdicts — but each clean verdict had a concrete substantive observation, not just "looks fine." 3 of 15 had real mechanical fixes (duplicate justfile recipes, removed `unwrap()` in test helper). 0 needs-builder-fix this batch.

The pattern: when reviewers default to "find something concrete," they catch things they would otherwise have approved. The 3 mechanical fixes were obvious in hindsight but only because the reviewer was actively looking. Without the disposition, those would have ridden into master.

### Skill-layer enforcement matters

PR #6808 codified the rules in agent definitions; PR #6811 carried them into runtime skill playbooks. Without the skill updates, the rules would sit in agent prompts but the procedural steps the agents follow at decision points wouldn't enforce them.

The two layers are complementary:
- Agent definitions = "what the agent does and why"
- Skill playbooks = "how the agent does it step by step"

Methodology debt accumulates in either layer if only one is updated. Future methodology PRs should bundle both unless there's a specific reason not to.

### The orchestrator's role as recursive supervisor

The session pattern was: dispatch agents → route returns → dispatch more. Most agents ran in parallel (peak ~13 in flight at one point). The orchestrator's role was:
- Pick what to dispatch based on queue gaps
- Read agent reports and decide next routing
- Catch methodology bugs in real-time when user interrupts
- Apply small mechanical actions directly (label fixes, comments, branch ops) rather than dispatching for trivial work

The methodology bug-catching is the load-bearing part. The user's interruptions ("Like, signoff is signoff. It's signoff *or* bouncing back, not both") prevented bad patterns from getting baked in. Without that, the methodology drifts toward whatever the agents happened to do.

### The compounding nature of methodology investment

The methodology PRs (#6808, #6811) compound BACKWARD — every prior session's accumulated debt now has rule-based prevention. The 19-PR label-reconciliation sweep this session was paying down debt that wouldn't have accumulated under the new rules. Future sessions won't need that sweep because the rules prevent the contradictory state from forming.

Similarly, the cross-PR source-file contamination check added to diff-auditor (PR #6808) prevents future #5870-class incidents (2043 lines of orphan source files riding into master). Each methodology investment closes a class of incidents, not just a single instance.

---

## Repo observations

### Master is "lightly red" most of the time

Throughout the session, master CI was repeatedly cancelled by rapid-merge sequences (visible in `gh run list --workflow=CI --branch=master`) or pending after a merge. The "true green" state is brief and elusive in high-throughput sessions.

This puts pressure on the master-green directive: requiring strict green-master-before-merge would slow the merge cadence considerably. The current workable compromise: workspace-wide CI gates on each PR (per #6808) catch most master breaks at PR time, so master-after-merge is usually still green even without explicit master-CI-wait between merges.

The CI cancellation cascade is per-design (CLAUDE.md: "batches of 3, wait for green between batches" — and the cancellations enforce this when the protocol slips).

### Microcrate collapse is mid-flight (still)

Multiple stranded PRs touched crates that have been absorbed into other crates (e.g., #6051 was about `perl-lsp-config` which was absorbed into `perl-lsp-rs-core::config` during microcrate collapse). The cherry-pick recovery had to manually port the fix and regression test to the new home.

The collapse is enabling architectural simplification but creating a recurring cost for any PR that branched before the collapse. Per `project_microcrate_collapse_v014.md`, this is expected — the v0.13.0 target is 30 published crates from 135. Pre-collapse PRs need cherry-pick + manual porting until the collapse is complete.

### v0.13.0 release-prep work is the dominant theme

Of 17 merges this session:
- 4 master fixes (release-prep enabler)
- 3 methodology infrastructure (release-prep methodology)
- 6 docs / forensics / agent definitions (release-prep methodology)
- 4 fixes / tests (perl-pragma, semantic, AI, perl-lsp-rs)

No new feature work merged this session. The pattern matches the broader v0.13.0 prep direction (per `2026-04-25-repo-direction-and-progress-signals.md`): polish, security, test infrastructure, methodology — not feature breadth.

### Tooling-debt is a real recurring tax

The 4 tooling-debt issues filed (#6791-#6794) and the 5th (#6802 perl-ci-hygiene PR Smoke gap) all describe friction patterns that recur every session. Cumulatively, they probably cost ~30-60 minutes of operator time per session. Fixing them would compound across future sessions.

The methodology layer (PR #6808 + #6811) prevents some classes (master-green directive prevents cascading master breaks); the underlying tooling fixes would prevent others (xtask fmt error message gives specific files; PR Smoke handles binary-only crates correctly). Both layers needed.

---

## What the next session should pick up

Priority-ordered:

1. **Verify master CI on 639a311dc** + cascade-update + ops drain for the 17 merge-ready PRs. Most should clear and merge.

2. **22 fresh-root strands** flagged `structural-blocker` — large cherry-pick recovery batch. Estimated ~5-6 hours total (~15 min per PR).

3. **New master fmt drift** flagged by pr-respond agent in 3 hot files (perl-pragma comprehensive_unit_tests, incremental_checkpoint, semantic-analyzer fuzz tests). Probably 1-2 narrow fixes, then cascade-update.

4. **#5881 logic bug** — claimed feature (lexical scope-distance ranking) doesn't actually deliver. Real builder work needed.

5. **Tooling-debt issues** (#6791-#6794, #6802) — each is bounded scope (~30 min to a few hours). Cumulative payoff is meaningful per-session waste reduction.

6. **Real-workspace baseline** (#6796) + **AI completion E2E** (#6797) — Tier-2 v0.13.0 release-readiness work that the methodology can't substitute for.

7. **Memory recalibration pass** — the recalibrator agent built on 2026-04-25 should run after this session to fold in fresh data (false-signal ratio update, sign-off-as-routing rule, master fix cadence calibration).

---

## Related forensics + memory entries

- `2026-04-26-sign-off-is-routing-methodology-strengthening.md` — companion doc on the methodology fix
- `2026-04-26-master-cascade-quartet-and-false-signal-calibration.md` — the four master fixes + calibration update
- `2026-04-25-repo-direction-and-progress-signals.md` — broader project direction context
- `2026-04-25-3day-arc-economics-and-learnings.md` — comparison baseline for session metrics
- `feedback_master_bit_rot_recurrence_pattern.md` — pattern this session updates with calibration
- `feedback_fresh_root_master_rebuild.md` — playbook for the 22 stranded PRs flagged this session
- PRs merged this session (17): see commit log between 2026-04-26 02:00 and 06:00 UTC

---

## Applies to

Reference this doc when:
- Planning the next session's priorities (top 7 list above)
- Comparing against future session retrospectives for trend analysis
- Onboarding a new operator who needs to understand current methodology state + queue dynamics
- Studying agentic development as methodology (the observations section is the meta-content)
- Calibrating expectations: 17 merges + 4 master fixes + 3 methodology PRs + 6 cherry-pick recoveries in ~4 hours is the demonstrated cadence at current substrate
