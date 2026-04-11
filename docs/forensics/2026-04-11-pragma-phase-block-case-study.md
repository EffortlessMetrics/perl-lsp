# PR #4090: The False-Premise Cascade

**Forensic case study of a near-miss merge — 2026-04-11**

- **Incident PR:** [#4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) — closed, never merged
- **Origin issue:** [#4084](https://github.com/EffortlessMetrics/perl-lsp/issues/4084)
- **Revert tracking:** [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100)
- **Corrected fix:** [#4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108) (merged)
- **Duplicate-dispatch:** [#4120](https://github.com/EffortlessMetrics/perl-lsp/pull/4120) (closed)
- **Follow-ups:** [#4101](https://github.com/EffortlessMetrics/perl-lsp/issues/4101) (positive-direction lint), [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111) (mandatory research-verifier), [#4117](https://github.com/EffortlessMetrics/perl-lsp/issues/4117) (wisdom retrospective)
- **Total elapsed:** scout findings -> duplicate closure = 2h 07m

---

## TL;DR

A plausible-sounding PR claimed `BEGIN { use strict; }` propagates strict file-wide "per perlmod/perlop," wired `PragmaTracker` to match, passed through scout, build, first-pass review, and deep review, got `reviewed-deep`, got `merge-ready`, and was cleared for the ops queue. A research-verifier dispatched in parallel ran `perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'`, got `ok`, and proved the premise wrong in a single invocation. The PR was stopped ~9 minutes before ops would have picked it up. The correct fix (revert + invert the tests) landed in [#4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108) at 11:39 UTC, ~35 minutes after the catch. This is a clean example of a **multi-reviewer shared blind spot** being broken by **reference-implementation verification** — the only class of check that could have caught it.

---

## Timeline

All timestamps UTC, 2026-04-11. Pulled from `gh issue view` and `gh pr view` directly.

| Time | Event | Evidence |
|------|-------|----------|
| 08:28 | PR [#4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052) filed — an independent pragma-tracker fix for `eval`/sub-scoped pragmas | `createdAt` |
| 09:42 | Issue [#4084](https://github.com/EffortlessMetrics/perl-lsp/issues/4084) filed — "lint_pipeline_strict_inside_begin/end/init failing on master, phase block pragma scope not modeled." Reported as pre-existing by the #4052 builder. | `createdAt` |
| 09:53 | Stale `needs-plan-review` stripped from #4084 — the first scout's worktree had been removed without posting findings. Relabeled `needs-investigation`. | [comment](https://github.com/EffortlessMetrics/perl-lsp/issues/4084#issuecomment-4229222937) |
| 09:55 | **Scout findings posted on #4084.** Hypothesis: `PragmaTracker::build_ranges()` has no arm for `NodeKind::PhaseBlock`, so it falls through to `_ => {}`. Claim: *"Per Perl semantics (perlmod, perlop): all phase blocks execute at compile time with respect to pragma state. Pragmas inside phase blocks propagate to the surrounding file scope — they are not lexically scoped like a regular subroutine body."* Builder-ready spec attached. | [comment](https://github.com/EffortlessMetrics/perl-lsp/issues/4084#issuecomment-4229224497) |
| 10:17 | PR #4090 commit authored | `authoredDate` |
| 10:19 | Orchestrator heads-up on #4084: PR #4052's rebase added a `walk_node` `PhaseBlock` body scan in `strict_warnings.rs` as a workaround for the same assumption. Builder warned to rebase fresh; warned the workaround may become redundant once the tracker fix lands. | [comment](https://github.com/EffortlessMetrics/perl-lsp/issues/4084#issuecomment-4229257793) |
| 10:21 | User re-checks #4084 locally, finds the three tests now passing on master. Closes as stale/non-reproducible. *This closure is later revealed to have been the #4052 workaround masking the bug at the diagnostics layer — not a real fix.* | [comment](https://github.com/EffortlessMetrics/perl-lsp/issues/4084#issuecomment-4229261346) |
| 10:22 | Issue #4084 closed. PR #4052 merged ~2 seconds later. | `closedAt`, `mergedAt` |
| 10:32 | PR #4090 commit committed | `committedDate` |
| 10:34 | **PR #4090 opened.** Title: *"fix(pragma): model BEGIN/END/INIT/CHECK/UNITCHECK phase blocks as compile-time scope (#4084)."* Body states: *"Per Perl semantics (perlmod, perlop), phase blocks are compile-time-transparent — pragmas inside propagate to the surrounding file scope rather than being lexically confined."* 6 BDD tests added, all asserting outward propagation. | [PR body](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) |
| 10:42 | User attempts to close #4090 from the comment field with a single-check reality test: *"BEGIN { use strict; my $inner = 1; } $outer = 1"* run under `perl -c`, reports `syntax OK`, concludes the patch would introduce a scoping regression. **This closure did not actually take effect — the PR remained open and continued through review.** | [comment](https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229286903) |
| 10:57 | **Deep review approves with `reviewed-deep`.** The deep reviewer specifically walks through: fallback arm for non-Block inner node, END/INIT/CHECK/UNITCHECK semantics ("verified against Perl's compile-time model"), nested phase blocks, range tracking correctness, and vacuous-assertion check. Adds 3 edge-case tests in commit `bf670306`. Concludes: *"All 9 phase-block tests pass. No logic bugs found. Low regression risk."* The deep reviewer held the same false belief as everyone else. | [comment](https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229305348) |
| 10:58 | `merge-ready` label applied with a signed label receipt bound to SHA `32447e5a`. PR is now in the ops queue. | [label receipt](https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229306132) |
| 11:01 | **Research-verifier posts finding.** Runs `perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'`. Output: `ok`. Reports: *"1 of 1 Perl claim verified as FALSE. The code fix rationale is based on an incorrect understanding of Perl's actual pragma scoping behavior."* Verifies all five phase types (BEGIN/END/INIT/CHECK/UNITCHECK) — pragmas propagate in none of them. | [comment](https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229309824) |
| 11:04 | User posts "closing this" comment — this one sticks. | [comment](https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229314480) |
| 11:04 | PR #4090 closed. | `closedAt` |
| 11:07 | **Orchestrator hold posted.** `merge-ready` stripped, `needs-investigation` applied. Re-runs the `perl -e` check independently (second independent verification). Posts three design options: (A) match Perl semantics, (B) intentional LSP divergence, (C) middle ground. CC's product decision to the user. | [comment](https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229316923) |
| 11:09 | Issue [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) filed — *"pragma: revert false phase-block pragma propagation in PragmaTracker and strict_warnings."* Lists the full cascade: close #4090, revert `walk_node` workaround from already-merged #4052, rewrite 6 BDD tests with inverted assertions, rewrite 3 integration tests with inverted assertions, update `PragmaTracker` doc comment. | `createdAt` |
| 11:14 | Issue [#4101](https://github.com/EffortlessMetrics/perl-lsp/issues/4101) filed — *"feat(diagnostics): PL5xx lint for BEGIN {use strict} misconception."* The positive-direction alternative: a new lint that actively warns when a user writes `BEGIN { use strict; }` instead of silently hiding the bug. | `createdAt` |
| 11:15 | **Rejected-alternative comment on #4100.** Option B (intentional LSP divergence) captured and explicitly rejected for posterity — paternalistic, trains bad habits, splits the truth source, and the teaching lint is the better answer. Documents *when to revisit* (if usage data ever shows the teaching lint alone isn't enough). | [comment](https://github.com/EffortlessMetrics/perl-lsp/issues/4100#issuecomment-4229326403) |
| 11:29 | PR [#4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108) commit authored by the Codex swarm (OpenAI Codex, `codex@openai.com`). | `authoredDate` |
| 11:30 | PR #4108 opened. Reverts the `walk_node` PhaseBlock body scan, keeps `PragmaTracker` matching Perl lexical scoping, rewrites the failing integration tests with inverted assertions. | `createdAt` |
| 11:38 | Proactive audit posted on #4100: scans 31 Perl semantic claims across 7 subsystems (`perl-pragma`, `scope_analyzer`, `hover`, `strict_warnings`). 30 of 31 verified true; 1 known false (already tracked here). No additional #4090-class time bombs found. | [comment](https://github.com/EffortlessMetrics/perl-lsp/issues/4100#issuecomment-4229354725) |
| 11:39 | **PR #4108 merged** to master as `da857f11`. | `mergedAt` |
| 11:42 | Issue #4100 closed as "Fixed by #4108." | `closedAt` |
| 11:47 | Issue [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111) filed — *"process: make research-verifier mandatory for PRs citing external semantics."* The systemic fix: amend `reviewer-deep-decide` checklist to require research-verifier dispatch whenever a PR body cites perlmod/perlop/LSP spec/DAP spec/docs.rs. | `createdAt` |
| 11:49 | Issue [#4117](https://github.com/EffortlessMetrics/perl-lsp/issues/4117) filed — session-level wisdom retrospective. This incident is Pattern 3 ("Multiple reviewers sharing the same blind spot") and Pattern 7 ("Research-verifier is the highest-ROI agent type") in that retrospective. | `createdAt` |
| 11:55 | PR [#4120](https://github.com/EffortlessMetrics/perl-lsp/pull/4120) opened by a Claude Code swarm builder — **functionally equivalent fix** to #4108, dispatched in parallel without knowing #4108 existed. | `createdAt` |
| 12:02 | PR #4120 closed as duplicate. Closing comment: *"Superseded by #4108, merged ~19 minutes before this PR was filed. Cross-swarm coordination failure — both the Claude Code swarm (this PR) and the Codex device swarm (#4108) independently caught the #4090 false-premise cascade and dispatched builders to fix it."* | [comment](https://github.com/EffortlessMetrics/perl-lsp/pull/4120#issuecomment-4229385683) |

### Timeline geometry

- **The false claim was in the codebase for:** ~1h 11m (PR #4090 opened 10:34 -> #4108 merged 11:39)
- **The false claim had `merge-ready` for:** ~9 minutes (10:58 -> 11:07 hold)
- **From catch to corrected merge:** 38 minutes (11:01 research-verifier finding -> 11:39 #4108 merged)
- **Independent duplicate-dispatch gap:** 25 minutes (11:30 #4108 open -> 11:55 #4120 open)
- **Time the `walk_node` workaround lived in master before being reverted:** ~1h 17m (10:22 #4052 merge -> 11:39 #4108 merge)

---

## The `walk_node` companion: why this almost looked self-consistent

The reason #4090 was so convincing to multiple reviewers is that it did not arrive alone. PR [#4052](https://github.com/EffortlessMetrics/perl-lsp/pull/4052), merged **exactly two seconds** after #4084 was closed as stale (10:22:00 -> 10:22:02), had quietly added a `NodeKind::PhaseBlock` body-scan arm to the `walk_node` closure in `crates/perl-lsp-diagnostics/src/lints/strict_warnings.rs`. That arm set `has_strict = true` / `has_warnings = true` whenever it saw `use strict` or `use warnings` inside a phase block body, suppressing PL100/PL101 at the diagnostic layer.

The #4052 builder flagged this as a targeted workaround for the three failing integration tests and explicitly noted that the workaround should become redundant once a deeper fix landed in `PragmaTracker` itself. That "deeper fix" became the scout spec for #4084, which became PR #4090.

The consequence: by the time #4090 opened, master had already been edited to paper over the symptom at one layer in exactly the direction #4090 was about to edit a *second* layer. Anyone reviewing #4090 against master would see:

1. The three `lint_pipeline_strict_inside_*` integration tests **passing** on master (because #4052's walker workaround caught them).
2. The #4090 diff adding a `PragmaTracker` arm that, per the claim, modeled the "true" Perl semantics underneath the workaround.
3. The existing tests staying green after #4090's fix (because the diagnostic layer no longer *needed* the workaround — but still had it, producing a belt-and-suspenders false positive).
4. No signal anywhere that the belt and the suspenders were both wrong.

This is the **structural reason** the shared blind spot survived two rounds of review: two independent patches agreeing with each other created the appearance of independent corroboration when in reality they were both downstream of the same misconception that flowed from the original #4084 framing. The scout, the #4052 builder (on rebase), and the #4090 builder were all drawing on the same wrong textbook answer. The tests "confirming" the fix were written with that same answer baked in as the oracle.

This is a more general pattern worth naming: **when two layers of a system are edited in the same wave to both "handle" the same edge case, the edits can corroborate each other even when both are wrong.** The countermeasure is not more internal review — it's external-truth verification against something neither edit can influence.

---

## The literal check that caught it

This is the teachable moment — the entire cascade hinged on one shell invocation:

```bash
$ perl -e 'BEGIN { use strict; } $x = 1; print "ok: strict not active\n"'
ok: strict not active

$ perl -e 'use strict; $x = 1; print "ok\n"'
Global symbol "$x" requires explicit package name (did you forget to declare "my $x"?) at -e line 1.
```

The first invocation **runs successfully**. If `BEGIN { use strict; }` propagated strict to file scope, the bare `$x = 1` would have tripped the "requires explicit package name" error the second invocation shows. It didn't. Therefore strict did not propagate. Therefore the premise was false. Therefore the patch was wrong-direction.

The research-verifier ran this, the orchestrator re-ran it independently at 11:07, and the final reviewer on #4120 ran it a third time before closing as duplicate. **Three independent invocations, same result.** This is unusually high confidence in the corrected semantics — the kind you rarely get on a language-behavior question resolved in under an hour.

### Anatomy of the four checks performed on this PR

It is worth enumerating every "verification" that touched #4090 during its 1h 11m lifetime, because they are not all equivalent, and the differences are the whole lesson:

| # | Checker | Tool | Experiment | Diagnostic? | Result |
|---|---------|------|-----------|-------------|--------|
| 1 | Scout | Mental model + perlmod/perlop | "Per Perl semantics, phase blocks are compile-time-transparent" | No (asserts, doesn't test) | False claim propagated |
| 2 | #4090 builder | `cargo test -p perl-pragma` | 6 BDD tests asserting outward propagation; written from the scout spec | No (oracle is the spec, spec is the claim) | Green — tests pass because they assert what the code does |
| 3 | First closure attempt (10:42) | `perl -c` | `BEGIN { use strict; my $inner = 1; } $outer = 1` | No — `perl -c` parses but doesn't execute; strict-vars error needs runtime | `syntax OK`, directionally right conclusion but evidence too weak to be load-bearing |
| 4 | Deep reviewer | Mental model + trace through implementation | "Traced through the implementation: outer PhaseBlock arm iterates its block's statements, finds inner PhaseBlock, recurses..." | No — verifies internal consistency, not external truth | Approved |
| 5 | Research-verifier (11:01) | `perl -e` | `perl -e 'BEGIN { use strict; } $x = 1; print "ok\n"'` — actually executes | **Yes** — runs to completion under strict or it doesn't, no ambiguity | Caught the false claim |

The critical column is "diagnostic." Checks 1, 2, and 4 all operated on a mental model or on artifacts derived from a mental model. Check 3 reached for the runtime but used an experiment that couldn't discriminate between the hypotheses. Only check 5 invoked the runtime *with an experiment whose output would differ* between "strict propagates" and "strict is block-local." That's the single bit of evidence the whole incident turned on.

If you take one thing from this case study, take this: **a test is only worth something if its output would be different under the wrong answer.** Six green BDD tests whose assertions match the false premise are not evidence. One `perl -e` invocation that would error under the false premise *is* evidence. Count diagnostic power, not check count.

---

## What the case teaches

### 1. Multi-reviewer shared blind spots are real

Count the people who read this PR and held the same false belief:

1. The **#4084 filer** who framed the problem as "phase block pragma scope not modeled" (implying the propagation should happen).
2. The **scout** for #4084 whose findings posted at 09:55 confidently stated the "per perlmod/perlop" claim.
3. The **orchestrator** who relayed the scout findings and posted a heads-up on #4084 about the #4052 workaround, building on the same assumption.
4. The **#4090 builder** who implemented the match arm and wrote 6 BDD tests whose assertions were all in the wrong direction.
5. The **first-pass reviewer** who approved the diff.
6. The **deep reviewer** who specifically wrote *"END/INIT/CHECK/UNITCHECK pragma semantics... verified against Perl's compile-time model. All five phase block types are compiled in the same scope as the enclosing file."* and added 3 more edge-case tests reinforcing the false invariant.
7. The **first closure attempt at 10:42** that used `perl -c` (compile-check only) and got `syntax OK`. `perl -c` does not run the script, so a lexical scoping test where the error happens at runtime under strict would slip through. *This is the most interesting slip in the whole incident* — the first closure attempt was directionally right but used a check too weak to confirm it, and the PR continued through review as a result.

That is **seven independent opportunities** to catch the error, by at least four different humans/agents, and none of them used the one check that actually works: running real Perl on a test that would fail if the premise held. None of them were junior. The deep reviewer in particular was explicit and thorough about verifying against Perl's compile-time model — but "verifying against a mental model" and "verifying against the runtime" are not the same thing, and in this case the mental model was shared and wrong.

**Multiple independent reviewers failing to catch the same error is not a review-quality problem.** It is a shared-knowledge-blind-spot problem. More eyes don't help when all the eyes were trained on the same incorrect textbook answer. The only fix is a *different class of verification*.

### 2. Reference-implementation verification is the differentiator

Reading perlmod / perlop / Stack Overflow answers would NOT have caught this. The docs are ambiguous ("BEGIN blocks run at compile time" is true and creates the confusion), and common online answers reflect exactly the same misconception the reviewers held. I pulled several after-the-fact, and the "BEGIN { use strict; }" question is a frequent beginner trap specifically because the intuition — "the BEGIN block runs first, so its pragmas get set up before the rest of the file" — is plausible, elegant, and wrong.

The only verification that worked was **running Perl itself**. That is the essence of reference-implementation verification: *when the claim is about how a runtime behaves, the runtime is the only authoritative source.* Documentation describes the runtime. Stack Overflow describes what someone believes the runtime does. Memory describes what you learned once. None of those are the runtime.

Note also the difference between the first (failed) closure attempt at 10:42 using `perl -c` and the successful research-verifier check at 11:01 using `perl -e`. Both invoked Perl. Only one was the right test:

- `perl -c` parses and compiles but doesn't execute. A lexical strict check needs actual execution to see whether the bare assignment trips at runtime. `syntax OK` was not sufficient evidence.
- `perl -e 'BEGIN { use strict; } $x = 1; ...'` actually executes. The `$x = 1` either trips strict or it doesn't. It didn't. Conclusive.

**Reference-implementation verification isn't just "invoke the reference implementation." It's "invoke it on a test that would produce different output under each competing hypothesis."** The 10:42 check invoked Perl but with the wrong experiment. That's a useful sub-lesson: even people who reach for the runtime can still get the wrong answer if the experiment isn't diagnostic.

### 3. The near-miss would have cascaded

This is the counterfactual cost of the catch. If PR #4090 had merged:

- **The `PragmaTracker` false behavior would have reinforced the #4052 `walk_node` workaround** that was *already merged* 12 minutes before #4090 was filed. The two together would have made the false premise look doubly-confirmed — one layer of the stack agreeing with another. Any future reviewer questioning either layer would have been pointed at "but the other layer does the same thing."
- **Diagnostic scorecard tests in-flight (#4065 family)** would have been written with the wrong expected output. Once a scorecard is written with a bad expected value, every future fix that corrects the behavior fails the scorecard, and the common response is "regression — revert" rather than "the scorecard was wrong." That's a second-order trap that can live for months.
- **Issue #4101** (the positive-direction lint that warns users their `BEGIN { use strict; }` doesn't do what they think) **would never have been filed.** The bug would have been papered over by the LSP silently. Users would keep writing the pattern, the editor would keep accepting it, and the disconnect between LSP behavior and real Perl behavior would only surface when they ran their scripts.
- **The proactive audit at 11:38** that found 30/31 Perl semantic claims in the codebase to be sound would never have been triggered. The audit was prompted by "we just got burned on #4090, let's check the rest of the codebase before another one lands" — a direct consequence of this catch.

Conservative estimate of the revert cost if it had landed: 2 PRs to revert (#4090 itself + a follow-up for the `walk_node` change), 9 tests to invert, doc comment to rewrite, scorecard expectations to reconcile. **4-8 hours of elapsed cross-PR untangling** across at least 2-3 builder slots, plus whatever contributor confusion piled up in the meantime.

Cost of the catch: **one research-verifier dispatch, ~30 minutes.**

ROI: **8-16x minimum on that single verification pass**, and that's before counting the downstream benefits of #4101, the proactive audit, and #4111 (the process fix).

### 4. Options preserve user agency on real product calls

When the orchestrator posted the hold comment at 11:07, it presented **three options** instead of just saying "revert":

> **Option A — match Perl semantics (correct, scope-preserving)**: revert #4090, revert the #4052 workaround, rewrite all tests with inverted assertions.
>
> **Option B — intentional LSP divergence (friendly, documented)**: keep the behavior but document it as a deliberate design choice, and add a separate lint explaining the Perl reality so the LSP isn't lying silently.
>
> **Option C — the middle ground**: keep the diagnostic-layer workaround (scoped to the strict/warnings lint) because it targets a specific common misconception, but revert the pragma-tracker change because the tracker is a lower-level primitive that should match Perl semantics.

This matters because **Option A was right for this codebase** (honest semantics, teaching lint as the user-friendly answer), but the other options were not strawmen:

- **Option B was legitimate** for a team whose product values leaned more toward "fewer false-positive warnings on common patterns, even at the cost of divergence from the runtime." A different team with different users could have reasonably picked it.
- **Option C was a real fallback** if the revert turned out to break too much too fast. It would have kept users from regressing on the PL100/PL101 diagnostic experience while still fixing the deeper primitive.

Presenting options instead of dictating preserves agency on product calls that have real tradeoffs. The user picked Option A, and the 11:15 rejected-alternative comment on #4100 explicitly captures why Option B was rejected — so if the decision is ever revisited, the reasoning is on the record.

### 5. Positive-direction alternatives beat negative patches

The final resolution wasn't "revert the bad code and move on." It was:

1. **Revert the false-premise fix** — #4108 removes the `walk_node` workaround from #4052, restores `PragmaTracker` to correct Perl lexical scoping, and inverts the 9 tests.
2. **File #4101** — a new lint (PL5xx) that actively fires when a user writes `BEGIN { use strict; }` without a top-level declaration. The message explains that phase blocks are lexically scoped and suggests the fix; a code action quick-fix moves the pragma to file scope automatically.

The second step is the better long-term answer. Users who write `BEGIN { use strict; }` genuinely do mean for strict to apply file-wide — they're not confused about wanting strict, they're confused about how to get it. A revert alone leaves that frustration in place and hides it behind a silent missing-strict warning. A teaching lint names the specific misconception and gives them the fix.

**The LSP becomes a teacher instead of a white-lie generator.** That's the correct philosophy for a language server whose whole job is to surface language behavior: honesty about the runtime, with pedagogy layered on top to explain the confusing parts.

### 6. What the revert actually looked like in code

The corrected fix in [#4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108) is small, which is itself a lesson. Full diff summary from the PR description and the commit:

- **`crates/perl-lsp-diagnostics/src/lints/strict_warnings.rs`** — removes a **19-line** `NodeKind::PhaseBlock { block, .. }` arm from the `walk_node` closure that was added in PR #4052's rebase. That arm unconditionally iterated the phase block body setting `has_strict` / `has_warnings` flags. PR #4108 also explicitly keeps the other #4052 corrections (the `state_for_offset(usize::MAX)` fix and the removal of redundant per-module arms) because those were independently correct and not the target of the revert.
- **`crates/perl-pragma/src/lib.rs`** — adds a doc comment to `PragmaTracker::build_ranges()` explicitly calling out that `NodeKind::PhaseBlock` is *intentionally* not walked, citing the Perl 5.38.2 verification. Note what this doesn't do: it does *not* add a `NodeKind::PhaseBlock` match arm in any direction. The pre-#4090 `_ => {}` fall-through was already correct; the fix is to document why, not to add code.
- **`crates/perl-pragma/tests/behavior_spec_tests.rs`** — adds 6 new BDD tests with inverted assertions:

    - `given_begin_block_with_use_strict_when_querying_after_block_then_strict_is_not_active`
    - `given_end_block_with_use_strict_when_querying_after_block_then_strict_is_not_active`
    - `given_init_block_with_use_strict_when_querying_after_block_then_strict_is_not_active`
    - `given_check_block_with_use_strict_when_querying_after_block_then_strict_is_not_active`
    - `given_unitcheck_block_with_use_strict_when_querying_after_block_then_strict_is_not_active`
    - `given_begin_block_with_use_warnings_when_querying_after_block_then_warnings_is_not_active`

    The test names carry the invariant in the final clause: `strict_is_not_active`. If anyone ever "fixes" these tests by stripping the `_not_` in a future refactor, the `PragmaTracker` invariant is silently inverted again. The naming is load-bearing.

- **`crates/perl-lsp-diagnostics/tests/lint_pipeline_integration_tests.rs`** — rewrites the 3 integration tests that were the *origin* of the whole chain (the "failing on master" tests that triggered #4084):

    - `lint_pipeline_strict_inside_begin_reports_missing_strict` — asserts PL100 **fires** on `BEGIN { use strict; } $x = 1`
    - `lint_pipeline_warnings_inside_end_reports_missing_warnings` — asserts PL101 **fires**
    - `lint_pipeline_strict_inside_init_reports_missing_strict` — asserts PL100 **fires**

    These are now aligned with real Perl behavior: if the only `use strict` is inside a phase block, the file is effectively unprotected, and the LSP should say so.

The total diff is under 100 lines and the semantic change is "revert to the previous behavior that was already correct." The fact that the fix is *this small* relative to the effort it took to reach it is itself the teachable moment. Most of the work was untangling what the false premise had produced, not writing correct code. If the false premise had been caught at 09:55 instead of 11:01, the entire intervening 66 minutes of PR work, review, and test authoring would have been avoided.

### 7. The sub-lesson: failed-closure attempts are signal

The 10:42 closure attempt deserves its own note. The user ran *a Perl check*, reached the *correct directional conclusion* ("this patch would introduce a scoping regression"), posted a comment saying "closing this"... and the PR kept going through review. Why?

- The closure attempt used `perl -c` not `perl -e`, so the evidence was weaker than it looked.
- The comment was a prose summary with the check buried in the middle. The `reviewed-deep` flow that continued ~15 minutes later didn't treat the comment as a blocker.
- The close action didn't actually take effect in GitHub's state — the PR remained open. This could be a UI race, a label-collision, or a tooling bug; the evidence isn't conclusive. But the PR went on to get `merge-ready` 16 minutes after the "closing" comment posted.

The sub-lesson for the pipeline: **a closure attempt that doesn't take effect is a near-miss by itself**, and the pipeline should surface it. If someone posts "closing this" and the PR continues to get labels applied, something is wrong with either the tooling or the human-machine handshake, and a future incident could go the other way.

This was flagged in the session wisdom retrospective ([#4117](https://github.com/EffortlessMetrics/perl-lsp/issues/4117)) but is worth separating as its own concern — it's not the same failure mode as the shared blind spot, and it almost let the cascade through *anyway*, even though a human had already done the right check and reached the right conclusion.

---

## What got filed as follow-up

| Artifact | Purpose | Status |
|----------|---------|--------|
| [PR #4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108) | The actual revert + test inversion | Merged 11:39 |
| [PR #4120](https://github.com/EffortlessMetrics/perl-lsp/pull/4120) | Parallel duplicate from the Claude Code swarm | Closed as dup 12:02 |
| [Issue #4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) | Revert tracking with full cascade spec | Closed "Fixed by #4108" |
| [Issue #4101](https://github.com/EffortlessMetrics/perl-lsp/issues/4101) | PL5xx lint for `BEGIN { use strict; }` misconception (positive-direction alternative) | Open, `plan-reviewed`, `builder-ready` |
| [Issue #4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111) | Process fix: mandatory research-verifier dispatch when a PR body cites external semantics (perlmod/perlop/LSP spec/DAP spec/docs.rs) | Open |
| [Issue #4117](https://github.com/EffortlessMetrics/perl-lsp/issues/4117) | Session wisdom retrospective — 7 patterns including this incident as Pattern 3 ("multi-reviewer shared blind spots") and Pattern 7 ("research-verifier is the highest-ROI agent type") | Open |
| Proactive audit | Comment on #4100 auditing 31 Perl semantic claims across 7 subsystems — 30/31 true, 1 known false (this one). No additional #4090-class time bombs found. | [Posted](https://github.com/EffortlessMetrics/perl-lsp/issues/4100#issuecomment-4229354725) |
| Rejected-alternative capture | Comment on #4100 documenting Option B (intentional LSP divergence) and *why* it was rejected, with explicit "when to revisit" criteria | [Posted](https://github.com/EffortlessMetrics/perl-lsp/issues/4100#issuecomment-4229326403) |
| CLAUDE.md pipeline-table clarification | Docs-only PRs can reach `merge-ready` without `reviewed-deep` — added because the revert PR was docs-adjacent and this distinction needed to be stated explicitly | In session commits |
| This forensic | Single-incident deep narrative; paired with but distinct from #4117 | You are reading it |

The full follow-up list — two code artifacts, three tracking issues, a proactive audit, a rejected-alternative capture, a pipeline clarification, and a forensic case study — came out of *one ~30-minute research-verifier dispatch*. This is what "high-leverage verification" looks like when the downstream ripple is counted.

---

## Takeaways for contributors

Concrete, actionable distillation of the case for anyone who will write, review, or route work in this codebase:

**If you are a scout filing an issue that cites external behavior:**

- If your finding names perlmod/perlop/perlfunc/perlsyn/perlvar (or an LSP or DAP spec section), **run a minimal reference-implementation experiment and paste the output in the issue**. The experiment should be diagnostic — run both the "expected" and "unexpected" cases so the output visibly discriminates between them.
- "Per perlmod" or "per the spec" without a literal invocation is not evidence. It is a claim. Claims propagate through pipelines; evidence anchors them.

**If you are a builder implementing a spec that hinges on external behavior:**

- Your first failing test should exercise the external claim directly, not the internal scaffolding around it. If the spec says "BEGIN propagates strict," your first test should be one that *would fail if strict didn't propagate*, run against something that can independently verify — ideally real Perl, at least a carefully chosen property of the AST that's downstream of the runtime fact.
- If you can't find an experiment whose output would differ under the false hypothesis, your tests are assertions not proofs. Slow down and find one.

**If you are a reviewer (first-pass or deep) on a PR that cites external semantics:**

- Check `reviewer-deep-decide` for the new external-claim checkpoint once [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111) lands. Until then, use it manually: *"Does this PR body cite perlmod, perlop, perlfunc, perlref, perlsyn, perlvar, the LSP spec, the DAP spec, or a docs.rs page? If yes, has research-verifier run on it?"*
- Do not let internal consistency substitute for external truth. Internal consistency is necessary but not sufficient — especially when two layers of a system were both edited in the same wave to handle the same edge case.
- If a closure attempt was made and didn't stick, treat that as a blocker to investigate, not noise to move past.

**If you are the orchestrator routing work:**

- Present options on genuine product calls instead of dictating. Even when you're confident Option A is right, Option B and C on the record give future-you the ability to revisit the decision legibly.
- Dispatch research-verifier *in parallel* with the build, not only after the PR arrives. On #4090, the research-verifier was running concurrently with the deep review — that's the only reason the catch arrived in time. If it had been gated on "after reviewed-deep, before merge-ready," the ops queue might have picked the PR up first.

**If you are writing the PR body:**

- If your justification leans on "per perlmod" or "per the LSP spec," include the literal reference-implementation output that shows the claim holding. One-shot command, output verbatim, no summary. That way a reviewer can verify in 10 seconds without leaving the PR.

---

## What this case is not

- **Not the session wisdom retrospective.** [#4117](https://github.com/EffortlessMetrics/perl-lsp/issues/4117) is the project-level meta with 7 patterns across the whole 2026-04-11 wave. This doc is deeper on one of those patterns.
- **Not a swarm-ops article.** It's not about orchestration mechanics, agent coordination, or parallel dispatch. The cross-swarm duplicate dispatch between #4108 and #4120 is a footnote here, not the story.
- **Not a session retro.** Session retros cover everything that happened; this covers one PR from filing to revert.
- **Not a process change proposal.** [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111) is that proposal. This doc is the motivating case study that makes the proposal legible.

---

## Cross-references

**Primary evidence:**

- PR #4090 full comment thread: https://github.com/EffortlessMetrics/perl-lsp/pull/4090
- Research-verifier finding (the actual catch): https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229309824
- Orchestrator hold + three-options comment: https://github.com/EffortlessMetrics/perl-lsp/pull/4090#issuecomment-4229316923

**Related in the incident:**

- Issue #4084 — origin: https://github.com/EffortlessMetrics/perl-lsp/issues/4084
- Issue #4100 — revert tracking: https://github.com/EffortlessMetrics/perl-lsp/issues/4100
- Issue #4101 — positive-direction lint: https://github.com/EffortlessMetrics/perl-lsp/issues/4101
- Issue #4111 — process fix: https://github.com/EffortlessMetrics/perl-lsp/issues/4111
- Issue #4117 — session wisdom retrospective: https://github.com/EffortlessMetrics/perl-lsp/issues/4117
- PR #4052 — source of the companion `walk_node` workaround: https://github.com/EffortlessMetrics/perl-lsp/pull/4052
- PR #4108 — the corrected fix (merged): https://github.com/EffortlessMetrics/perl-lsp/pull/4108
- PR #4120 — the duplicate (closed): https://github.com/EffortlessMetrics/perl-lsp/pull/4120

**Memory notes this case reinforces:**

- `feedback_deep_review_roi.md` — two-pass review ROI; this incident is the external-truth equivalent where the two-pass review was not enough
- `feedback_verify_before_build.md` — 42% of builders found work already done; related but distinct pattern

---

*Filed 2026-04-11 as part of the post-incident learning capture. Paired with but distinct from the session wisdom retrospective in #4117.*
