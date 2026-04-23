# Session Retrospective: 2026-04-11

**Type**: Meta-retrospective — metric-scoping + scorecard planning + reality-check wave
**Scope**: Not a single issue->PR->merge trail. This is a session-level synthesis across the full day's work.
**Canonical artifacts**: #4062 (metric-stack umbrella), #4099 (reference-model research), #4100 (pragma revert),
#4102 (test-wiring guards), #4105 (ratchet model), #4106 (xtask-metrics framework), #4107 (DAP catalog, merged)

---

## Pattern 1: Systematic underselling of real capability

### What happened

`features.toml` catalogued 102 total capabilities entering the session. PR #4107, which merged during this
session, found 14 uncatalogued DAP handlers by reading `dispatch.rs` directly — a 14% undercount in just the
DAP subsystem. The corrected total is 116 (87 LSP + 24 DAP + 5 extension). A substrate sweep scout
(a04275492) independently found 8 more uncatalogued opportunities across refactoring, hover, completion,
code actions, inlay hints, semantic tokens, benchmarks, and workspace index. Most strikingly, the
`perl-refactoring` crate has 264 tests and workspace-wide rename working, but zero entries in `features.toml`.

The plausible true capability count is 130-150, meaning the public-facing number may be off by 25-30%.

### Why it happened

The catalog was maintained by hand, entry by entry, as features shipped. Nobody ran a systematic audit
comparing `dispatch.rs` handler arms to `features.toml` entries. The DAP undercount went unnoticed because
the catalog looked plausible — 10 entries for a young DAP implementation sounds reasonable — and no automated
check enforced parity between implementation and catalog.

### What to do about it

1. Add an xtask command that cross-checks `dispatch.rs` handler arm count against `features.toml` DAP entries.
   This is ~50 lines and could be part of `cargo xtask features invariants`.
2. Before any external announcement citing a capability count, run a codebase-first audit (grep for handler
   arms, test counts, feature entry points) rather than trusting the catalog at face value.
3. The refactoring crate gap is the canonical example: if a crate has >50 tests and no `features.toml`
   entries, something is wrong. Consider adding this as a `just doctor` check.

**Evidence**: #4107 diff, scout comment on #4062, scout a04275492 findings.

---

## Pattern 2: "Measurement exists but isn't wired" — four distinct failure modes

### What happened

Four separate incidents during the session each represent a different way that measurement can exist on disk
but produce no signal:

1. **Not compiled** (#4079): `unclosed_block_recovery_tests.rs` had 6 tests with real assertions. The file had
   never compiled because the `mod unclosed_block_recovery_tests;` declaration was missing from `mod.rs`.
   Duration dormant: unknown.

2. **Not exercised** (#4068): The 1020-line `multi_root_workspace_tests.rs` was added in PR #3984. It requires
   `--features workspace,expose_lsp_test_api` plus `PERL_LSP_WORKSPACE=1`. No justfile recipe or CI config
   activates those flags. The tests compile but never run in CI.

3. **Running but wrong** (#4100): Three integration tests in `perl-lsp-diagnostics` were passing via a
   `walk_node` workaround in `strict_warnings.rs` that encoded false behavior. The workaround made the tests
   green while the underlying assertion was based on an incorrect model of Perl semantics.

4. **Tool supports `--json`, CI doesn't pass it** (#4070): `cargo mutants` supports `--json` output that
   enables structured result capture. CI invokes `cargo mutants` without the flag. The data was never written.
   #4102 proposes Guards A/B/C for cases 1-3 but has no guard for case 4.

### Why it happened

Each failure mode exploits a different gap in the feedback loop. No single guard catches all four. The common
thread is that test/measurement infrastructure was added without a corresponding CI activation step — the
build and the test were treated as the complete work, not the build, the test, and the wiring into CI.

### What to do about it

Issue #4102 covers Guards A (mod-declaration audit), B (feature-flag exercise audit), and C (red-test
tracker). A **Guard D** is needed for case 4: for every tool invoked in CI recipes, audit whether that tool
has structured output flags (`--json`, `--format`, etc.) that the invocation is not using. Guard D is a one-
time audit with a human decision for each case, not an automated gate. File as a comment or extension of
#4102.

The deeper fix: treat "wiring the measurement into CI" as a mandatory acceptance criterion for any PR that
adds test infrastructure. It is currently not listed explicitly in the builder checklist.

**Evidence**: #4079, #4068, #4100, #4070, #4102.

---

## Pattern 3: Multiple reviewers sharing the same blind spot

### What happened

PR #4090 proposed that pragmas inside Perl phase blocks (BEGIN/END/INIT/CHECK/UNITCHECK) propagate to the
surrounding file scope. This claim was cited as "per Perl semantics (perlmod, perlop)." The PR went through:
a builder who wrote 6 tests confirming the premise, a first-pass reviewer who found no issues, a deep
reviewer who verified nested phase blocks, edge cases, and vacuous-assertion risk — and still approved.
The tests passed. CI was green. The label receipt was written. `merge-ready` was set.

The research-verifier ran `perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'` and got `ok: strict not
active`. The premise was false. Pragmas in phase blocks are lexically scoped to the block; they do not
propagate. The orchestrator re-verified and removed `merge-ready`. Issue #4100 was filed for the cascade of
corrections needed across #4090, the already-merged #4052, and the 9 tests that needed rewriting.

### Why it happened

The deep reviewer's analysis was technically thorough: it checked parser invariants, traced recursion paths,
verified range tracking, confirmed tests were non-vacuous. None of that work required running Perl. The
reviewer trusted perlmod documentation rather than empirical verification. The same false belief was present
in the scout's issue, the builder's PR body, the tests, and both reviewers — a consistent error introduced
at the scout stage and propagated faithfully through the pipeline.

Green tests are not evidence of correct Perl semantics. They are evidence that the code is internally
consistent with whatever premise the tests encode.

### What to do about it

The research-verifier agent exists exactly for this case. The problem is that it was dispatched late (after
`merge-ready` was set) and was treated as optional. For any PR whose justification cites Perl language
semantics, LSP spec behavior, DAP protocol behavior, or crate API contracts, research-verifier dispatch
should be **mandatory before `merge-ready` is applied** — not after.

The reviewer-deep workflow should include an explicit checkpoint: "Does this PR body cite any external
semantics claim? If yes, block on research-verifier before proceeding." The current checklist does not
have this gate.

A dedicated follow-up issue (#4102 comment or new) should track making research-verifier mandatory in the
reviewer-deep skill for external-claim PRs.

**Evidence**: PR #4090, research-verifier comment at #4090#issuecomment-4229309824, issue #4100, the
proactive audit comment on #4100 (30 of 31 other semantic claims verified correct — the audit was worth
running).

---

## Pattern 4: Approximately half of scoped work was already done

### What happened

The session filed roughly 15 follow-ups and scoped 10+ gaps that turned out to be already implemented or
already tracked:

- #4072 (test bug) — already fixed by #4082
- #4073 / #4080 (Windows path fix) — already in #4081
- #3513 multi-root support — shipped in #3984
- #4067 position-aware `@INC` — already correct in the implementation
- #4084 tests — passing (via workaround encoding the wrong premise, not a genuine gap)
- #3472 `qw()` imports — shipped in #3808
- #3522 / #3523 — already 30%-80% done
- 14 DAP handlers (#4107) — implemented in `dispatch.rs`, just uncatalogued
- 8 substrate items — implemented but uncatalogued (scout a04275492)

The already-done rate is consistent with the memory note from prior sessions ("42% of builders found work
already done"). But this session's rate is higher because the scoping wave was broad and fast — scouts were
looking at an area they hadn't recently swept.

### Why it happened

Scouts were checking open issue state and codebase grep, but not checking recent merged PRs. A common
pattern: an issue filed 3-6 weeks ago, a PR merged in the interim, the issue still open (closed by the
merge but not explicitly). `gh issue list` returns open state; it does not show recently-merged fixes.

The scout-dedup step checks for duplicate open issues. It does not check for recent merged PRs that address
the same concern.

### What to do about it

Add a step to the scout workflow: before filing a new issue, run:

```bash
gh pr list --search "<keywords>" --state merged --limit 20
```

If a merged PR from the last 30 days addresses the finding, close it as `already-fixed` rather than filing.
This is the scout equivalent of what `accuracy-verify-status` does for already-fixed issue detection — but
focused on merged PRs, not open PRs.

The `scout-dedup` skill should document this step explicitly. It currently focuses on open issues only.

**Evidence**: The pattern is consistent across this session and prior sessions (SESSION_3_LEARNINGS item 9,
SWARM_SESSION_2026_04_10 friction log item 9).

---

## Pattern 5: Swarm contamination is cumulative and persistent

### What happened

Across this session: multiple branch flips on main checkout, files from 5+ builder worktrees leaking into
the main working tree, nested worktree directories (worktree a6e72727 containing worktree a8f1af70),
stray revert + reapply commits on local master, an unpushed `fix/clippy-needless-borrow` branch from an
agent that committed to the wrong branch, and 17 zombie worktree directories purged mid-session.

No single incident was catastrophic. The cumulative cleanup cost was non-trivial — several recovery actions,
manually verifying that master's working tree matched HEAD, cleaning nested worktrees that `git worktree
remove` refused to touch.

### Why it happened

Each worktree agent is given a worktree path. On Windows, path resolution ambiguity (`H:/` vs `/h/`) causes
agents to sometimes resolve absolute file paths against the main checkout rather than their assigned
worktree. Once one agent has leaked a file, the next agent may find the file "already modified" and either
overwrite it or panic. The nested worktree problem occurred when an agent ran `git worktree add` inside an
existing worktree, creating a worktree-within-worktree structure that git does not support cleanly.

### What to do about it

1. The `agent-preflight` skill should validate that the working directory is the expected worktree path
   before any edit. On Windows, normalize both the working directory and the expected path to forward-slash
   lowercase before comparing.
2. The worktree-manager should refuse to run `git worktree add` if `$PWD` is already inside a worktree.
   Check via `git rev-parse --is-inside-work-tree` and compare against the known worktree list.
3. `just clean-worktrees` must be in the pre-session checklist. It was present in CLAUDE.md at the time of
   this session but was not run before the wave. Consider making `just doctor` call it automatically.

**Evidence**: SWARM_SESSION_2026_04_10 friction log items 1, 5. Session-level git log showing stray commits.
Memory note `feedback_swarm_worktree_contamination.md`.

---

## Pattern 6: External merges outpace scouting state

### What happened

While this session's scouting wave ran, approximately 15 PRs merged from other operators and scheduled work.
Several were preemptive fixes for issues being scouted simultaneously. The swarm is self-healing at the
individual PR level but coordination requires fresher state checks than the session opening provides.

### Why it happened

The orchestrator's session-opening state snapshot (PR list, issue list, recent commits) goes stale within
30-60 minutes when the swarm is active. Scouts operating on hour-old state will scout issues that are
already in-flight or already fixed.

### What to do about it

For sessions longer than 2 hours with active parallel building, run a mid-session state refresh:

```bash
gh pr list --state open --limit 50 --json number,title,labels
gh issue list --label "in-build" --limit 30
```

Compare against the opening snapshot. Any issue newly labeled `in-build` or `merge-ready` in the interim
should be removed from the active scout queue before dispatching scouts against it. This is a 5-minute
check that prevents 30-minute wasted scout runs.

**Evidence**: Session observation. Consistent with `feedback_multi_pr_cargo_toml_race.md` pattern.

---

## Pattern 7: The research-verifier is the highest-ROI agent type in this class of work

### What happened

A single ~30-minute research-verifier run on PR #4090 prevented a compound failure. Had #4090 merged:

- `PragmaTracker` would model false pragma propagation at the core level
- #4052's workaround (already merged) would have been treated as correct companion behavior
- Future scorecard tests for strict/warnings would have been built against false premises
- The `perl-pragma` BDD test suite (9 tests) would have served as a false assurance baseline
- Untangling would have required: revert PR, revert workaround, rewrite 9 tests, update doc comment,
  notify any downstream scorecards — a minimum of 2-3 builder slots and a half-day of elapsed time

The research-verifier cost: one `perl -e` invocation that took under a minute to produce conclusive evidence.

The prior wave's deep-review-ROI memory note documents 12-16x ROI for two-pass review catching 4 real bugs.
This incident is in the same category: a verification step that costs little and saves disproportionate
rework. The difference is that deep review checks internal consistency while research-verifier checks
external truth.

### Why it happened

Research-verifier is listed in the pipeline as optional — dispatched when a reviewer has concerns about
external claims. On #4090, the first-pass reviewer had no concerns (the premise was stated confidently and
matched the reviewer's priors). The deep reviewer had no concerns (the code was internally consistent with
the premise). Neither triggered research-verifier dispatch.

### What to do about it

Make research-verifier **mandatory for merge-ready** when the PR body contains any of:
- citations to Perl documentation (perlmod, perlop, perlfunc, perlref, perlsyn, etc.)
- citations to LSP specification sections
- citations to DAP protocol sections
- claims about external crate API behavior citing docs.rs

The reviewer-deep skill's decision checklist should add: "Scan PR body for external semantics citations. If
found, dispatch research-verifier before proceeding to `merge-ready`. Do not approve based on internal
consistency alone when the premise is unverified."

This is a process change in the reviewer-deep agent definition, not a code change. A tracking issue should
be filed so the orchestrator can make the agent file edit in a controlled way (the agent files are control-
plane and require the lock).

**Evidence**: PR #4090 full comment thread, issue #4100, memory note `feedback_deep_review_roi.md`.

---

## Cross-cutting observation: the catalog, the tests, and the CI pipeline are three separate truth surfaces

The session surfaced that implementation can be correct, tests can be green, CI can be passing, and the
catalog can still be wrong — or the catalog can be right, tests can be green, and the behavior being tested
can still be wrong because the premise is false. These three surfaces (catalog, tests, CI) are checked by
different agents at different pipeline stages and currently have no structural enforcement of consistency
with each other or with external ground truth.

The metric-stack work (#4062, #4105) addresses the catalog gap. The test-wiring guards (#4102) address the
CI gap. The research-verifier mandatory dispatch (Pattern 7) addresses the premise gap. All three are
needed; none substitutes for the other.

---

## Memory updates warranted

The following patterns are durable enough to promote to project memory:

1. **Scout-dedup must check merged PRs, not just open issues** — add to `feedback_scout_dedup_merged_prs.md`
2. **Research-verifier is mandatory for external-claim PRs** — add to `feedback_research_verifier_mandatory.md`
3. **Guard D: CI invocation flags audit** — add to the #4102 scope as a comment

---

## Related canonical artifacts

| Artifact | Role |
|---|---|
| #4062 | Metric-stack umbrella — the triggering context for this session |
| #4099 | Reference-model research — rust-analyzer, gopls, pyright, clangd approaches |
| #4100 | Pragma phase-block revert — the #4090 incident write-up |
| #4102 | Test-wiring regression guards A/B/C |
| #4105 | 4-layer ratchet model |
| #4106 | xtask-metrics-framework umbrella (PRs 2-5) |
| #4107 | DAP catalog undercount fix — 102 -> 116 capabilities (merged) |
| #4097 | Reviewer fast-track label semantics — docs-only PR pipeline gap |
| PR #4090 | The false-premise pragma PR — primary evidence for Pattern 3 and 7 |
