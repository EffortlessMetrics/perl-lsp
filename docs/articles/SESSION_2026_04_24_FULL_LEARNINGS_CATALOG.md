# Full-Session Learnings Catalog: 2026-04-24

**Session window:** 2026-04-24, spanning five phases across one Claude 5h window + memory
compaction midpoint
**Companion economics docs:** #6106 (phase 1), #6148 (phase 1 extension), #6161 (phase 2
boundary), #6445 (phase 2), #6449 (phase 3)

---

## 1. Framing

This session created seven new memory entries documenting load-bearing patterns discovered
under the pressure of the highest-throughput day in the project's history. Each pattern is
reproducible, each has a detection heuristic, and each is actionable by the orchestrator on
the next dispatch.

Rough session numbers drawn from `gh pr list --json` queries at session end:

| Metric | Approximate count |
|---|---|
| PRs merged on 2026-04-24 | ~120 |
| PRs closed (not merged) | ~280 |
| PRs analyzed / triaged | 500+ |
| Master commits pushed | ~60 |
| New memory files created | 7 |

The economics (cost ratios, quota burn, throughput efficiency) are documented separately in
the phase companion docs. This document focuses exclusively on the learnings dimension: what
changed in the team's operational understanding of how the pipeline works.

---

## 2. The Full-Stack Economic Architecture

Three memory entries created today jointly describe the cost architecture that makes
high-throughput sessions sustainable. They are complementary — each explains one tier of
a cascade that the others depend on.

**The cascade:**

```
cheap upstream    →    cheap generation    →    premium verification    →    cheap retrospective
(web threads)          (Codex bursts)           (sonnet gates)               (session docs)
      ↑                                                                              |
      └──────────────────────────────────────────────────────────────────────────────┘
                                  next-cycle input
```

### 2.1 Multi-gate catches cheap-model drift

Memory file: `memory/feedback_multigate_catches_cheap_model_drift.md`

The oppositional + research + diff-auditor + deep-reviewer gate stack reliably catches
misaligned PRs from cheaper upstream agents. Confirmed at 435-PR scale on this session.
Gate decomposition:

| Gate | What it catches |
|---|---|
| Research | Hallucinated features ("Codex Desktop supports LSP" — it doesn't) |
| Oppositional | Wrong layer, wrong framing |
| Diaboli | Shouldn't have been built at all |
| Architecture | Structural drift from microcrate layering |
| Maintainer-pr | Vision / scope drift |
| Diff-auditor | Branch contamination (PR says X, diff has X+Y+Z) |
| Deep-review | Correctness bugs, vacuous tests, coordinate-space errors |

Key operating principle: do not short-circuit gates under throughput pressure. Each gate
catches a different error class. Eliminating one does not speed up the pipeline — it shifts
the catch from "at gate" to "reported by user."

**Ensemble outcome distribution (new this session):** A 4-shot Codex burst can produce
0 keepers (wrong premise), 1 keeper (traditional sibling triage), or N keepers (different
PRs cover different angles and both are worth merging). The N-keepers case is now the
expected outcome in well-planned bursts. Curators should ask "how many distinct angles does
this cluster cover?" not "which one wins?"

### 2.2 Upstream research reduces downstream verification burden

Memory file: `memory/feedback_upstream_research_improves_pr_quality.md`

The quality of the Codex prompt controls how much verification work is needed downstream.
A prompt that names the exact file, line number, and MetaCPAN/LSP-spec citation produces
PRs that converge on the real fix. A prompt that says "fix the lifecycle bug" produces
5 PRs each guessing at what "the bug" is.

Observed ratio improvement: close-on-wrong-premise rate dropped ~3-5x when pre-planning
was added. Budget suggestion:

| Activity | Session time fraction |
|---|---|
| Upstream research + prompt quality | ~20% |
| Dispatch + verification | ~60% |
| Retrospective learning capture | ~20% |

Starving research to maximize dispatch creates the high-verification-burden trap — the
downstream gates spend their budget fixing problems that better prompts would have avoided.

### 2.3 Prompt generation is itself a cheap commodity step

Memory file: `memory/feedback_prompt_generation_is_cheap_web_thread.md`

The "ChatGPT-Pro pre-planning" step is 2 cheap low-attention web threads reading Claude
session history (forensic retrospectives, memory files, gap analyses, maintainer direction)
and synthesizing repo-aligned Codex prompts. This is commodity cost, not premium-model cost.

The key insight: premium budget stays downstream where judgment errors are load-bearing
(deep review, plan review, architecture check). Everything that can run on commodity does.
The cascade only works if the premium tier is not diluted by work that cheaper models can
handle.

**Net:** cheap upstream + cheap generation + premium verification + cheap retrospective
= sustainable economics. The `0/1/N keeper` distribution from section 2.1 is what makes the
generation tier cheap in practice — generating 4 PRs at Codex cost to get 1-2 good ones is
still cheaper than generating 1 PR at sonnet cost.

---

## 3. Operational Landmines

Three patterns that blocked work in ways that were not immediately obvious. Each has a
detection heuristic and a fast fix.

### 3.1 Local `origin/master` branch blocks all worktree spawns

Memory file: `memory/feedback_ambiguous_origin_master_branch.md`

**What happened:** All 10 agents in one wave failed to spawn with
`fatal: ambiguous object name: 'origin/master'`. A local branch literally named
`origin/master` existed alongside `refs/remotes/origin/master`. Git could not resolve which
one `git worktree add` should use.

**Detection:**
```bash
git branch -a | grep master
# If you see both "origin/master" (without "remotes/") and "remotes/origin/master",
# you have the shadow branch
```

Or add to the preflight script:
```bash
if git show-ref --verify --quiet refs/heads/origin/master; then
    echo "ERROR: local branch 'origin/master' exists — worktree spawns will fail"
fi
```

**Fix (safe):**
```bash
git branch -D origin/master
```
The local branch is a phantom that points to a commit already on real master. No work is lost.

**How it gets created:** `git checkout origin/master` on an older git version, or any script
that passes a slash-containing ref to `git branch`. Git happily creates local branches with
slashes, silently shadowing the remote-tracking refs.

**Why it matters for swarm throughput:** 10 simultaneous failures look like infrastructure
collapse. First response should be shadow-branch check, not CI investigation.

### 3.2 Label-set skill silently fails ~80% of the time

Memory file: `memory/feedback_label_skill_silent_failure.md`

**What happened:** 10+ agents returned reporting "`maintainer-pr-reviewed` set" on PRs
#6219-6227. Direct inspection showed zero of those labels had landed. Only `deep-reviewed`
(and sometimes `review-reviewed`) were present.

Agents post their verdict comments correctly. Labels don't land. The state machine never
advances. Ops sweeps find 0 merge-ready candidates despite verified work.

**Detection:** When multiple agents report "label X set" but ops still reports 0 ready
candidates, run a batch label check:
```bash
for pr in <list>; do
    gh pr view $pr --json labels -q '.labels[].name' | sort | tr '\n' ','
    echo " (PR $pr)"
done
```

**Fix from the orchestrator:**
```bash
for pr in 6219 6220 6221; do
    gh pr edit $pr --add-label "review-reviewed,maintainer-pr-reviewed,diff-audited"
done
```

This is cheap, authoritative, and requires no worktree.

**Cost if missed:** Each affected PR has to go through another agent pass, and the ops sweep
finds nothing actionable until the labels are corrected. One wave of 9 PRs with this failure
lost ~1 full wave's verification work before detection.

### 3.3 Master bit-rot recurs in bursts

Memory file: `memory/feedback_master_bit_rot_recurrence_pattern.md`

**What happened:** Four distinct master bit-rot instances in one day, each blocking 8-30
PRs:

| PR | Root cause | PRs blocked |
|---|---|---|
| #6163 | `perl-workspace-index` package rename; `xtask/ci_scope.rs` still referenced old name | ~8 |
| #6286 | `cargo xtask fmt` abort-on-first-failure; fmt + format-string escape drift in parser tests | ~30 |
| #5986 | `std::fs::canonicalize` expands Windows `RUNNER~1` short-names; path comparisons failed | ~18 |
| #6446 | `normalize_include_path` emits `\` on Windows; tests hardcoded `/` | ~12 |

**Detection heuristic:** If 3+ PRs have the same individual CI gate failing with similar
error signatures, investigate master before dispatching pr-responders. The pr-responders
will waste cycles trying to fix something that is not their problem.

**Protocol:**
1. Dispatch a "report only, don't fix" investigator to identify the root cause narrowly.
2. Dispatch a builder with tight scope ("change exactly these N lines, no other files").
3. Admin-merge the fix (master fix; aggregator flake is expected — do not wait for full green).
4. Cascade `gh pr update-branch` across the top 15-20 affected PRs.

**Economics:** Detection + fix takes 10 minutes. Miss-detection sends 20 PRs to pr-responder
at 5-10 minutes each = 2-3 hours of wasted premium-agent time.

---

## 4. Cache TTL and Session Pacing

Memory file: `memory/feedback_cache_ttl_session_pacing.md`

**Finding:** A ~2-hour idle gap between messages cost approximately +3% of the 5-hour
session quota on the first message after the gap (rehydration of evicted prompt cache).
Observed directly: quota jumped from 2% to 5% on a single message turn after a 2h pause.

**Implications:**

| Pause duration | Cache state | Rehydration cost |
|---|---|---|
| < 5 minutes | Inner cache alive | Negligible |
| 5–120 minutes | Outer cache alive | Small |
| > 120 minutes | Full eviction | ~3% session quota |

For a 5-hour window: 4-5 idle gaps of 2+ hours each could burn 12-15% of session quota on
cache misses alone — comparable to the productive work cost in a well-paced session.

**Pacing rule:** Treat "no agent activity for more than 1.5 hours" as a state to break out
of. Either dispatch a thin keep-alive (status check, ensemble scan, cascade) or explicitly
end the session and start fresh. Idle waiting is not free.

---

## 5. Vacuous-Test Catches: Deep Review's Load-Bearing Contribution

Four vacuous-test instances in this session (from `docs/articles/SESSION_2026_04_24_PHASE_3_ECONOMICS_AND_LEARNINGS.md`
section 6, and the catalog in `docs/articles/DEEP_REVIEW_FIX_FORWARD_CATALOG_2026_04_24.md`).
All share the same structure: the assertion is true under both correct behavior AND under
the specific bug.

### 5.1 Case taxonomy

| PR | Claimed behavior | Vacuity mechanism |
|---|---|---|
| #6155 | Symlink guard prevents `Create Missing Dirs` for out-of-workspace paths | Mock blocked `mkdir` regardless of guard state; removing the guard entirely still passed |
| #6308 | Recovery metrics classify dirty files into four buckets | `profile != default()` passed on any non-zero struct; wrong-bucket allocation also passes |
| #6342 | `no feature ':all'` clears all features | Baseline was already empty; the assertion proved nothing about the clearing operation |
| #6396 | `perf_scorecard` computes median and p95 correctly | Arrays were large enough that nearest-rank formula never needed boundary clamping; off-by-one at boundaries would still pass |

Additional vacuous-test patterns from the deep-review catalog (entries 8, 9, 13):
- `assert_eq!` inside `proptest!` blocks disables shrinking — tests find failures but make
  them impossible to debug.
- Vacuous type invariants (`u32 is non-negative`) add no coverage signal.
- `assert!(true)` — never fails.

### 5.2 Detection heuristic

After writing a test, ask: "What is the simplest mutation of the production code that
would still pass this test?" If the answer is "remove the feature I just added," the test is
vacuous.

### 5.3 Fix pattern (uniform across all cases)

Add a positive-control assertion that would fail under a buggy implementation:

- **Mock blocks real code:** Also test the non-mocked path. Verify the code DOES run when
  it should.
- **Non-zero check only:** Assert specific field values (`count == 3`, not `count != 0`).
- **Empty baseline:** First assert the setup operation populated the state, then assert the
  teardown operation cleared it.
- **Missing boundary:** Add single-element and boundary-size inputs explicitly.

### 5.4 Why haiku review cannot catch this

The standards pass checks banned patterns, formatting, and scope. It does not reason about
whether an assertion constrains the code's behavior. This is load-bearing work for the sonnet
deep-review pass — it requires reading the test, the implementation, and the mental model of
what mutations would survive.

---

## 6. Stale-Base Security Regression Risk

From `docs/articles/SESSION_2026_04_24_PHASE_3_ECONOMICS_AND_LEARNINGS.md` section 5.

**The pattern:** When a security cluster merges, PRs that were branched before the merge
can silently undo the security guards when they rebase. The reversal produces a clean diff
against the PR's declared base — no banned patterns, correct formatting, in scope — but
removes lines that were recently added to the target file by the security cluster.

**Security cluster that exposed this (merged 16:00-16:05 UTC 2026-04-24):**

| PR | Guard added |
|---|---|
| #6220 | `MAX_DISABLED_WARNING_CATEGORIES` cap: bounded growth of pragma tracker warning-category set |
| #6219 | `path_to_relative_string()` workspace guard: `use lib` path injection respects workspace boundaries |
| #6221 | `should_parse_document()` size + binary guard: size limit and binary-content check before full parse |

PRs #6314, #6333, and #6367 were all identified as having diverged before this cluster
merged. Each would silently remove one or more of these guards if rebased without
conflict-aware review.

**Why haiku review misses this:**
- Standards pass checks the diff against the PR's base commit, not against current master.
- A 5-line security guard removal produces a clean diff on its own — no banned patterns,
  correct format.

**Detection sequence (three-layer):**

1. **diff-auditor:** Compare the PR diff against current master HEAD, not just the PR base.
   Look for recently-added lines that are absent from the PR.
2. **maintainer-pr:** Review whether any removed lines were recently added intentionally.
3. **deep-review:** Verify that the implementation still contains all security guards
   present in current master.

**Mitigation:** When a security cluster merges, immediately run `gh pr update-branch` across
the top-N highest-risk open PRs (those touching the same files). This converts a potential
silent reversion into a visible merge conflict that can be reviewed explicitly, rather than
a transparent erasure.

---

## 7. Pre-Existing Bug Catches via PR-Responder Investigation

From `docs/articles/SESSION_2026_04_24_PHASE_3_ECONOMICS_AND_LEARNINGS.md` section 7.

**The case:** PR #6308 (recovery-salvage metrics) had a CI failure on
`test_execute_command_recovery_timeout`. The initial assumption: the PR broke the test.

Investigation revealed: `Command::new()` in `execute_command/provider.rs` was not setting
`.stdout(Stdio::piped())` and `.stderr(Stdio::piped())`. The provider relied on inheriting
the parent process's stdio, which works interactively but fails in CI where inherited stdio
is unavailable or buffered differently. The PR-responder found this by running the failing
test in isolation against current master before modifying the PR branch.

**Finding:** The test was right; the provider was wrong. The bug was master-side, latent
since the provider was written, and invisible until CI ran without interactive stdio.

**Pattern for pr-responders:** When investigating a "PR-specific" CI failure on a test
that the PR does not directly touch, run the failing test in isolation against master before
attributing the failure to the PR. Distinguishing "PR broke this" from "master was already
broken" is a required first step, not an optional optimization.

**Forward-looking implication:** Tests that pass via inherited stdio may be silently broken
— they do not actually verify output content. Adding explicit `.stdout(Stdio::piped())` to
command construction exposes this latent class of failures. The 13 tests in PR #6308 that
were previously passing "by accident" are now properly isolated.

---

## 8. The Forensic-Doc-PR Cadence

This document is the sixth forensic retrospective produced in this session:

| PR | Document | Phase |
|---|---|---|
| #6106 | Session 2026-04-24 throughput cycle | Phase 1 |
| #6148 | Economic maturity + deep-review catalog + architecture audit | Phase 1 extension |
| #6161 | Extended throughput session retrospective | Phase 2 boundary |
| #6445 | Continuation session economics + learnings | Phase 2 |
| #6449 | Phase 3 economics: master bit-rot, vacuous tests, stale-base security | Phase 3 |
| This document | Full-session learnings catalog | Session close |

Each document in the series is both a record and a prompt input. The deep-review catalog
(#6148) was cited in the phase-2 doc. The master bit-rot playbook (from phase 1) is cited
in phase 3. The session-close learnings catalog (this document) synthesizes the memory files
into a single discoverable artifact.

**Why admin-merge cadence matters:** Each doc is merged same-day, making its patterns
available to future agents within the same session window. A retrospective that lives in a
PR for 3 days teaches no one. The admin-merge pattern converts session-time learning into
next-agent-call context.

**The cadence as a load-bearing mechanic:** The series is its own evidence that the wisdom
loop (agent-wrapup → memory file → future agent reads memory → avoids mistake) is
functioning. Each session document teaches the next session. The pattern has now been
stable across Sessions 6, 7, 2026-04-23, and 2026-04-24.

---

_Related memory files (all created 2026-04-24):_
- `feedback_multigate_catches_cheap_model_drift.md`
- `feedback_upstream_research_improves_pr_quality.md`
- `feedback_prompt_generation_is_cheap_web_thread.md`
- `feedback_ambiguous_origin_master_branch.md`
- `feedback_label_skill_silent_failure.md`
- `feedback_master_bit_rot_recurrence_pattern.md`
- `feedback_cache_ttl_session_pacing.md`

_Related docs/articles:_
- `docs/articles/SESSION_2026_04_24_ECONOMICS.md`
- `docs/articles/SESSION_2026_04_24_PHASE_3_ECONOMICS_AND_LEARNINGS.md`
- `docs/articles/DEEP_REVIEW_FIX_FORWARD_CATALOG_2026_04_24.md`
- `docs/articles/VERIFICATION_LADDER_PER_LAYER_ROI.md`
