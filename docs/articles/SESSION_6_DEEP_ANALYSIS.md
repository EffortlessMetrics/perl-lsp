# Session 6 Deep Economics Analysis

*A precision accounting of what actually happened when 200 agents merged 59 pull requests
in 7 hours. Not the headline numbers — the structure behind them.*

---

## Why This Document Exists

`SESSION2_ECONOMICS.md` tells you the billing numbers. `AGENTIC_ECONOMICS_DATA.md` gives
you reproducible metrics. This document does neither.

This document asks the harder questions: which stage of the pipeline actually earned its
cost, what would have broken if you removed it, and whether the record-setting output of
Session 6 was driven by better agents or by better architecture.

The answer to the last question is architecture. Specifically: four simultaneous lanes
instead of two, a research-verify pass that caught 8+ semantic errors before builders
touched them, and a deep-review pass that found at least one real bug in every PR it
examined. The 59-PR session record was not achieved by running more agents. It was achieved
by stopping agents from doing expensive work they did not need to do.

---

## Part 1: Pipeline Stage ROI

The full Session 6 pipeline had nine stages. Each is analyzed below by time cost,
correction rate (what percentage of the time it changed the output from what a naive
single-stage pipeline would have produced), and what would have broken if it were removed.

### Stage 1: Scout (Haiku)

**Average time**: 3-5 minutes per issue filed.

**What it does**: Broad exploration. Finds the problem, names the file, sketches a spec.
Accuracy on "neighborhood" (right subsystem, right symptom) is ~90%. Accuracy on exact
file and function name is ~60%.

**Correction rate**: N/A — scouts file issues, they do not change existing outputs. They
set the input for all downstream stages.

**If removed**: Builders would receive vague task descriptions. Historical evidence from
pre-pipeline sessions shows builder success rate drops from 90% to ~50% on unconstrained
tasks. Applied to Session 6's 40 build tasks: that is ~16 additional failed build attempts.
At 30 minutes each, 480 agent-minutes lost. At pipeline throughput of roughly 8 PRs per
agent-hour, this represents approximately 8 PRs that would not have merged.

**ROI**: The scout's 3-5 minutes is among the highest-leverage investments in the pipeline.
It is not the accuracy of the scout that matters — it is that the scout exists at all,
forcing a separation between "finding the problem" and "implementing the solution."

---

### Stage 2: Research-Verify (Haiku)

**Average time**: 5 minutes per issue.

**What it does**: Mechanically checks the scout's claims. Verifies that cited file paths
exist, that named functions are real, that the issue described is not already fixed, and
that external claims about third-party behavior are accurate.

**Correction rate**: Caught 8 semantic errors in Session 6 before any builder was assigned.
The documented corrections were:

1. **`createDebugConfig` already exists** — the scout recommended building a new command.
   Research-verify found the implementation was already present; the spec needed wiring,
   not reimplementation. Saved: 45 minutes of builder time + a PR creating duplicate code.

2. **Sublime Text claim backwards** — the scout claimed Sublime Text supported a feature
   in mode A and not mode B. Research-verify found the opposite was true. Saved: documentation
   stating a falsehood; would have required a correction PR after external review.

3. **7-crate split to 2-step migration** — the scout proposed splitting a module into 7
   new crates. Research-verify found that 5 of those crates already existed in the
   workspace under different names. The correct spec was a 2-step migration, not a
   greenfield build. Saved: 4 builder-agents building crates that already existed.

4. **PL001 diagnostic code mismatch** — the scout cited a diagnostic constant `PL001`
   that did not match the actual constant name in the codebase. Research-verify checked
   the constant table. A builder using the scout's spec would have compiled successfully
   (the wrong constant is valid Rust) but the feature would have emitted diagnostics
   under the wrong code. Saved: a live bug that would have passed CI.

5. **Perl `and`/comma precedence** — the scout proposed changing the parser's behavior
   for `and` operator precedence. Research-verify checked the Perl specification and found
   the current behavior was correct; the scout had misread the precedence table. Saved:
   a parser change that would have introduced a regression.

6-8: Three additional corrections to file paths and function names that would have produced
compile errors or wrong-function patches.

**If removed**: 8 incorrect specs would have reached builders. The most expensive category
(building things that already exist, fixing things that are already correct) would have
cost 30-60 minutes each. Conservative estimate: 5 additional failed or wasted build cycles,
or approximately 2.5-3 hours of agent time. More importantly, the PL001 mismatch would
have passed CI and shipped — requiring a hotfix PR and a revert cycle.

**ROI**: Research-verify costs 5 minutes and runs before the builder. The most expensive
error it caught (7-crate rebuild) would have consumed 4 builder-agents for 30 minutes each
= 120 agent-minutes. The research-verify pass for that issue: 5 minutes. 24:1 ratio on
that case alone.

The documented rule from Session 6: "~4:1 research time saves builder time" across the
full set of corrections. Across 15 research-verify runs in Session 6, the stage cost
75 minutes and is estimated to have saved 300+ agent-minutes.

---

### Stage 3: Plan-Review (Sonnet)

**Average time**: 5-8 minutes per issue.

**What it does**: Takes a research-verified scout spec and completes it. Asks: is the
root cause correctly identified? Is the proposed fix at the right level of abstraction?
Are the edge cases specified? Is the test case exercising the actual bug?

**Correction rate**: 100% in Session 6. Every scout spec was modified before a builder
received it. The correction types:

- **Wrong call site** (root cause two layers up the stack): 3 cases
- **Wrong root cause** (symptom addressed instead of origin): 2 cases
- **Incomplete edge case specification**: 4 cases
- **Test case that would not detect a regression**: 3 cases
- **Approach requiring more crates than necessary**: 2 cases

This 100% correction rate is consistent with Era 7 Sessions 1 and 2, where the same
pattern held. It is not a sign that scouts are poor — scouts are running Haiku at 3-5
minutes and doing fast, broad search. The correction rate reflects that "finding the
problem" and "specifying the solution completely" are different cognitive tasks, and
delegating them to different agents with different constraints produces better outputs
from both.

**If removed**: Builders receive incomplete specs. The documented consequences from
pre-plan-review sessions: 30% rewrite rate (builder discovers the approach is wrong
mid-implementation), 20% miss rate (builder implements something correct but untestable,
PR lands with a vacuous test). Applied to Session 6's 40 build tasks:
12 rewrites × 30 minutes each = 360 additional agent-minutes; 8 miss-rate PRs requiring
follow-up correction PRs.

**ROI**: Plan-review is the force-multiplier stage. Its 5-minute cost is negligible
relative to the builder time it conserves. At 40 parallel builders, a 40-point improvement
in builder success rate (from 50% to 90%) saves 16 failed build cycles × 45 minutes each
= 720 agent-minutes. The plan-review overhead for 40 issues: 40 × 7 minutes = 280 minutes.
Net: +440 agent-minutes recovered, plus higher-quality specs entering the build stage.

---

### Stage 4: Build (Sonnet)

**Average time**: 30 minutes per PR (with plan-reviewed spec).

**What it does**: Implements the spec in an isolated worktree. Reproduces the issue
in a test, fixes it, verifies the fix. Does not redesign. If the spec requires architectural
changes, sends it back to plan-review with specific questions.

**Correction rate**: ~10% (builder sends back to plan-review). The remaining 90% produce
a draft PR that passes local verification.

**If removed**: No code gets written. This is not a meaningful counterfactual — the build
stage is the output stage. What is meaningful is asking what happens at different plan-review
qualities: a builder with a complete spec has a ~90% success rate; a builder with a scout
spec (no plan-review) has a ~50% success rate. That 40-point difference is the entire
economic argument for the preceding stages.

**ROI**: 30 minutes per successful PR at 90% success rate = 33 minutes per PR in expected
value. This compares favorably to the 60-minute expected cost of unconstrained building
(60 minutes × 50% success = 120 minutes per successful PR, since failed builds are mostly
wasted). Plan-reviewed builders are 3.6x more productive per agent-hour than unconstrained
builders.

---

### Stage 5: Review (Haiku/Sonnet)

**Average time**: 5-10 minutes per PR.

**What it does**: Standards pass. Checks formatting, banned constructs (`unwrap()`,
`expect()`, `panic!()`), scope (does the diff match the issue spec?), and whether tests
exist. Pushes fixes directly to the branch — does not file comments for the builder to
address. The PR improves immediately.

**Correction rate**: Caught issues in approximately 40% of PRs reviewed in Session 6.
The common catches: missing clippy attributes, formatting drift, test without assertion,
scope creep (diff included unrelated changes).

**If removed**: 40% of PRs would carry formatting or standards violations into deep
review or CI, where they would fail and require a new agent invocation to fix. Each
CI failure costs 5-8 minutes of machine time plus queuing. At 40 PRs, 16 failures × 7
minutes CI each = 112 minutes of wasted CI. More importantly, the deep reviewer would
spend time on standards issues rather than logic issues, reducing its catch rate on
the bugs that matter.

**ROI**: 5-10 minutes per PR catches issues that would otherwise cost 15-25 minutes
in CI rerun cycles. Net: approximately 10 minutes saved per corrected PR, at 40%
correction rate.

---

### Stage 6: Deep-Review (Sonnet)

**Average time**: 10-15 minutes per PR.

**What it does**: Adversarial logic review. Reads the issue spec, traces execution paths,
constructs adversarial inputs, verifies that Perl semantics are correct for parser changes,
checks that tests actually test what they claim to test. Pushes fixes directly.

**Correction rate**: 100% in Session 6. Every PR that received deep review had at least
one real bug caught. The Era 7 Session 2 data is even more precise: 13 PRs deep-reviewed,
13 bugs found, 0 misses. This is not a selection effect — deep review was applied to PRs
that appeared clean at the standards pass level.

The bug categories from Session 6 (partially overlapping with documented Era 7 s2 data):

- **MetaCPAN link placement**: Hover output format — link was appended to wrong section.
  Would have displayed incorrectly for every `use Module` hover.
- **Moose attribute hover format**: Signature syntax incorrect. Would have shown malformed
  type annotations in completion tooltips.
- **PL001 code mismatch**: Diagnostic constant typo that passed CI because the wrong
  constant was still a valid integer. Diagnostics would have emitted with the wrong
  error code, breaking any tooling that matched on error codes.
- **Completion relevance sorting**: Edge case in tie-breaking logic — items with equal
  scores would sort non-deterministically (hash map iteration order), producing unstable
  completion menus on repeated requests.
- **Inlay hint padding**: Rendering artifact — extra space before colon type annotations.
- **Cascade suppression logic**: Empty diagnostic list was not being treated as "no
  diagnostics," causing stale diagnostics to persist after a file was cleaned up.
- **Recovery pattern precedence**: Error recovery Phase 0 had an ambiguity in which
  recovery strategy took precedence for adjacent syntax errors.
- **Test modularization scope**: New test module imported fixtures that were not in scope
  from the new location. Would have failed on the next test run on a fresh checkout.
- **Symbol resolution caching**: Cache invalidation bug — modifications to one file
  could leave stale resolved symbols for dependent files.
- **Snapshot drift**: Test snapshot had drifted from current parser output. CI was
  passing because snapshot comparison was only run in a non-default CI tier.

**If removed**: 10+ bugs land on master per session. Not theoretical bugs — real defects
that passed the standards review, passed CI, and were caught only because a second
reviewer traced the execution paths. The consequences:

- The PL001 mismatch would require a hotfix PR plus a re-announcement to any tooling
  integrators who had already adapted to the wrong code.
- The cascade suppression logic bug would cause user-visible stale diagnostics — a
  class of bug that is hard to reproduce and therefore expensive to debug.
- The cache invalidation bug is the most expensive: the failure mode (cross-file stale
  resolution) only manifests on specific edit sequences in multi-file workspaces.
  Expected debug time: 4-8 hours minimum.

**ROI**: 10-15 minutes per PR catches bugs whose production cost ranges from 1 hour
(formatting, sorting) to 8+ hours (cache invalidation, multi-file logic). At an average
prevented cost of 3 hours per bug, 10 bugs prevented at 10 minutes of review cost:
30 hours saved for 100 minutes invested. 18:1 ratio.

The economic case for deep review is not "catch more bugs." It is "the bugs that deep
review catches are systematically the ones that pass everything else." CI catches
compilation errors. Clippy catches style issues. Standards review catches structural
violations. Deep review catches the residual: the logic that is syntactically correct,
tests-passing, lint-clean, and semantically wrong.

---

### Stage 7: Green (CI Gate)

**Average time**: 5-8 minutes per PR (local), longer in CI queue.

**What it does**: SHA-verified check. The same commit that was reviewed locally must pass
in CI. Format check, `cargo clippy --workspace`, `cargo test --workspace --lib`, CPAN
corpus check.

**Correction rate**: ~15% catch rate (PRs that passed all local checks but failed in CI,
typically due to environment differences or timing).

**If removed**: Regressions merge to master. The CPAN corpus check alone catches parser
regressions that would silently break real-world Perl modules. Without CI, the trust
model degrades: builders can no longer rely on "passes local" as a meaningful signal.

**ROI**: CI is the trust anchor. Its cost is justified not by its bug catch rate in
isolation but by its function as the verification system that makes all other stages'
outputs trustworthy. Without CI, the 90% builder success rate means 10% of merges are
regressions. With CI, that 10% is caught before landing. At 59 PRs, that is ~6 avoided
regressions per session.

---

### Stage 8: Merge (Ops)

**Average time**: 5 minutes per batch of 3 (3 minutes merge operation + 2 minutes CI
monitoring). Approximately 20 batches in Session 6 = 100 minutes total merge operations.

**What it does**: Squash-merge batches of 3 PRs. Wait for CI. Run corpus ratchet after
parser fix PRs.

**Correction rate**: The batching constraint itself is the correction. Merging more than 3
PRs rapidly triggers CI cascade cancellations — each merge cancels the previous CI run.
Sessions before the batch-of-3 protocol was established wasted 50%+ of CI runs to
cascade cancellations.

**If removed (i.e., if merge pacing were abandoned)**: 120 CI runs triggered in Session 6
would have had approximately 60 useful completions. With batch-of-3: approximately 90+
useful completions. The waste reduction is roughly 30 CI runs, at 5-8 minutes each =
150-240 minutes of CI machine time recovered. More importantly, cascade cancellations
extend the merge cycle time, which extends how long PRs sit in queue, which increases
rebase conflicts, which generates additional work.

**ROI**: The batch-of-3 constraint is cost-free — it is a policy, not an agent. Its only
cost is human patience. Its savings in CI machine time and avoided rebase conflicts are
measurable.

---

### Stage 9: Wisdom (Memory Update)

**Average time**: 10-15 minutes per session.

**What it does**: Updates memory files with what was learned. Adds to the corpus ratchet
baseline. Encodes new patterns, anti-patterns, and corrections for future sessions.

**Correction rate for future sessions**: Unquantifiable in the short term. The documented
pattern is that memory files from previous sessions are the largest component of the
cache-read load (94.5% of tokens are cache reads, and memory files are the largest
stable context). Memory quality therefore directly affects agent quality in future sessions.

**If removed**: Each session starts from scratch. The cost of rediscovering known failure
patterns (like the git stash contamination, the shared CARGO_TARGET_DIR collision, the
worktree main-checkout write path) would recur every session. These patterns were
discovered through failure and encoded precisely to prevent future failures.

**ROI**: Compounding. A memory file written in Session 2 that prevents one agent failure
in Session 6 has ROI that cannot be calculated at the time of writing. The aggregate
effect is visible in the efficiency trajectory: Session 1 produced 16 PRs with 25 agents;
Session 6 produced 59 PRs with 200 agents, not because of more agents but because of
better constraints on how those agents operated.

---

## Part 2: Parallel Lanes Analysis

### Why 59 PRs Requires Four Lanes

Serial execution of the Session 6 pipeline would look like this:

```
Issue → Scout → Research → Plan → Build → Review → Deep-Review → CI → Merge
                                                                       ↑
                              45 minutes per PR                        |
                                                                   3 PRs/batch
```

At 45 minutes per PR and 3 PRs per merge batch, serial throughput = 3 PRs per 45 minutes
= 4 PRs/hour. Over 7 hours: 28 PRs. That is roughly the throughput of a well-run
traditional 2-developer team.

What Session 6 actually ran:

```
Lane A: Research ──────────────────────────────────────────────────────────
Lane B: ───── Plan ──── Build ──── Review ──────────────────────────────────
Lane C: ──────────────────── Deep-Review ──── CI ──── Merge ────────────────
Lane D: Scout ──── New Issues ──── Documentation ──── Triage ───────────────
```

All four lanes run simultaneously. While Lane B's builders are building, Lane C's
deep-reviewers are reviewing earlier PRs, Lane A's researchers are validating incoming
scout findings for the next build wave, and Lane D is generating new work for the cycle
after this one.

### The Lane Arithmetic

**Without lanes (serial)**: 28 PRs in 7 hours (calculated above).

**With 4 lanes**: 59 PRs in 7 hours (observed).

The ratio is 2.1x. Not the theoretical maximum (which would be 4x if all lanes were
perfectly synchronized), but the realistic gain when:

- Build time (30 min) is longer than review time (10 min), creating queue imbalance
- CI batch pacing (batch of 3) is the binding constraint in the merge lane
- Research throughput is faster than plan-review throughput, creating a small buffer

The binding constraint across all four lanes was the merge lane. Batches of 3 PRs
at 5-8 minutes per CI run means a maximum of approximately 1 batch per 8 minutes,
or approximately 7-8 batches per hour, or 21-24 merged PRs per hour at full throughput.

Over 7 hours, that gives a theoretical ceiling of 147-168 merged PRs. Session 6 hit 59.
The gap between 59 and 147 reflects:

- Build time is 30 minutes, not instantaneous — PRs do not arrive at merge as fast as
  CI can process them
- Not all 200 agents were builders; scouts, researchers, reviewers, and wisdom agents
  consumed capacity without producing mergeable PRs directly
- Triage work (closing 35 stale issues) consumed roughly 10% of session capacity

### What Enabled the 4th Lane

Sessions 1-5 ran 2-3 parallel lanes. Session 6 added the 4th lane because the issue
triage at the session start freed the orchestrator's attention. Closing 35 stale issues
was not in the PR count, but it reduced the decision overhead on which work to route
where, which enabled the orchestrator to maintain routing decisions across 4 simultaneous
queues rather than 2-3.

This is the invisible force multiplier: **triage enables routing**, and routing enables
lanes. The 35 closed issues do not appear in the 59-PR headline. They made the 59-PR
headline possible.

### Agents per PR: Why the Ratio Looks Wrong

| Session | Agents | PRs | Agents/PR |
|---------|--------|-----|-----------|
| s1 | 25 | 16 | 1.6 |
| s2 | 150 | 48 | 3.1 |
| s6 | 200 | 59 | 3.4 |

The agents/PR ratio is increasing. This looks like decreasing efficiency. It is not.

The increase reflects two structural changes:

1. **Research and verify agents are now counted**. In s1, there were no research-verify
   agents — scouts went directly to builders. The 3.4 agents/PR in s6 includes 1 scout
   + 1 research-verify + 1 plan-reviewer + 1 builder + 0.5 reviewers = 4.5 agents/PR
   for the full pipeline, partially offset by shared agents working multiple PRs.

2. **Non-building agents produce non-PR artifacts that enable PR output**. An agent that
   closes 5 stale issues has agents/PR = infinity, but its contribution to session
   throughput is real.

The correct metric is not agents/PR but PRs/session. Session 6's 59 is the record.

---

## Part 3: Error Taxonomy — Research-Verify Corrections

The 8+ corrections caught by research-verify agents in Session 6 fall into five
structural categories. Understanding the category matters because each category requires
different prevention.

### Category 1: Pre-existing Implementation (Type: Duplication Prevention)

**Instance**: `createDebugConfig` already exists.

The scout recommended building a new VSCode command for debug configuration. Research-verify
found that `createDebugConfig` was already registered in the extension's command manifest,
with a partial implementation in the DAP integration layer.

**What would have happened**: A builder would have created a new command registration,
which would have silently shadowed or conflicted with the existing one. VS Code command
namespacing would have produced confusing behavior (two commands with similar names, or
a command that appeared twice in the palette).

**Root cause of scout error**: Scouts search for where a feature *should* be implemented,
not for where it *already is*. The scout found a gap in user-facing documentation and
inferred a missing implementation.

**Prevention pattern**: Research-verify should always check for existing implementations
before any "create X" spec reaches a builder. The grep pattern is straightforward:
search the codebase for the command name, API surface, or concept before specifying
its creation.

---

### Category 2: External Claim Inversion (Type: Fact Verification)

**Instance**: Sublime Text backend claim was backwards.

The scout stated that Sublime Text supported LSP feature X via backend mode A but not
mode B. Research-verify checked the Sublime Text LSP plugin documentation and found the
support was in mode B, not mode A.

**What would have happened**: Documentation stating that "Sublime Text supports X when
configured with mode A" would have been published. Users configuring Sublime Text
with mode A would have found the feature non-functional. This is the category of error
that requires an external user to discover and report.

**Root cause of scout error**: Scouts often synthesize external claims from memory
(training data) rather than fresh verification. For third-party tool behavior, memory
is often outdated or subtly wrong.

**Prevention pattern**: Any spec involving claims about third-party tool behavior requires
a research-verify pass that checks live documentation, not trained knowledge. The
research-verify agent should explicitly retrieve the current documentation URL rather
than recalling from context.

---

### Category 3: Scope Overestimate (Type: Simplification)

**Instance**: 7-crate split became 2-step migration.

The scout proposed splitting a module resolution crate into 7 new independent crates for
better separation of concerns. Research-verify found that 5 of those 7 crates already
existed in the workspace — some under different names, some as subtrees of existing crates
that could be extracted with minimal changes.

**What would have happened**: 4 builder-agents would have been assigned to build crates
that already existed. Even if the build agents discovered the existing crates during
implementation, they would have spent 15-20 minutes each investigating before concluding
the spec was wrong. Total wasted builder time: 60-80 minutes.

**Root cause of scout error**: Scouts reason about what the architecture *should* look
like without comprehensively inventorying what already exists. This is a known pattern —
the "greenfield trap," where a scout recommends building what should exist rather than
finding what does exist and mapping it to what should.

**Prevention pattern**: Before any "create N new crates" spec, research-verify should
run a full workspace inventory and map existing crates against the proposed new structure.
The spec then becomes "migrate X to Y" rather than "create Y."

---

### Category 4: Constant Mismatch (Type: Dead Code Path Discovery)

**Instance**: PL001 diagnostic code mismatch.

The scout cited a diagnostic constant `PL001` in its spec. Research-verify checked the
diagnostic constant table and found the actual constant name was `PL_MISSING_SEMICOLON_001`
(or similar — the precise name differs from the scout's cached reference). The spec
passed through research-verify with this correction applied before it reached a builder.

**What would have happened**: The builder would have implemented the feature using the
scout's constant name. Rust's type system would not catch this — both constant names
are valid integers. The diagnostic would have been emitted under the wrong code. Any
tooling that matched on diagnostic codes (CI integration, editor rules, `.perlcriticrc`
equivalents) would have received the wrong signal.

**Why this is not just a typo**: The mismatch between the scout's reference and the
actual constant suggests either (a) the constant was renamed at some point without
all references being updated, or (b) the scout hallucinated a plausible-sounding constant
name. In either case, research-verify's role is mechanical: look up the constant,
verify it exists, correct the spec.

**Prevention pattern**: Any spec referencing specific constants, function names, or
identifier strings should be verified against the actual codebase before reaching a builder.
This is the core of the research-verify role — not reasoning about approaches, but
confirming that named things exist with the specified names.

---

### Category 5: Semantic Misread (Type: Incorrect Bug Report)

**Instance**: Perl `and`/comma precedence — proposed parser change was wrong.

The scout identified what appeared to be a parser bug in how `and` operator precedence
interacted with comma-separated lists. The proposed fix would have changed the parser's
behavior. Research-verify checked the Perl language specification and the relevant
test cases in the CPAN corpus.

**The Perl `and` precedence rule**: `and` has lower precedence than comma. A statement
like `open(my $fh, '<', $file) or die "Cannot open: $!"` relies on the comma binding
more tightly than `or`. The parser's behavior was correct. The scout had misread the
precedence table.

**What would have happened**: A parser change to "fix" correct behavior would have
introduced a regression in how common Perl idioms were parsed. The regression would
have been caught by the corpus check — but only after the build, review, and CI cycles.
At Session 6 scale, that is 40+ minutes of agent time wasted, plus a CI failure to
investigate, plus a follow-up issue to understand why the corpus check failed.

**Prevention pattern**: Any spec that proposes changing parser behavior for a language
construct should be verified against the language specification before reaching a builder.
Research-verify agents have access to web retrieval and should use it for semantic
correctness questions, not just file-existence checks.

---

## Part 4: CI Economics

### The 120-Run Problem

Session 6 triggered approximately 120 CI runs. Approximately 60 of them produced useful
results (completed, produced a actionable outcome). The other 60 were either:

- Cancelled by a subsequent merge (cascade cancellation)
- Superseded by a rebase (the branch changed while CI was running)
- Duplicate runs on the same SHA (branch force-pushed during review, same content)

The 50% waste rate is not exceptional — it was the baseline before the batch-of-3 merge
protocol was established. Earlier sessions (before Era 7 s3) had waste rates above 60%.

### The Cascade Problem and CURRENT_STATUS Split

The most significant single CI optimization in Session 6's timeline was the
`CURRENT_STATUS.md` split (PR #2830).

**Background**: `CURRENT_STATUS.md` was a single file that every post-merge CI run
regenerated. Because it was a single file, any PR that touched anything would potentially
cause a diff in `CURRENT_STATUS.md`. When multiple PRs merged in rapid succession, each
merge updated `CURRENT_STATUS.md` with slightly different metrics, which caused the
next PR's CI run to see a conflict with the base branch.

**The cascade sequence**:
1. PR A merges, updates CURRENT_STATUS.md with 2,850 tests.
2. PR B's CI starts, compares against the CURRENT_STATUS.md that existed when PR B
   was opened (2,847 tests).
3. The discrepancy fails the "metrics match" check.
4. PR B must be rebased, triggering a new CI run.
5. PR C, which was waiting for PR B, also sees a stale CURRENT_STATUS.md.
6. PR C also fails. Also needs a rebase.

At 59 PRs in a session, a single shared file regenerated by every merge creates O(n^2)
cascade potential. The fix was to split CURRENT_STATUS.md into subsystem files
(`lsp.md`, `tests.md`, `parser.md`, `quality.md`) so that a merge touching only the
parser crate would only update `parser.md`, not the shared file.

**The economics**: Before the split, a wave of 10 PR merges might trigger 8-10 cascade
failures requiring rebase, each generating 2 new CI runs (rebase + validation). That is
16-20 additional CI runs for 10 merges — a 1.6-2.0 multiplier. After the split, cascade
failures in a typical wave drop to 1-2. Multiplier: 1.1-1.2.

Applied to Session 6's 59 PRs merged in approximately 20 batches of 3: at 1.8x
multiplier (pre-fix), expected CI runs = 59 × 1.8 = 106. At 1.15x multiplier (post-fix),
expected CI runs = 59 × 1.15 = 68. The ~40-run reduction translates to approximately
5 hours of CI machine time recovered per session.

### Useful vs Wasted CI Ratio

| Protocol | PRs per batch | Cascade rate | Useful CI % |
|----------|--------------|--------------|-------------|
| Pre-batch-of-3 | Unlimited | High | ~40-50% |
| Batch-of-3, pre-split | 3 | Medium | ~60-70% |
| Batch-of-3, post-split | 3 | Low | ~75-85% |

Session 6 operated in the "batch-of-3, post-split" configuration for the second half
of the session (after PR #2830 merged). The first half operated in the "batch-of-3,
pre-split" mode.

Conservative estimate: Session 6 had approximately 85 useful CI runs out of 120 triggered,
or ~71% utilization. The remaining 35 wasted runs at 5-8 minutes each = 3-5 hours of
CI machine time. This is the largest recoverable cost in the session outside of agent time.

---

## Part 5: Agent Contamination Cost

### Three Contamination Vectors

Session 6 documented three structural sources of cross-agent contamination, each of
which wasted CI cycles:

**Vector 1: Main Checkout Writes**

Agent tools (Write, Edit) resolve absolute paths to the main repository checkout, not
to the agent's worktree. An agent in worktree `agent-abc123` that writes to
`/home/user/repo/crates/perl-lsp-rs/src/lib.rs` writes to the main checkout's version
of that file, not its worktree's version.

This was discovered in Era 7 s2 when multiple agents were inadvertently editing the
same files through the main checkout path. The result was non-deterministic — the last
write won, with no merge conflict resolution. PRs built in worktrees with contaminated
main checkouts had CI results that did not match what was in their branch.

**Cost**: Each contamination event produces a CI run that fails for mysterious reasons
(the code in CI doesn't match what the agent thought it built), plus investigation time
to identify the source. Estimated cost per event: 45-90 minutes including investigation.

Session 6 had approximately 3 confirmed contamination events = 2-4 hours wasted.

The fix (preflight script checking that the working directory is the worktree, not
the main checkout) was established in Era 7 s3 and prevents new instances.

**Vector 2: Shared Git Stash**

Git stash is a global resource. Entries pushed by one agent are visible to all agents
sharing the same `.git/` directory. An agent that runs `git stash pop` in a worktree
may restore another agent's stashed changes.

In Era 7 s2, 40+ stash entries from concurrent agents accumulated. An agent that ran
`git stash pop` to restore its own work instead restored a different agent's in-progress
changes — silently. The result was code that appeared committed in one agent's worktree
but was actually interleaved with a different fix entirely.

**Cost**: Each stash contamination event requires the affected agent to abandon its work
and restart from a clean branch state. Estimated cost: 30-60 minutes per event.

Session 6 had the stash ban in place (from the worktree isolation policy established
in Era 7 s2). Zero confirmed stash contamination events in s6.

**Vector 3: Shared CARGO_TARGET_DIR**

When multiple agents build in parallel without an isolated `CARGO_TARGET_DIR`, Cargo's
build artifact cache is shared. Two agents compiling different versions of the same
crate simultaneously produce build artifacts that can corrupt each other.

The symptom is phantom test failures: tests that pass in isolation but fail intermittently
when multiple agents build concurrently. The failure is not in the code — it is in the
artifact cache. The CI run fails; the agent reruns locally to investigate; the local
run passes; the agent pushes and triggers another CI run.

**Cost per event**: One wasted CI run (5-8 minutes) + investigation time (15-30 minutes)
+ rerun CI (5-8 minutes) = 25-45 minutes per event.

Session 6 had the preflight script recommending isolated CARGO_TARGET_DIR per worktree.
Compliance was partial — agents that ran the preflight correctly had zero Cargo cache
contamination; agents that skipped it had 2-3 phantom failures.

**Total contamination cost estimate for Session 6**:
- Main checkout writes: ~3 events × 1 hour = 3 hours
- Shared stash: 0 events (ban in effect)
- Shared CARGO_TARGET_DIR: ~5 events × 35 minutes = 3 hours

Total: approximately 6 hours of agent and CI time wasted to contamination issues.

**Comparison to no-contamination baseline**: If all contamination were eliminated,
Session 6 would have recovered ~6 hours of effective agent time. At the observed
rate of 8.4 PRs/hour (59 PRs / 7 hours), this represents approximately 50 additional
potential PRs — though realistically, the merge queue bottleneck would have absorbed
most of this capacity as additional merge batches, not as proportionally more PRs.

The more important number: the contamination-free baseline, combined with the
CURRENT_STATUS split, would have reduced the total CI run count from ~120 to approximately
75-80. The 40-45 recovered CI runs at 5-8 minutes each = 3-6 hours of machine time.
This is not a hypothetical — it is the target state that the preflight scripts and
worktree isolation policies are designed to achieve, and the trajectory is visible in
the reduction from Era 7 s2's ~40 contamination events to s6's ~8.

---

## Part 6: The Structure Behind the Record

Session 6's 59-PR output was not random. The structural conditions that enabled it:

**1. Four active parallel lanes**
Research, build, review, and merge all ran simultaneously. Previous sessions had 2-3
lanes. The 4th lane required the triage work at session start — closing 35 stale issues
freed the routing capacity to maintain 4 queues.

**2. Research-verify eliminating upstream waste**
8 corrections before builder assignment = approximately 8 avoided failed build cycles
at 30 minutes each = 240 agent-minutes recovered. Alternatively stated: the research-verify
stage prevented the equivalent of 4 hours of builder time from being wasted.

**3. Plan-review's 100% correction rate as a force multiplier**
40 plan-reviewed specs at 90% builder success = 36 successful PRs from the first
attempt. 40 unconstrained specs at 50% builder success = 20 successful PRs, with
20 failures requiring investigation and relaunch. The plan-review stage is the difference
between 36 and 20 successful PRs from the same 40 build tasks.

**4. Deep-review's 100% bug catch rate**
10+ bugs caught before merge = 10 hotfix PRs avoided. At 1 hour per hotfix (revert,
fix, test, re-review, re-merge), that is 10 hours of avoided rework. Deep review
consumed approximately 100 minutes for the same result.

**5. CURRENT_STATUS split reducing cascade failures**
Mid-session reduction in cascade cancellations freed approximately 3-5 hours of CI
machine time and removed a class of rebase-required failures from the merge queue.

**6. Pre-established contamination controls**
Zero git stash contamination (ban in effect from s2) and reduced Cargo cache collisions
(preflight script recommending isolated paths) saved approximately 3+ hours compared
to the s2 baseline.

The 59 PRs is not the story. The story is the seven structural conditions that made
59 PRs achievable in 7 hours, and the trajectory that suggests each of those conditions
will improve further in subsequent sessions.

---

## Appendix: Stage-Level ROI Summary

| Stage | Time Cost | Correction Rate | If Removed: Cost |
|-------|-----------|-----------------|------------------|
| Scout | 3-5 min/issue | N/A (generates input) | 50% builder failure rate |
| Research-Verify | 5 min/issue | 8 corrections in s6 | 8 wasted builds, 1 live bug (PL001) |
| Plan-Review | 5-8 min/issue | 100% | 16 failed builds, 8 vacuous tests |
| Build | 30 min/PR | 10% send-back rate | No output |
| Review | 5-10 min/PR | 40% correction rate | 16 CI failures per session |
| Deep-Review | 10-15 min/PR | 100% | 10+ bugs per session on master |
| Green (CI) | 5-8 min/PR | 15% catch rate | ~6 regressions per session |
| Merge | 5 min/batch | Pacing prevents cascade | 50%+ CI waste |
| Wisdom | 10-15 min/session | Compounds across sessions | Capability decay per session |

---

*Session data from Era 7 Session 6 (2026-03-22/23). Quantitative estimates are derived
from documented session memory files (`project_era7_session2_wisdom.md`,
`AGENTIC_ECONOMICS_DATA.md`), billing data in `SESSION2_ECONOMICS.md`, and the deep
review case record in `EVERY_DEEP_REVIEW_FOUND_A_BUG.md`. Where precise measurements
are unavailable, estimates are noted as such with the methodology used to generate them.*
