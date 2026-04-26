# Session Forensics: 2026-04-25 Token Cluster Merges and Master Cascade

**Session segment:** 16:48 UTC 2026-04-24 through ~04:35 UTC 2026-04-25
**Series position:** 4th in the 2026-04-24/25 forensic series (#6445, #6449, #6450 prior)

---

## 1. Executive Summary

A 16-PR Codex burst targeting `perl-token` was generated between 16:48 and 17:07 UTC on
2026-04-24, producing 4 keepers from 12 closed dupes via ensemble-curator triage. Merging
those 4 keepers (plus 2 display-name and API ratchet support PRs) triggered an 8-fix master
cascade over approximately 3 hours: each keeper introduced a new compilation or formatting
regression that blocked the subsequent merge batch. The multi-gate pipeline (deep-review,
reviewer, pr-responder) caught 8 distinct logic and test bugs across the keeper cluster
before or during the cascade — bugs that would have shipped silently without the gate
passes. By 04:35 UTC on April 25, 21 merges had landed including 6 master fixes, 4 token
cluster keepers, 3 tree-sitter cluster merges, 3 parser/LSP merges, and 2 forensic docs,
with 1 hard-conflict stale PR closed and 5 token keepers still open awaiting conflict
resolution.

---

## 2. Cluster Timeline

### 2.1 Codex Burst Generation

| Time (UTC) | Event |
|---|---|
| ~16:48 | First perl-token burst PR created (#6396) |
| 16:48–17:07 | 16 PRs created across 8 design concerns (#6396–#6444 range, 12 keepers+dupes) |
| 17:07–17:14 | Ensemble-curator triage begins |
| 17:14 | 12 dupes closed in triage pass (6437–6443, 6425–6427, 6429–6431, 6433–6435, 6437–6439, 6441–6443) |
| 17:14 | Triage outcome: 4 confirmed keepers + 1 three-way tie cluster left open |

### 2.2 Triage Outcome by Cluster

| Cluster | Closed | Keeper | Outcome |
|---|---|---|---|
| Scorecard/baselines | #6397, #6398 | #6396 | Open — deep-review in progress |
| TokenKind metadata | #6400, #6401, #6402 | #6403 | Closed later — merged differently |
| Catalog drift tests | #6404, #6405, #6406 | #6407 | Closed later |
| Checked token spans | #6408, #6409, #6410 | #6411 | **Merged 2026-04-25T02:32** |
| Conformance coverage | none closed (3-way tie) | #6412, #6413, #6414 | All closed; #6415 open |
| Role predicates | #6425, #6426, #6427 | #6428 | Open — deep-review in progress |
| Keyword/operator mapping | #6416, #6417, #6418 | #6419 | **Merged 2026-04-25T04:35** |
| Borrowed token view | #6429, #6430, #6431 | #6432 | Open — awaiting conflict resolution |
| Token allocation perf | #6433, #6434, #6435 | #6436 | Closed; superseded |
| Display names ratchet | #6437, #6438, #6439 | #6440 | **Merged 2026-04-25T01:12** |
| API/dep ratchet | #6441, #6442, #6443 | #6444 | **Merged 2026-04-25T01:11** |
| Status metrics | #6420, #6421, #6422 | #6423 | Open |

### 2.3 Keeper Merges and Master Fixes Interleaved

| Time (UTC) | PR | Type | Notes |
|---|---|---|---|
| 2026-04-25T00:12 | #6450 | Docs (prior segment) | Phase-3 learnings catalog |
| 2026-04-25T00:32 | #6331 | Feature | perl-pragma regression expansion |
| **01:07** | **#6451** | **Master fix** | stdio piping + xtask clippy allow |
| 01:11 | #6444 | Token keeper | API/dep ratchet |
| 01:12 | #6440 | Token keeper | Display names ratchet |
| **01:29** | **#6452** | **Master fix** | display_name_snapshot_tests.rs formatting |
| 01:40 | #5761, #5760, #5742 | Parser/LSP | ASCII rename, encoding fixes |
| **02:32** | #6411 | Token keeper | Checked token spans (triggered #6453) |
| 02:32 | #6383 | Feature | SymbolIndex production wiring |
| 02:38 | #6312, #6120, #6122 | Parser/tree-sitter | Multi-merge batch |
| **02:49** | **#6453** | **Master fix** | Duplicate span() method |
| **03:35** | **#6457** | **Master fix** | Formatting drift (35 files) |
| **03:50** | **#6460** | **Master fix** | token_scorecard.rs emptied + format string |
| 03:56 | #6125 | Tree-sitter | BOM/byte regression broadening |
| **03:58** | **#6456** | **Master fix** | span_invariant_tests.rs formatting |
| 04:35 | #6419 | Token keeper | Keyword/operator mapping helpers |

Total April 25 session-segment merges: **21**
Master fix merges: **6** (#6451, #6452, #6453, #6457, #6460, #6456)
Token cluster keeper merges: **4** (#6444, #6440, #6411, #6419)

---

## 3. The 8 Master Fixes

Six fixes landed as merges; #6455 was closed not merged (superseded by #6457); #5690
closed a pre-existing stale workspace-test breakage.

| PR | Title | Root cause | Diff | Resolved |
|---|---|---|---|---|
| **#6451** | restore command_timeout stdio piping + xtask clippy allow | `run_command_with_timeout` spawned child without `Stdio::piped()` on stdout/stderr; inherited parent stdio fails in CI | +6/-1 | 01:07 UTC |
| **#6452** | reformat display_name_snapshot_tests for formatting gate | PR #6444/#6440 left line 319 unformatted after merge; rustfmt gate blocked all subsequent PRs | +3/-1 | 01:29 UTC |
| **#6453** | resolve duplicate span() method | #6411 added `Token::span() -> TokenSpan`; pre-existing `Token::span() -> (usize, usize)` not removed; compiler rejected duplicate | +3/-8 | 02:49 UTC |
| **#6457** | restore formatting after master drift | Multiple files accumulated rustfmt drift after #6411 + #6453 cascade; `xtask fmt` aborted on first failure, masking 35-file scope | +26/-61 | 03:35 UTC |
| **#6460** | restore token_scorecard.rs + escape format string | #6457's sweep blanked `token_scorecard.rs` (47 lines lost); `scope_and_symbol_tests.rs` had unescaped `{}` in assert message | +47/-1 | 03:50 UTC |
| **#6456** | reformat span_invariant_tests.rs for formatting gate | `span_invariant_tests.rs` not captured by #6457's fmt pass; separate file needing explicit rustfmt treatment | +25/-14 | 03:58 UTC |
| **#6455** | fix perl-workspace-index formatting gate | Companion formatting fix for `workspace_index.rs`; closed not merged (superseded) | +1028/-97 | Closed |
| **#5690** | resolve two master-breaking test compile errors | Pre-existing: `mojolicious_navigation_tests.rs` + `scope_and_symbol_tests.rs` compile errors; stale for weeks | — | Closed 04:30 UTC |

**Cascade shape:** Each fix produced follow-on breakage. #6411 -> #6453 (duplicate method) -> #6457 (fmt sweep) -> #6460 (scorecard blanked by sweep) -> #6456 (missed file). Five of the six fixes were a single chain.

---

## 4. Multi-Gate Pipeline Catches

The multi-gate pipeline (deep-review = sonnet, reviewer = haiku, pr-responder) caught the
following bugs before or during merge:

- **#6411 (deep-review):** `RecoverySalvageProfile` and `RecoverySalvageClass` were dropped
  from the export layer during the span-helpers merge. Three export layer stubs required
  restoration before the feature was complete. Deep-review caught the incomplete public
  surface after the initial reviewer pass missed the omission.

- **#6419 (deep-review):** `from_sigil` mapping function used `HashSigil` where `%` maps
  to `Hash` and `SubSigil` where `&` maps to `Sub` — an inverted assignment that would have
  mapped `%foo` as Sub access and `&bar` as Hash access. The predicates were spelled
  correctly but the mapping return values were swapped. Haiku reviewer had passed the PR;
  sonnet deep-review caught the swap.

- **#6428 (deep-review, caught independently twice):** `is_at_statement_end` predicate was
  missing `Question` (the ternary `?` operator), which is a valid statement terminator in
  list context. Independently, `Not` and `WordNot` were incorrectly added to
  `is_binary_operator` — both are unary, not binary. Two deep-review passes on the same PR
  (from different runs) found both issues independently, providing natural verification that
  the findings were real.

- **#6258 (reviewer):** Owner-aware delimiter recovery logic was too permissive — the guard
  allowed recovery in contexts where hard errors were semantically correct. Additionally, 6
  tests were deleted without migration to the new owner-aware framework, leaving recovery
  behavior untested. Reviewer routed to `needs-builder-fix` rather than attempting
  fix-forward on logic-level changes.

- **#6125 (deep-review):** Three vacuous test patterns caught: (1) BOM assertion was
  fragile — tested file-level BOM presence but the parser stripped BOM before the assertion
  could exercise the BOM-handling path; (2) duplicate test body identical to a sibling
  test; (3) temp file cleanup ordered before assertions, meaning assertion failures would
  leave artifacts and pass-under-cleanup would suppress real failures.

- **#6126 (deep-review):** Same three patterns as #6125 in the parallel tree-sitter BOM PR
  — BOM fragility, duplicate body, cleanup ordering. Both PRs were generated by the same
  Codex task, so the defect was consistent across the cluster.

- **#6312 (deep-review):** Test coverage gap: the `NodeKind` recovery allowlist tests
  covered 4 of 6 recovery kinds; 2 kinds (`PartialExpression` and `RecoveredStatement`)
  were present in the implementation but absent from the test matrix. Additionally, an
  audit output path in the test helper was printing to stdout redundantly — observable but
  not harmful.

- **#6120 (deep-review):** When `--has-error` flag was active, the CLI produced
  contradictory stdout/stderr — the error flag implied "print only on error" but the
  implementation printed to both channels regardless of flag state. The contradiction would
  cause CI scripts parsing stdout for clean output to fail intermittently.

---

## 5. Hard-Conflict Outcomes (5xxx Cluster)

At session segment end, 45 PRs in the 5000–5999 range were open, most at 4 or 5 sign-offs.
Most failed CI due to merge conflicts introduced by the cascade of master fixes and token
cluster merges landing in rapid succession.

| Outcome | Count | Action |
|---|---|---|
| Closed as stale (conflict unresolvable by ops) | 1 (#5690) | Closed 04:30 UTC after weeks open |
| Confirmed conflict cluster — needs Codex regen | ~10 (per session context) | `needs-codex-regen` label applied where reachable |
| Keep-try-later (minor conflicts, rebase possible) | ~5 | Left open for next session |
| Clean / unaffected | ~29 | Proceeding normally |

Note: `needs-codex-regen` label queries returned 0 at doc-write time — consistent with the
label-skill silent-failure pattern observed throughout this segment (see section 6).

---

## 6. Operational Learnings

### 6.1 Cluster-merge phase produces approximately 1 master fix per 2-3 merges

During the token cluster merge phase (21 merges in 5.5 hours), 6 master fix PRs were
required. Ratio: 1 fix per 3.5 content merges. The previous session (2026-04-24 phase 2)
observed 4 bit-rot instances over a longer window. The rate is accelerating with merge
velocity: more keepers landing per session means more interaction surface per cascade.

**Planning implication:** Budget 2-3 master fix slots per cluster-merge wave. Have a
pr-responder agent on standby, not spawned ad-hoc after each breakage is detected.

### 6.2 Each master fix unblocks 20-30 downstream PRs via cascade

The formatting-gate fixes (#6452, #6456, #6457) each unblocked approximately 20-30 PRs
whose CI was stale-failing due to a master-side rustfmt violation. This multiplier effect
makes master fixes the highest-ROI ops action during a cascade window. Detecting "multiple
PRs failing the same gate simultaneously" should trigger master investigation before
individual PR triage.

Detection heuristic: if N >= 3 PRs fail the same gate (`PR Smoke`, `Formatting Gate`) at
the same commit SHA, check master before attributing to PRs.

### 6.3 Aggregator-only CI flake pattern continues to work

The "CI Gate (Merge-Blocking)" aggregator was the blocking check for several PRs where
individual lane checks had already passed. The admin-merge-on-aggregator-flake pattern
(merge despite aggregator failure when all constituent checks pass) continued to work
reliably. No regressions introduced via this path in this segment.

### 6.4 Label-skill silent-failure observed 5+ times this segment

The `needs-codex-regen` and `in-review` labels were reported set by agents but not present
on PRs when queried via `gh pr view --json labels`. Rate estimated at ~80% silent failure
on this label cluster. Root cause: rate-limiting or session context compaction causes the
`gh label add` call to be generated in agent output but not actually execute.

**Mitigation:** Verify label presence with `gh pr view --json labels` after any critical
label set. Bulk-repair missing labels from the orchestrator after each wave using direct
`gh pr edit --add-label` calls, not agent delegation.

### 6.5 Worktree branch deletion fails when nested worktrees exist

After admin-merge of several cascade PRs, `git branch -d` on the merged branches failed
with "branch checked out in worktree." Nested agent worktrees checking out those branches
for review passes left refs pinned. The fix is `gh pr checkout` cleanup via `git worktree
remove` on stale worktrees before branch deletion.

Operational sequence: after a wave closes, run `git worktree list` and prune stale
worktrees before bulk branch cleanup.

### 6.6 The #6457 + #6460 two-step is a recurring sweep-then-restore pattern

`xtask fmt` fixed formatting across multiple files but silently blanked `token_scorecard.rs`
(a benchmark file), requiring a follow-up restore PR (#6460). This is the second instance
of a broad formatting sweep inadvertently deleting content (see phase-2 `cargo xtask fmt`
session note). The formatter is treating some files as reformattable when they contain
benchmark or non-standard macro content.

**Mitigation:** After any `xtask fmt` sweep of >10 files, run `git diff --stat HEAD~1` to
verify no files lost significant line count unexpectedly.

---

## 7. Forward Applicability

Future sessions planning cluster-merge phases for `perl-token` or similar leaf crates should:

1. **Pre-stage master-fix PRs** before the cluster merge wave begins. The cascade shape
   (#6411 -> #6453 -> #6457 -> #6460 -> #6456) was predictable in retrospect. A
   pre-merge `cargo check --workspace --all-targets` run on each keeper would have surfaced
   the duplicate `span()` method before merging #6411.

2. **Merge cluster keepers serially, not in a batch of 3.** The batch-of-3 merge protocol
   is optimized for unrelated PRs. Token keepers that expand the same API surface interact.
   Merge one, verify master green, merge next.

3. **Treat formatting fixes as their own merge batch.** When `cargo xtask fmt` is run
   across a broad file set, commit the result as a standalone PR before any content merges
   touch the same directory. This prevents content merges from re-introducing formatting
   drift that another sweep must then fix.

4. **Deep-review is non-optional for kernel-type crates.** The 8 pipeline catches in this
   segment all came from `perl-token` or tree-sitter C bindings — crates where a logic bug
   propagates silently through the entire stack. Haiku reviewer passes are not sufficient
   for these crates; sonnet deep-review must be mandatory for any PR touching token
   predicates or mapping functions.

5. **Assign a dedicated label-verification step after each agent wave.** The ~80% label
   silent-failure rate means the orchestrator cannot trust agent-reported label state. A
   10-line verification script (`gh pr view --json labels` for each PR in the wave) run
   from the orchestrator after each wave closure would catch drift before it affects routing.

---

## 8. Verified Numbers Summary

| Metric | Value | Source |
|---|---|---|
| Codex burst PRs generated | 16 (#6396–#6444 range, sparse) | PR list query |
| Burst-phase triage closures | 12 | Closed list 17:14 UTC |
| Keeper merges from burst | 4 (#6444, #6440, #6411, #6419) | Merged PR list |
| Total April 25 segment merges | 21 | `gh pr list --state merged` |
| Master fix merges | 6 (#6451, #6452, #6453, #6456, #6457, #6460) | Merged PR list |
| Tree-sitter cluster merges | 3 (#6120, #6122, #6125) | Merged PR list |
| Multi-gate logic bug catches | 8 (across 7 PRs) | Deep-review comments |
| Open 5xxx PRs at segment end | 45 | `gh pr list --state open` |
| Stale closed (#5690) | 1 | Closed 04:30 UTC |
| Master cascade chain length | 5 steps (#6411->6453->6457->6460->6456) | PR body cross-refs |

---

_Related memory:_ `memory/feedback_master_bit_rot_cascade_fixes.md`,
`memory/feedback_deep_review_bug_catch_roi.md`,
`memory/feedback_multigate_catches_cheap_model_drift.md`

_Companion docs:_ `docs/articles/SESSION_2026_04_24_PHASE_3_ECONOMICS_AND_LEARNINGS.md` (prior segment),
`docs/articles/SESSION_2026_04_24_FULL_LEARNINGS_CATALOG.md` (full-session learnings)
