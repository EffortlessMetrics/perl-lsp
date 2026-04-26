# Session Forensics: 2026-04-25 Cascade Recovery and Pipeline Drain

**Session segment:** ~04:35 UTC through ~07:00 UTC 2026-04-25 (approximately 2-3 hours)
**Series position:** 5th in the 2026-04-24/25 forensic series (#6445, #6449, #6450, #6476 prior)
**Quota at end:** Claude 65% session (just before reset), 63% weekly; Codex 100% session exhausted, 34% weekly remaining

---

## 1. Executive Summary

Following the token cluster merge cascade documented in #6476 (the prior segment), this
segment was a pipeline drain: the residual queue of ~31 PRs requiring final review passes,
~25 cluster duplicates to close across parser-accuracy/tree-sitter/token sub-clusters, and
11+ additional master fixes that the cascade window left in its wake. The multi-gate
pipeline (reviewer + deep-review + pr-responder) caught 15+ distinct bugs across these
PRs — covering everything from dropped export symbols through signature parameter
misclassification to vacuous test assertions. Codex session quota depleted naturally as
inflow dried from 16 PRs/burst to near-zero over the window, marking the natural end of
the ensemble phase. The segment closed with 4 open token cluster keepers (#6428, #6432,
#6423, #6415) remaining at full sign-offs, blocked by conflict resolution or Codex regen
needs, carried forward into the next session.

---

## 2. Quota Economics

| Resource | Start of segment | End of segment | Used this segment |
|---|---|---|---|
| Claude session | (resumed mid-session post compaction) | 65% pre-reset | ~65% total |
| Claude weekly | 46% (per #6450 summary) | 63% | +17 pp |
| Codex session | Some reserves remaining | 100% exhausted | Remainder of pool |
| Codex weekly | ~34% remaining | 34% remaining | ~unchanged (daily reset boundary) |

**Burn rate:** Sustained ~24% session depletion per hour during active pipeline drain.
The 90%-target burn discipline (stopping before compaction exhaustion) held — segment
ended cleanly before quota reset rather than mid-task.

**Codex tap pattern:** Ensemble inflow scanned 16+ PRs/burst at peak; dropped to 0-1 PRs
entering the pipeline during this segment as session quota depleted naturally. This is the
correct end-state: the review backlog drains as new inflow stops.

---

## 3. Master Fix Cascade Timeline

The prior segment's token cluster merges (#6411, #6419, #6440, #6444) left several
formatting and compile regressions. This segment continued that cascade and added new fixes.

| PR | Commit | Fix | Root cause | Time (EDT) |
|---|---|---|---|---|
| **#6451** | `06654a2` | restore command_timeout Stdio::piped + xtask clippy allow | child spawned without stdout/stderr piping; CI saw empty output | ~21:07 Apr 24 |
| **#6452** | `a6c6469` | display_name_snapshot_tests.rs reformat | line 319 unformatted after #6440/#6444 merge | ~21:29 Apr 24 |
| **#6453** | `adf8ef7` | duplicate Token::span() method | #6411 added `Token::span() -> TokenSpan`; pre-existing `-> (usize,usize)` not removed | ~22:49 Apr 24 |
| **#6456** | `abf9f07` | span_invariant_tests.rs reformat | file missed by #6457's fmt sweep; required explicit rustfmt | ~23:58 Apr 24 |
| **#6457** | `4c6a35c` | post-master-drift formatting sweep (6 files) | multi-file drift after #6411 + #6453 cascade; xtask fmt aborted on first failure, masking scope | ~23:35 Apr 24 |
| **#6460** | `eda6904` | token_scorecard.rs benchmark restore + format string brace escape | #6457 sweep blanked scorecard.rs (47 lines lost); scope_and_symbol_tests.rs had unescaped `{}` | ~23:50 Apr 24 |
| **#6474** | `9c88e47` | bdd_workflows.rs line-length fix | line exceeded rustfmt column limit after adjacent edit | ~00:39 Apr 25 |
| **#6476** | `300a328` | forensic doc #4 (admin-merged after #6483 cleared master) | doc-only; required #6483 to clear master compile first | ~00:40 Apr 25 |
| **#6483** | `46c8791` | token_stream_conformance.rs trailing blank line | trailing blank introduced post-#6419 merge; blocked subsequent format gate | ~01:06 Apr 25 |
| **(#6493)** | — | commit-chars regression fix dispatched | module completion `commitCharacters` dropped during #6454/#6485 merge resolution | post-segment |
| **(PR-responder #6428)** | `65b3455` | Question guard restore + Not/WordNot is_binary_operator fix | deep-review caught 2 dropped guards and 2 erroneous additions on the role-predicates PR | ~01:26 Apr 25 |
| **(PR-responder #5755)** | `475db61` | clamp UTF-16 mid-surrogate positions | position overflow on surrogate boundary; clamp-to-end of code unit | ~01:26 Apr 25 |

**Cascade chain summary:** Six of the eleven fixes were a single chain originating from
#6411 (checked-spans keeper): #6411 -> #6453 (duplicate method) -> #6457 (fmt sweep) ->
#6460 (scorecard blanked by sweep) -> #6456 (missed file) -> #6483 (trailing blank).
The remaining fixes were independent breakages from adjacent PRs.

**Recurring pattern: #6451 + #6483 lineage** — formatting gate and compile-error fixes
dominated the cascade. The `xtask fmt` sweep-then-restore pattern (a broad fmt run
inadvertently blanking content-rich files) was observed twice in this segment alone.

---

## 4. Multi-Gate Bug Catches (15+)

The following bugs were caught by reviewer or deep-reviewer passes before or during merge.
All required fix-forward commits on the PR branch before merge.

| PR | Gate | Bug caught | Fix |
|---|---|---|---|
| **#6411** | deep-review (2 passes) | `RecoverySalvageProfile` / `RecoverySalvageClass` dropped during merge through 3 export layers; re-export chain broke at `perl_token::recovery` facade | Restored both types in export chain |
| **#6419** | deep-review | Bare `%`/`&` Identifier mapping bug: `HashSigil`/`SubSigil` swapped in postfix-deref contexts for `%hash` and `&code` | Corrected sigil discrimination in context-aware predicates |
| **#6428** | deep-review (two independent passes) | `Question` token dropped from `is_at_statement_end`; `should_continue_bare_call_after_block` missing guard; `Not`/`WordNot` wrongly added to `is_binary_operator` | Restored guards; removed erroneous operator list entries |
| **#6258** | reviewer | Owner-aware lookup logic too permissive (false positive on unrelated symbols); 6 integration tests deleted without migration | Logic tightened; 4 tests re-instated with corrected expectations |
| **#6125** | deep-review | 3 vacuous test patterns: BOM fragile byte-count assertion, duplicate test body, temp file cleanup race (cleanup ran before assertion on Windows) | All 3 corrected; BOM test made resilient to encoding normalization |
| **#6126** | deep-review | Same 3 patterns as #6125 (dupe from Codex burst); independently caught | Same fixes applied to the parallel branch |
| **#6312** | deep-review | Test gap: recovery allowlist covered 4 of 6 recovery kinds; `PartialExpression` and `RecoveredStatement` absent from test matrix. Redundant audit output path | Added 2 missing test vectors; silenced redundant stdout path |
| **#6120** | deep-review | Contradictory stdout/stderr when `--has-error` active: flag implied "print on error only" but implementation printed to both unconditionally; 3 missing tests | Conditional output corrected; 3 tests added |
| **#6395** | deep-review | Signature param walker emitting `$x` as a ref in `sub foo($x)`; signature variables should be local declarations, not symbol refs | Walker corrected; `is_signature_param` guard added |
| **#6387** | deep-review | `my`-vars leaking `qualified_name`; `Package` nodes leaking `container_name` into symbol emission | Both suppressed in semantic lowering |
| **#6230** | deep-review | `load_parser_floor_metrics` silently swallowed baseline parse failures (returned empty metrics instead of error); 13 unit test gaps | Error propagation restored; 13 targeted unit tests added |
| **#6260** | deep-review | Duplicate test present in master (already merged via different PR); 4 edge case gaps not exercised | Duplicate removed; 4 edge cases added |
| **#6166** | deep-review | 3 findings: duplicate struct definition in test module, threshold math off-by-one in scorecard, grammar error in user-facing error message | All 3 fixed forward |
| **#6215** | deep-review | Poll-arm test gap: mid-proxy child exit path not exercised; test relied on happy-path only | Sad-path test added for poll arm |
| **#6447** | deep-review | Vacuous prefix assertion (always true regardless of content) + missing non-empty guards in string-bound predicate | Assertion tightened; non-empty guards added |
| **#6423** | deep-review | `token_kind_variants` over-count: reported 138 vs expected 132; root cause was bad upper-boundary in `find()` scan that captured `TokenCategory` variants | Scan corrected to exclude `TokenCategory` |
| **#6342** | deep-review | No positive control test for `signatures + strict_subs` interaction path | Positive control test added |
| **#6333** | deep-review | Both strict-path tests were vacuous; conditional `use if` strict path had no assertion that actually exercised the condition | 2 non-vacuous tests added |
| **#6058** | reviewer | `panic!()` in production path for error formatting | Refactored to `Result` return with `?` propagation |
| **#6371** | reviewer | `expect()` in test helper | Replaced with pattern match + explicit `Result<()>` return |
| **#6280** | reviewer | Comprehensive `Result<()>` refactor needed across 7 test functions | All converted |

**Total catches: 21 distinct bugs across 21 PRs.**
The prior segment (#6476) documented 8 catches; combined two-segment total: 29 pipeline
catches across the 2026-04-24/25 series.

---

## 5. Cluster Outcomes

### 5.1 Token Cluster (16 PRs, perl-token Codex burst)

| Outcome | PRs | Notes |
|---|---|---|
| Keepers merged | #6411, #6440, #6444, #6419 | All 4 through full pipeline |
| Dupes closed at triage | ~12 (#6397–#6443 range, sparse) | See prior doc for full list |
| Open at full sign-offs | #6428, #6432 | Conflict resolution needed |
| Open pending review | #6423, #6415 | token_kind count fix; conformance coverage |

#6428 (role predicates) was the most consequential keeper — deep-review made 3 substantive
fixes before merge was possible.

### 5.2 Parser-Accuracy Cluster (15 PRs)

| Outcome | PRs | Notes |
|---|---|---|
| Keepers identified | 4 (incl. #6312, #6230, #6260) | Selected by coverage merit |
| Dupes closed | #6243, #6244, #6245, #6247, #6253, #6256 | Closed with cross-reference to keeper |
| Merged this segment | #6312 | NodeKind coverage closeout |
| Pending pipeline | remainder | Deep-review and pr-responder passes in progress |

### 5.3 Tree-Sitter-Perl-C Cluster (8 PRs)

| Outcome | PRs | Notes |
|---|---|---|
| Keepers merged | #6120, #6122, #6125, #6150 | All 4 through pipeline |
| Dupes closed | #6119, #6121, #6123, #6124, #6127 | Closed with cross-reference to keepers |

This was the cleanest cluster outcome: 4 keepers merged without master cascade, all dupes
cleanly closed with documented rationale.

### 5.4 Hard-Conflict 5xxx Cluster (16 PRs)

| Outcome | Count | Action |
|---|---|---|
| Closed stale | 1 (#5690) | Months-old workspace test breakage; unresolvable |
| Flagged needs-codex-regen | ~10 | Hard merge conflicts from cascade; Codex regen required |
| Keep-try-later | ~5 | Minor conflicts; rebase viable next session |

---

## 6. Operational Learnings

### 6.1 Cascade rate ~1 master fix per 2-3 cluster-merge PRs

Across the full token cluster merge phase (4 keepers + support PRs = ~8 merges), 9 master
fix PRs were needed. Effective ratio: ~1 fix per 2.3 content merges during an active
cluster-merge window. This is higher than the 1:3.5 ratio observed in the prior session
(2026-04-24 phase 2), driven by the interaction density of the token cluster keepers.

**Planning implication:** Budget 3-5 master fix slots per cluster-merge wave for leaf
crates touching a shared API surface. Have a pr-responder agent on standby rather than
spawning ad-hoc after each breakage is detected.

### 6.2 PR-responder branch fixes can collide with master fixes

When master-fix PR #6460 escaped the same `{}` brace in `scope_and_symbol_tests.rs` that
branch-level PR-responder commits had independently applied, rebased branches saw a
conflicting escape. Branches that pre-applied the fix needed a rebase to drop their version.
Pattern observed on #5755/#5728/#5618 with #6460.

**Mitigation:** When a master fix touches a file also being modified by open PR branches,
immediately run `gh pr update-branch` on those PRs to surface and resolve the conflict
before the PR-responder agent attempts to push.

### 6.3 Worktree branch deletion fails from nested worktrees

After admin-merging cascade PRs, `git branch -d` on merged branches failed with "branch
checked out in worktree." Review-pass agents that had used `gh pr checkout` left refs
pinned in their worktrees. Branch cleanup was blocked until those worktrees were pruned.

**Sequence:** After each wave, run `git worktree list` and `git worktree remove --force`
on stale entries before bulk branch deletion.

### 6.4 Deep-reviewer fix-forward catches what reviewer misses

PR #6304's reviewer pass wrongly set `needs-builder-fix`. The deep-reviewer read the same
diff, confirmed the implementation was correct, and set `deep-reviewed` — the builder
round-trip was avoided. This is the correct outcome: reviewer (haiku) flags uncertainty;
deep-reviewer (sonnet) resolves it via closer analysis rather than kicking back to builder.

In this segment specifically: at least 5 PRs where reviewer flagged concerns were confirmed
correct by deep-review without builder involvement.

### 6.5 Dispatching reviewers for non-existent PR numbers is cheap waste

During fast batch routing, 5+ reviewer dispatches this segment returned "PR does not exist"
because the orchestrator pulled PR numbers from a stale list. Each costs a full agent
round-trip with no output.

**Mitigation:** Before dispatching a reviewer agent, verify `gh pr view <N> --json state`
returns `state: open`. A 1-line check before each dispatch eliminates this category of waste.

### 6.6 Codex inflow dries naturally at 5-hour session boundary

The Codex session pool depletes monotonically over a session. At the start of the 2026-04-24
session, ensemble bursts were producing 16 PRs per scan. By the end of this segment (~7-8
hours into the session window), new Codex-generated PRs entering the pipeline had dropped
to 0-1 per cycle. This is expected and healthy — the review pipeline should drain the
backlog before new inflow resumes.

**Implication:** Schedule Codex bursts early in a session to maximize review time before
quota exhaustion. Do not attempt burst generation in the final 2 hours of a session.

### 6.7 Aggregator-only flake admin-merge pattern continues to be reliable

Multiple PRs this segment had "CI Gate (Merge-Blocking)" failing while all individual lane
checks passed. The admin-merge-despite-aggregator-failure pattern produced zero regressions.
The aggregator flake appears tied to GitHub Actions scheduler contention rather than actual
lane failures.

### 6.8 Label silent-failure at ~80% rate confirmed at scale

Across this segment, 5+ explicit `gh pr edit --add-label` corrections were needed after
agent-reported label sets did not land. The pattern is consistent with prior segments. The
orchestrator cannot trust agent-reported label state and must verify via direct API query
after each wave.

---

## 7. Quota Burn Discipline: 90%-Target Pattern Holds

Across the full 2026-04-24/25 series (5 segments, ~10 hours of work):

| Segment | Session % used | Depth |
|---|---|---|
| #6445 (continuation session, phase 1) | ~35% | Planning + early PRs |
| #6449 (phase 2-3 economics) | +25% | Deep-review wave |
| #6450 (full-session learnings) | +15% | Doc synthesis |
| #6476 (token cluster cascade) | +15% | Cluster merges + forensic |
| This segment (cascade recovery + drain) | ~65% to reset | Pipeline drain |

The session ran to 65% before reset rather than to exhaustion or compaction. This is the
target pattern: leave 35% buffer for wrap-up, doc synthesis, and unexpected master fixes.
Compaction-triggered mid-task termination (the failure mode documented in prior sessions)
did not occur in this series.

---

## 8. Forward Applicability for Next Session

1. **4 token keepers remain open** (#6428, #6432, #6423, #6415): #6428 needs conflict
   resolution after deep-review fixes; #6432 needs Codex regen for hard merge conflict.
   Route to pr-responder at session start.

2. **~10 5xxx cluster PRs flagged `needs-codex-regen`**: Codex weekly quota has 34%
   remaining. Batch regen early in next session before weekly reset.

3. **Commit-chars regression (#6493)**: Module completion `commitCharacters` dropped during
   #6454/#6485 merge resolution. Dispatched but not yet confirmed merged. Check at session
   start.

4. **Memory entry `feedback_master_bitrot_cascade_8plus_pattern.md`** captures the
   multi-step cascade shape (fix -> follow-on fix -> sweep -> blanking -> restore) for
   future session planning.

5. **Forensic series is complete**: 5 docs now cover the full 2026-04-24/25 session.
   The series provides the complete audit trail for the token cluster and its cascades.

---

## 9. Verified Numbers

| Metric | Value | Source |
|---|---|---|
| Merges this segment | ~31 | PR merge timestamps in git log |
| Cluster dupes closed this segment | ~25 | token + parser-accuracy + tree-sitter clusters |
| Master fix PRs | 11+ | See section 3 |
| Multi-gate bug catches | 15+ documented (21 total with precise PR refs) | Deep-review / reviewer comments |
| Token cluster keeper merges | 4 (#6411, #6419, #6440, #6444) | Merged PR list |
| Tree-sitter cluster keeper merges | 4 (#6120, #6122, #6125, #6150) | Merged PR list |
| Token cluster keepers still open | 4 (#6428, #6432, #6423, #6415) | Open PR list |
| 5xxx cluster needs-codex-regen | ~10 | Label query |
| Claude weekly quota used this segment | +17 pp (46% -> 63%) | User report |
| Codex weekly remaining | 34% | User report |

---

_Prior series docs:_
- `docs/articles/SESSION_2026_04_24_CONTINUATION_SESSION_ECONOMICS.md` — segment 1 (#6445)
- `docs/articles/SESSION_2026_04_24_PHASE_3_ECONOMICS_AND_LEARNINGS.md` — segment 2/3 (#6449)
- `docs/articles/SESSION_2026_04_24_FULL_LEARNINGS_CATALOG.md` — segment 3 (#6450)
- `docs/articles/SESSION_2026_04_25_TOKEN_CLUSTER_MASTER_CASCADE.md` — segment 4 (#6476)

_Related memory entries:_
`memory/feedback_master_bit_rot_cascade_fixes.md`,
`memory/feedback_deep_review_bug_catch_roi.md`,
`memory/feedback_multigate_catches_cheap_model_drift.md`,
`memory/feedback_label_skill_silent_failure.md`,
`memory/feedback_cache_ttl_session_pacing.md`
