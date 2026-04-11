# Swarm Operations: The 2026-04-11 Metric-Scoping Session

*A swarm-level retrospective on how the agent fleet coordinated, drifted, collided,
and recovered during a day spent planning the metric-stack umbrella (#4062) and
reality-checking the features catalog. This article is the operational companion
to the project-level wisdom retrospective at
[docs/project/wisdom/2026-04-11-session-learnings.md](../project/wisdom/2026-04-11-session-learnings.md);
that one is about perl-lsp, this one is about the swarm running on top of it.*

---

## The Session in One Sentence

A day of sustained scouting, scorecard planning, and reality-checks against the
features catalog — with roughly a dozen unrelated PRs merging in parallel from
other operators, seven scout-to-plan-review chains in flight, and a single
30-minute verification run that prevented a compound semantic failure from
landing on master.

## Why a Second Retrospective

The wisdom retrospective (commit `aa40d01c`)
captures the **project-level** learnings: systematic underselling of capability,
four distinct failure modes for "measurement exists but isn't wired," reviewers
sharing blind spots about Perl semantics, the already-done rate, the three truth
surfaces (catalog / tests / CI) that can each lie independently.

What that document does not capture is what the swarm itself looked like while
producing those findings. How did agents coordinate? Where did they drift? What
collisions were absorbed silently by the worktree isolation, and which ones
leaked? Which agent type was the single highest-ROI safety gate, and why did
that one gate catch things that five other agents missed?

This retrospective answers those questions. It is observational — the process
rules that came out of the session are being captured separately in
CONTRIBUTING.md updates, CLAUDE.md pipeline-table edits, and memory files. What
lives here is the raw account of how the swarm ran, so future operators can
recognize the same patterns when they happen again.

---

## What Went Well

### The research-verifier as critical safety gate

Single highest-ROI agent of the session, by a wide margin.

PR #4090 proposed that
pragmas inside Perl phase blocks (`BEGIN`, `END`, `INIT`, `CHECK`, `UNITCHECK`)
propagate to the surrounding file scope, citing `perlmod` and `perlop` as
authority. It went through the full pipeline:

| Stage | Outcome |
|-------|---------|
| Scout | Filed the gap, cited external docs as the premise |
| Builder | Wrote 6 tests confirming the premise, implementation passed them |
| First-pass reviewer | Found no standards issues, approved |
| Deep reviewer | Verified parser invariants, traced recursion paths, checked nested-phase cases, confirmed non-vacuous assertions, approved |
| Label receipt | `merge-ready` applied |

Four agents plus the original scout — all holding the same false belief, all
reasoning from the same shared premise. None of their work was sloppy. The deep
reviewer's analysis was thorough within its frame. The tests were green. The
label receipt was valid. The PR was minutes from merging.

Then the research-verifier ran one command:

```perl
perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'
```

The output: `ok`. `$x = 1` compiles without a `Global symbol "$x" requires
explicit package name` error because strict is lexically scoped to the `BEGIN`
block and does not propagate. The premise was false. Not subtly wrong — wrong
at the ground-truth level, in a way a five-line Perl program makes obvious.

The cost of the verification: under a minute of shell time. The cost of the
near-miss, had #4090 merged: PR #4052's
`walk_node` workaround (already on master) would have been retroactively
validated as correct companion behavior; future scorecard tests for phase-block
pragma scope would have been built against false expectations; and the
`PragmaTracker` at the core of the metric stack would have been modeling
semantics that do not exist. Untangling would have taken a revert of #4090, a
revert of #4052's workaround, rewriting nine tests, updating the tracker
documentation, and retracting anything downstream that had already started
citing the corrected behavior. Estimated recovery: half a day of elapsed time
and 2–3 builder slots.

The research-verifier was dispatched because the orchestrator — reading the PR
body for the merge-ready decision — noticed the external citations and routed
a late verification out of caution. Had that routing decision gone the other
way, #4090 would have merged.

Issue #4100 was filed to
track the revert cascade and the follow-up audit of 30+ adjacent pragma claims
(the proactive audit found 30 of 31 other semantics claims correct — not just
the one in #4090 — so the verification run was worth doing beyond the immediate
incident). This particular catch also seeded a parallel CONTRIBUTING.md update
making research-verifier mandatory for any PR whose body cites external
semantics.

**Takeaway, observational:** when five agents share a premise, they are no
longer independent checks. They are a single check repeated five times. The
research-verifier's job is to break the consensus illusion by consulting ground
truth. It did, once. That once was worth the entire session.

### The parallel scout wave

Roughly a dozen scouts ran simultaneously during the mid-session scoping pass,
each investigating a different subsystem against the features catalog. The
majority of the session's structural findings — the 14 uncatalogued DAP
handlers, the 8 other undersold subsystems (#4114),
the wiring gaps that became issue #4102,
the reference-model research in #4099
— came from this wave. Parallel scouting at this scale was possible because:

1. **Crate isolation held.** The microcrate architecture meant most scouts
   could read their target crate without interfering with each other's working
   directories or analyses.
2. **The catalog was the common artifact.** `features.toml` served as a shared
   point of comparison — every scout could ask "does my finding appear here?"
   and get an unambiguous yes/no.
3. **Scope boundaries were narrow.** Scouts were assigned subsystems, not
   problems. "Audit refactoring against the catalog" is a finite task. "Find
   bugs in refactoring" is not.

### Plan-reviewer synthesis

The plan-review stage consolidated findings across multiple scouts into three
coherent umbrellas: #4102
(test-wiring guards), #4105
(4-layer ratchet model), and #4106
(xtask-metrics framework). Individual scouts filed narrower findings; the
plan-reviewers grouped them into umbrellas with shared fix approaches. This
works when the plan-reviewers have enough context to see the cross-cutting
pattern — which only happens when scout reports are structured consistently and
the plan-reviewer is given access to multiple adjacent reports at once.

---

## What Went Sideways

### Swarm contamination is persistent, not one-off

Across the session, the main checkout accumulated cumulative damage from
builder worktrees. No individual incident was catastrophic, but the pattern
shows how worktree isolation is softer in practice than it is on paper.

**Branch flips on main's HEAD.** The main checkout ended up on feature branches
at least three times during the session — `worktree-agent-*`, a stray
`fix/clippy-needless-borrow`, and at one point `fix/diagnostics-eval-pragma-scope-3489`.
Each time the fix was the same: `git checkout master`, verify the working tree
matched HEAD, clean up. But the recurrence of the problem across unrelated
agents suggests a persistent race condition in how worktree agents interact
with the main checkout's git state.

**File leaks into main's working tree.** At least five builders working in
isolated worktrees somehow modified files in the main checkout. Known leaks:

- `strict_warnings.rs` — leaked twice by different agents
- `execute_command/mod.rs` and `execute_command/provider.rs` — from a single
  worktree agent
- `runtime/language/misc.rs`
- Three files from the #4085 builder

These files had no business being in the main checkout's working tree. The
agents were in their own worktrees; the edits were intended for
`.claude/worktrees/agent-<hash>/...` paths but resolved to the top-level
repo paths instead. Windows path ambiguity (`H:/` vs `/h/` vs `H:\`) is the
leading suspect — a path that looks absolute to one resolver looks relative
to another, and on Windows both forms appear in the same session depending on
which tool is invoking file I/O.

**Nested worktree directories.** Worktree `agent-a6e72727` was found to
contain `agent-a8f1af70` inside its own `.claude/worktrees/` subdirectory.
One agent had run `git worktree add` while already inside a worktree,
creating a worktree-within-a-worktree structure that `git worktree remove`
refuses to touch cleanly. Recursive leak; manual `rm -rf` was required
(and even that was partially blocked by Windows file locks).

**Stray commits on local master.** The session's git log showed a revert +
reapply pair from some agent's confusion about what was already on master,
plus a clippy fix that ended up *both* as a merged commit on the main branch
and as an unpushed local branch `fix/clippy-needless-borrow` that duplicated
the merged fix. Two independent observations of the same bug, without
coordination between them.

**Zombie worktree directories.** The `.claude/worktrees/` tree accumulated
17 completed-but-undeleted worktree directories before the mid-session
cleanup. Several refused to delete via `rm -rf` or `git worktree remove
--force` due to Windows file locks held by processes that had already
exited at the OS level. Cleanup required waiting for the locks to release
organically.

None of this was caused by a single misbehaving agent. Each incident was a
different agent making a different localized mistake. The cumulative effect
— a repeatedly contaminated main checkout, recovery actions scattered
through the session — is the cost of the swarm operating model when worktree
hygiene drifts.

### External merge races outpaced scouting

During a single session, roughly 30 PRs merged externally from other operators
and scheduled work. Several of those preemptively fixed issues that scouts
were actively investigating or that plan-reviewers were about to dispatch
builders against:

| Investigation | Resolved by | Resolution time |
|---------------|-------------|-----------------|
| #4072 (stale test) | #4082 | Merged before scout reported |
| #4073 (Windows path bug) | #4081 | Merged before builder started |
| #4080 (panic baseline drift) | Symptom of #4073, closed on resolution | — |
| #4096 (pre-push friction) | #4088 | Merged before my fix was even approached |
| #3513 (multi-root) | Shipped in #3984 | Already on master when scoped |

This is healthy for the project. The swarm is self-healing at the per-PR level
— somebody fixes something before somebody else has to. But it exposes a gap
in scout discipline: `scout-dedup` currently only checks the open issue queue.
It does not check for recent merged PRs that address the same concern. A scout
that runs on hour-old state against a swarm that is merging every ten minutes
will scout issues that were fixed 45 minutes ago.

The fix is a one-line addition to the scout workflow:

```bash
gh pr list --search "<keywords>" --state merged --limit 20 --search "merged:>1w"
```

That `merged:>1w` filter is as important as the open-issue dedup query. Without
it, roughly half of this session's scout-to-plan-review chains were reinventing
fixes that had already merged.

### Agent drift beyond what definition files can prevent

Several agents had complete Todo lists in their definition files and still
skipped terminal skills. This is not a new problem — the
`feedback_agent_prompt_todo_list.md` memory note from a prior session covers
it — but the session produced three fresh data points showing that having the
Todo present is not sufficient to guarantee the Todo is executed.

**Scouts completing analysis without posting the report.** A scout with
`/scout-report` as step 8 of 9 completed the investigation, wrote all the
findings internally, and returned to the orchestrator without ever posting the
comment. The orchestrator had to relay findings manually, which defeats the
point of the terminal skill — the scout-report comment is how the issue queue
becomes the handoff surface for downstream agents. No comment, no handoff.

**Deep reviewers setting `reviewed-deep` but forgetting `/pr-ready`.** Observed
on four PRs during the session: the deep reviewer completed its analysis, set
the `reviewed-deep` label, and stopped. The PR remained in draft state because
no agent called `/pr-ready` to exit draft. From the label's perspective the PR
was done; from the CI pipeline's perspective it was still a work-in-progress.
Ops agents looking for merge-ready PRs had to scan past the drafts. The
behavior happened on #4046,
#4077, and
#4079; a fourth
observation, #4090, was
held in draft for the separate reason that its premise was false.

**Reviewer role confusion.** #4087
was a docs-only PR to `.claude/agents/` that the first-pass reviewer
fast-tracked with *both* `merge-ready` and `reviewed-deep` labels — despite
`reviewed-deep` being explicitly reserved for the deep-review agent. The PR
was correct and the fast-track was reasonable; the label application was not.
Issue #4097 was filed to
document the gap and was closed the same day via a CLAUDE.md pipeline-table
clarification that docs-only PRs may reach `merge-ready` without passing
through `reviewed-deep`. The label semantics needed to be tightened, not the
reviewer's judgment.

The common thread across all three cases: when an agent has a definition file
with explicit steps and still skips one, the skip tends to be at a **terminal
action** — the comment post, the label exit, the draft flip. Terminal actions
are where the agent's local work becomes visible to the rest of the swarm.
Definition files describe the work; they do not describe the handoff surface.
The drift is therefore most consequential exactly where it happens most often.

---

## Operational Curiosities Worth Remembering

These are the oddities that do not cleanly fit "went well" or "went sideways"
but are worth noting for future operators:

### The same bug observed twice, independently

PR #4052 merged with a
clippy regression that triggered a CI hotfix chain. An ops agent autonomously
created PR #4098 — a 2-line
`&idx` → `idx` fix — to recover the gate. Separately, the same bug had already
been noticed by a different agent earlier in the session and committed to a
local branch `fix/clippy-needless-borrow` that was never pushed. The branch
was found during the worktree cleanup sweep, still unpushed, with a commit
that was now identical in effect to the merged #4098. Two agents, same bug,
same fix, neither aware of the other. One of them made it to master; the
other became a local artifact that had to be manually deleted.

**What this says about the swarm:** uncoordinated parallelism catches bugs
faster but wastes work silently when two agents independently find the same
thing. The merged fix cost 2 lines; the unpushed fix cost somebody's attention.
Deduplication would have saved the attention.

### The Windows `os error 206` saga

Four related issues, each a different failure mode on the same
developer-experience surface (Windows long-path handling + git hook integration):

| Issue | Failure mode | Resolution |
|---|---|---|
| #4044 | `subagent-stop.sh` mis-derived issue number | Merged #4064 |
| #4047 | `cargo fmt --check` crashed on long paths | Merged #4061 |
| #4073 | Path-separator comparison broke panic allowlist | Merged #4081 |
| #4080 | Panic baseline drift visible only as symptom | Closed as superseded by #4073 fix |

All four surfaced during this session because the metric-stack work
exercised the hook surface harder than usual: more scout comments posted
through `subagent-stop`, more fmt runs against long-pathed worktrees, more
panic-baseline checks against newly-touched code. The pattern is worth
noting: when a subsystem gets exercised at higher volume, latent bugs that
were "known but tolerated" all surface at once.

### The test-wiring 4-failure-mode family

The session found four distinct ways that a test can exist on disk but
produce no signal:

| Mode | Example | What fails |
|---|---|---|
| A — not compiled | `unclosed_block_recovery_tests.rs` missing `mod` declaration in `mod.rs` (#4079) | Rustc never sees the file |
| B — not exercised | `multi_root_workspace_tests.rs` requires unset feature flags (#4068) | Tests compile, CI never runs them |
| C — running but wrong | `strict_warnings.rs` tests passed via `walk_node` workaround encoding false semantics (#4100) | Tests run, assertions test the wrong thing |
| D — structured output unused | `cargo mutants` supports `--json`, CI invokes it without the flag (#4070) | Data exists but never captured |

Individually each is a small bug. Collectively they represent four layers where
measurement can silently fail without tripping any alarm. Issue
#4102 covers guards for
A, B, and C. Guard D was added as a comment on #4102 — a one-time audit, not
an automated gate, because the question "is there a structured-output flag
we are not passing" has to be asked per-tool by a human.

The swarm-operational observation: the four modes were found by four different
agents, each investigating a different PR. No single scout would have spotted
all four, because none of the individual incidents looks like a pattern in
isolation. The pattern only appeared once someone put the four findings next
to each other in the plan-review stage. **Cross-scout synthesis is where the
systemic view emerges.**

### Plan-reviewer disagreement on shared infrastructure

Two plan-reviewers running in parallel on overlapping scope — the metric-stack
umbrella #4062 and the
canonical gold-corpus planning work — independently chose different locations
for the shared gold corpus. One proposed `crates/perl-lsp/tests/fixtures/gold/`;
the other proposed `test_corpus/gold/`. Both were defensible choices; neither
reviewer was wrong about their own subsystem. But they were wrong about each
other.

The orchestrator resolved the disagreement by picking the cross-crate-neutral
option (`test_corpus/gold/`) and leaving a comment on both plan-review threads
explaining the choice. The lesson is small: when two plan-reviewers are
running on adjacent scope, the scope boundaries need to be flagged in their
input context, not discovered through the output clash.

### Multi-operator swarm dynamics

This session was clearly running alongside other operators — the external
merge races noted above were not theoretical. For the session operator that
meant constant state freshening: the opening PR list was stale within an
hour, and the scout-dispatch decisions based on that list had a half-life
shorter than the scouts themselves. The swarm is self-healing as a system,
but coordination as an operator requires much fresher state than a once-at-
session-start snapshot can provide. A mid-session `gh pr list` refresh every
60–90 minutes would have prevented the duplicate dispatches.

---

## Cross-References

### The wisdom retrospective (project-level)

[docs/project/wisdom/2026-04-11-session-learnings.md](../project/wisdom/2026-04-11-session-learnings.md)
covers the **project-level** patterns from the same session: systematic
underselling of features.toml (Pattern 1), the four failure modes for unwired
measurement (Pattern 2), reviewers sharing a blind spot on Perl semantics
(Pattern 3), the ~50% already-done rate (Pattern 4), swarm contamination as
cumulative phenomenon (Pattern 5), external merge races vs scouting state
(Pattern 6), and the research-verifier as highest-ROI agent (Pattern 7).

This article and the wisdom retrospective are intentionally complementary.
The wisdom document is the canonical project-level record of the day's
learnings. This document is the operational record for swarm operators —
what the day felt like from the orchestrator's seat, which agent behaviors
were load-bearing, and which coordination failures were absorbed silently
versus surfaced painfully.

### Canonical artifacts

| # | What | State |
|---|---|---|
| #4062 | Metric-stack umbrella — the session's triggering context | Open |
| #4099 | Reference-model research (rust-analyzer, gopls, pyright, clangd) | Open |
| #4100 | Pragma phase-block revert — primary evidence for the false-premise catch | Closed |
| #4102 | Test-wiring guards A / B / C (and Guard D comment for structured output) | Open |
| #4105 | 4-layer ratchet model | Open |
| #4106 | xtask-metrics framework umbrella | Open |
| #4107 | DAP catalog undercount fix (102 → 116 capabilities) | Merged |
| #4114 | Additional 8 undersold subsystems | Open |
| #4097 | Reviewer fast-track label semantics for docs-only PRs | Closed (via CLAUDE.md update) |
| #4090 | The false-premise pragma PR — the near-miss that justified research-verifier | Closed without merge |

### Complementary memory files

- `feedback_verify_before_build.md` — prior data point on the already-done rate
- `feedback_deep_review_roi.md` — 12–16x ROI on two-pass review; now extended by the research-verifier story
- `feedback_swarm_worktree_contamination.md` — prior record of the worktree leak pattern, of which this session is the latest instance
- `feedback_agent_prompt_todo_list.md` — prior data point on Todo presence not guaranteeing Todo execution

---

## The Session as a Whole

The 2026-04-11 session was not defined by the 14 DAP handlers catalogued or the
8 subsystems found undersold or the 30-minute Perl verification that prevented
a compound cascade. It was defined by the gap between those headline findings
and the infrastructure that produced them. The swarm did productive work. It
also drifted, leaked, collided, and raced against other operators, all while
drifting through `.claude/worktrees/` at a file count that required an
ad-hoc cleanup pass. The productive work and the drift happened in the same
hours, with the same agents, and neither set of observations is complete
without the other.

What made the difference between a session that produced canonical artifacts
(#4062, #4099, #4100, #4102, #4105, #4106, #4107, #4114) and a session that
would have produced only drift was exactly one agent type: the research
verifier. One verification run, one minute of shell time, one near-miss averted.
Every other agent did its assigned job and would have approved a PR built on
a false premise. The swarm's self-healing property is real but it is not
symmetric — it works reliably for mechanical failures (a test that does not
compile, a clippy warning, a merge conflict) and less reliably for semantic
failures (a premise that is wrong in exactly the way that makes internally
consistent tests green).

The session's most durable lesson is the one the research-verifier catch
implied rather than stated: **consensus across agents is not evidence of
correctness when the agents share a premise.** The pipeline catches
disagreement; the research-verifier catches agreement that is wrong.

Every session the swarm runs will produce drift. Worktree leaks, stale
branches, missed Todo steps, agents racing other operators — these are the
operational cost of the swarm being a swarm. The goal is not to eliminate
them. The goal is to keep them small enough to absorb while the productive
work is producing, and to keep at least one agent at every stage pointed at
ground truth instead of at the other agents' output.

That is what the 2026-04-11 session was about. This article is the record of
how it looked from the orchestrator's seat.

---

*2026-04-11 session retrospective — swarm-operations view. For the project-level
view, see [docs/project/wisdom/2026-04-11-session-learnings.md](../project/wisdom/2026-04-11-session-learnings.md).*
