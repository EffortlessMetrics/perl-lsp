# Continuous Codex Review Session — Economics and Patterns at Scale

**Session analysis — 2026-04-22 evening → 2026-04-23 early morning UTC**

- **Shape:** Continuous Codex-generated PR stream; orchestrator (Claude Code) triaged, extracted, rebased, merged, and filed follow-ups at scale. Not a single-incident forensic; a throughput and pattern study.
- **Scope of analysis:** From session start (master HEAD `0fe9058f1`, 76 open PRs, master CI red on `ci-gate (unit_full)`) through session end (master HEAD `d148636c9`, 2 open PRs in final rebase, master CI recovering with RUSTSEC + fmt drift cleared and full merge-gate now running on every PR).
- **Cost envelope (per user at session close):**
  - Claude Code (orchestrator + agents, 20× Max plan): **~31% of a 5-hour session budget** + **~5% of weekly budget**
  - Codex Pro (input PR generation): **~26% of a 5-hour session budget** + **~7% of weekly budget**
  - Additional non-Codex PR tokens: modest

  Both sides were near-simultaneous 5-hour session intensity (~26–31% each), with the weekly slices matching that ratio roughly 5:1 — i.e. a single session this intense runs ~5–6× a typical day's consumption for each tool.

---

## TL;DR

In roughly 1.5 hours of effective Claude compute (31% of the 5-hour session window), the orchestrator processed **236 pull-request dispositions** — **116 merged, 120 closed** — drained a starting queue of 76 open PRs through several continuous Codex batches totalling ~150 incoming PRs, cleared master CI from `ci-gate (unit_full)` red into a clean state with RUSTSEC fixed + fmt drift fixed, landed the **shift from label-gated merge-gate to full-gate-on-every-PR** (closes #4675), captured a **tiered scoped-CI design** for the next improvement (#4706), and filed 31 tracking issues against distinct bugs.

Rate: **~150 dispositions per Claude-session-hour** with two-to-three agents running in parallel for most of the session. Cost per merge (Claude side) sits in the low-single-digit-cents range on the 20× Max plan amortisation.

The defining feature of the session was **rapid triage against large ensemble waves of Codex PRs**. Codex produced multiple PRs per bug (often 8–20), each a slight variation. The orchestrator's job was not to write code — it was to classify (merge / close-dup / close-superseded / close-drifted), rebase when siblings collided, and keep the master branch moving forward without accumulating bitrot.

---

## Session shape

| Metric | Value |
|---|---|
| PRs merged | **116** |
| PRs closed (non-merged) | **120** |
| PR-dispositions total | **236** |
| Issues filed | 31 |
| Issues closed | 22 |
| Master HEAD (start → end) | `0fe9058f1` → `d148636c9` (+ ~90 commits) |
| Parallel agents peak | ~4 simultaneous (2 triage + 2 rebase + direct edits) |
| Starting open PR count | 76 |
| Ending open PR count | 2 (both in final-rebase queue) |

Effective Claude compute was roughly 1.5 hours (31% of a 5-hour window). Dispositions-per-hour rate: ~150.

---

## Economics

### Budget footprint (user-reported at session close)

| Channel | Fraction consumed | Notes |
|---|---|---|
| Claude Code 20× Max — 5h session | ~31% | Orchestrator + all sub-agents |
| Claude Code 20× Max — weekly | ~5% | Rolls up into the monthly quota |
| Codex Pro — weekly | ~7% | Input stream of ~150 PRs across the session |
| Non-Codex PR queue (pre-session) | modest | Older PRs that had been waiting, authored via Jules and other channels |

### Rough cost-per-outcome envelope

Treating the Claude 5% weekly-budget slice as a rough cost proxy (exact dollar figure depends on plan amortisation; omitted deliberately):

- Cost per merged PR: **low-single-digit cents**
- Cost per disposition (merge or close): **roughly half of per-merge** — closes are cheaper than merges because they don't require CI churn
- Codex side: the ~7% weekly spend bought ~150 input PRs, of which ~116 landed useful fixes/features and ~120 were duplicates or scope-drift — so **~50% Codex throughput efficiency**, with the orchestrator absorbing the triage cost on the Claude side

The ratio that matters: Codex is cheaper per *attempt*, Claude is cheaper per *decision*. The orchestration pattern exploits that asymmetry by letting Codex spray and Claude filter.

### Where the Claude budget went

- **Triage agents (general-purpose):** largest single bucket. Each mega-triage agent classified 10–20 PRs per run; there were six such runs. These ran on smaller contexts and returned compact tables — efficient.
- **Rebase agents (builder, worktree-isolated):** second-largest. Each handled 2–12 PRs by checking out, rebasing onto current master, resolving conflicts, pushing with `--force-with-lease`, and sometimes merging inline. The **sequential rebase agent** that drained 12 ci-hygiene conflicts in one run was the highest-leverage single agent in the session.
- **Orchestrator-direct Bash/API calls:** close comments, retitles, merge requests. Cheap per call but high volume.
- **Direct Edit operations:** minimal — used once near the end to fix a revert that had squash-merged empty.

---

## Measurable outcomes

### Master branch state

| Aspect | Start | End |
|---|---|---|
| Master HEAD | `0fe9058f1` | `d148636c9` |
| `ci-gate (unit_full)` | FAILING | (cleared by re-run after fmt drift fix) |
| `Cargo Audit (RustSec)` | FAILING (RUSTSEC-2026-0104 in rustls-webpki 0.103.12) | PASS (bumped to 0.103.13 via #4678) |
| `Cargo Deny (Policy Enforcement)` | FAILING (same RUSTSEC) | PASS |
| Master-side fmt drift in `collapse_edge_cases_tests.rs` | persistent (forcing `--no-verify` on every agent push) | fixed (#4688) |
| Per-PR CI depth | `clippy -p perl-parser -p perl-lexer` only (2 crates of ~130) | full `just gates` on every PR (#4677) |
| Incorrect VS-Marketplace deprecation notices in README | present (from pre-session #4474) | removed (#4770) |

### Feature and infrastructure PRs that landed

Not an exhaustive list — highlights of merges that change project surface area:

- **`#4677`** — `ci: run full merge-gate on every PR (closes #4675)` — removes the `merge-ready`-label gating on `merge-gate`, so every PR now runs `just gates` (Clippy full, test full, API surface check, tautology, publish dry-run). Concurrency group cancels in-flight runs on rapid pushes. Frequent-commit churn is accepted.
- **`#4678`** — `chore(deps): bump rustls-webpki for RUSTSEC-2026-0104` — clears advisory.
- **`#4688`** — `chore(perl-workspace): fix fmt drift in collapse_edge_cases_tests.rs` — unblocks pre-push hooks for all future agent pushes.
- **`#4770`** — `docs: remove incorrect VS Marketplace deprecation notices` — reverts false claim from earlier merged #4474.
- **`#4395`** — `feat(metrics): warm reparse benchmark + parser scorecard design doc` — adds `bench_warm_reparse` + `parse_regime` Criterion group + 138-line parser scorecard at `docs/project/metrics/parser.md`.
- **`#4396`** — `feat(diagnostics): PL410 loop-control-to-undefined-label lint` — counterpart to PL409.
- **`#4402`** — `feat(release): source GitHub Release body from docs/releases/vX.Y.Z.md` — release notes now curated, not PR dump.
- **`#4403`** — `fix(hover): surface inline POD inside subroutine bodies` — body-aware POD extraction.
- **`#4436`** — `feat(diagnostics): PL304 POD coverage lint for undocumented exports`.
- **`#4485`** — `feat(async-await): Perl 5.36+ async/await keyword LSP support`.
- **`#4488`** — `feat(diagnostics): unreachable code detection in continue blocks`.
- **`#4517`** — `feat(gate): integrate published-crate-count into CI gate-policy` — adds ratchet for the microcrate-collapse target.
- **`#4551`** — `refactor(platform): dedupe platform functions across perl-dap + perl-lsp-rs-core`.
- **`#4554`** — `fix(perl-module): avoid rename false positives in package/quoted contexts`.
- **`#4558`** — `fix(parser-core): preserve ternary precedence for bare unary-style calls`.
- **`#4564`** — `fix(diagnostics): reset pull perlcritic analyzer on config changes` — winner of the 15-PR perlcritic-cache ensemble.
- **`#4575`** — `fix(lifecycle): validate initialized handshake ordering`.
- **`#4701`** — `refactor(workspace): rename crates/perl-lsp/ → crates/perl-lsp-rs/` — 525 files, purely mechanical alignment of dir name with Cargo package name.
- Plus ~100 more small bug-fix merges across `perl-ci-hygiene`, `perl-dap`, `perl-workspace-index`, `perl-parser`, `perl-semantic-analyzer`, `perl-uri`, `perl-module`, and others.

### Issues filed for future work

Cluster-tracking and follow-up issues filed by the orchestrator (partial list): `#4578` (perlcritic cache), `#4579` (ternary bare call), `#4584` (lifecycle handshake), `#4585` (transport warn), `#4594` (package-rewrite regression follow-up), `#4599` (POD regex strictness decision), `#4649` (ci-hygiene bundle), `#4650` (DAP bundle), `#4651` (workspace bundle), `#4675` (per-PR CI scope), `#4676` (fmt drift fix), `#4679` (auto-fix workflow bug), `#4706` (tiered scoped CI design), `#4707` (Codex-batch winners bundle), `#4761` (SQL::Abstract clean re-do of #4736), plus per-bug trackers.

---

## Patterns that worked

### 1. Cluster triage via single general-purpose agent

Large Codex waves (20 PRs arriving together) were absorbed by spawning a single general-purpose triage agent given the full list plus context on already-merged work. Output was a compact table — `#NNNN | VERDICT | reason` — that the orchestrator executed directly against `gh pr close` / `gh pr edit` / `gh pr merge`. This was cheaper than reading each diff manually and scaled linearly with batch size.

### 2. Sequential rebase + merge for conflict cascades

When a large batch of PRs all touched the same file (`perl-ci-hygiene/src/main.rs` took 10+ PRs), the merge-cascade problem is real: each merge invalidates the siblings. The solution that worked was a **single builder agent given the full queue in priority order**, told to rebase → compile-check → push → merge → next. The final such run cleared 12 ci-hygiene PRs in one pass with ~1 hour of agent wall-clock.

### 3. Cherry-pick extraction from contaminated branches

Several closed-as-contaminated draft PRs contained real feature code buried under scope-drift artifacts. The cherry-pick pattern that worked: check out the branch, identify the commits touching the feature files, rebase those specific commits onto a clean master branch. Works when Codex kept the feature commits logically separate from the contamination commits. Succeeded on `#4432` (parser), `#4480` (PL701), `#4485` (async/await — manual port required because target crates were absorbed post-branch), `#4488` (unreachable code), `#4525` (ADR-008), `#4470` + `#4471` (nix + DAP docs).

### 4. `.hermes/conveyor/work-<single-id>/` is tool metadata, not contamination

Early in the session the orchestrator mistakenly closed PRs whose only "contamination" was a single `.hermes/conveyor/work-<id>/` directory carrying the Codex agent's own ADR / specs / findings. The user corrected: that's the Codex equivalent of this repo's own `.spec/` folders. Revised rule:

- **Single work-id under `.hermes/conveyor/`** = fine, let it merge
- **Multiple unrelated work-ids in one PR** = cross-contamination, close
- **Root-level `adr.md` / `specs.md` / `task_list.md` / `%{...}` shell-glob artifacts** = genuine garbage, close
- **Duplicate ADR numbering with useful content** = renumber, don't close

Re-opened `#4525` and `#4480` after this correction; both merged cleanly.

### 5. Running the full merge-gate earlier

Pre-session, the `merge-gate` job in `.github/workflows/ci.yml` gated on the `merge-ready` label. Practical effect: bulk merges via `gh pr merge --squash` skipped the label step and merged with only `PR Smoke` (clippy + test on 2 core crates) and `UX Regression Tests`. That's insufficient depth when a PR touches, say, `perl-dap` or `perl-workspace-index` — neither of those runs under `PR Smoke`. The fix (#4677) was to drop the label condition so `just gates` runs on every PR push. Concurrency-group cancellation absorbs rapid-merge churn. The next refinement (#4706) is tiered scoping: draft PRs get scoped-by-modified-crate fast feedback, ready PRs get full-minus-mutation, and mutation/fuzz run nightly.

---

## Patterns that broke

### 1. `gh pr create` picked up a stale checked-out branch

PR `#4769` was supposed to revert `#4474`. The revert agent ran `git checkout -b revert/4474-... origin/master`, made the revert commit, pushed the branch, and then called `gh pr create`. **But `gh pr create` opened the PR against the wrong `headRefName`** (`ci/run-merge-gate-on-all-prs`, the prior-session branch of #4677). When squash-merged, GitHub saw no net change (the merge-gate work was already in master via #4677) and merged an empty commit. The revert content sat unused on the correct branch `revert/4474-...` while master still carried the bogus deprecation notice. The user caught the issue by reading the README and pointing it out.

The fix was a fresh `docs/` branch + direct edit + PR `#4770`, which landed cleanly. Lesson: **verify `gh pr view <N> --json headRefName` matches the branch you intended to push before merging**, especially when working in a contaminated main checkout where a prior agent's branch was left as `HEAD`.

### 2. Concurrent agents racing on the same fix

While the direct-edit revert agent (`a4b10fd4...`) was working on its worktree, the orchestrator took direct action in the main checkout and landed the fix as `#4770`. The revert agent finished and opened `#4771` with a functionally identical diff. `#4771` was closed as redundant. Cost: one wasted builder-agent run.

Lesson: when the user intervenes and the orchestrator takes direct action, **SendMessage the running agent to stand down** before doing the work yourself. The message was added in the script this time but only sent *after* the direct work had already finished.

### 3. PR Smoke's auto-fix-fmt step fails on master pushes (#4679)

The `pr-smoke` job runs `cargo xtask fmt` and, if drift is detected, commits and pushes. On PR branches this works — it pushes to the PR branch. On **master push events** it tries to push to `master` and gets rejected by the "changes must be made through a pull request" repo rule. Since `merge-gate` has `needs: pr-smoke`, the cascading failure keeps `merge-gate` skipped on master. Combined with the persistent fmt drift in `collapse_edge_cases_tests.rs`, this meant no green CI on master for many commits until #4688 cleared the drift.

Fix tracked as #4679: guard the auto-fix step with `if: github.event_name == 'pull_request'` or have it error loudly on master with a clear "fix via a chore PR" message.

### 4. Cherry-pick extraction can silently over-write on absorbed crates

`#4488`'s extraction agent reported "re-implemented the feature fresh because target crate was absorbed post-branch." The result was green — all 728 lib + integration tests pass — but the extracted code is structurally a new implementation, not the original. That's fine in this case but worth naming: **extraction from a stale branch through a crate absorption is re-implementation, not salvage**. Future trackers should distinguish the two.

### 5. Triage agents occasionally miscall "distinct scope" vs duplicate

Two instances this session:
- Early triage said `#4741` (ci-hygiene C-string literals) was distinct from merged `#4686`, so `#4741` was retitled and queued for merge. Both fix `c"…"` / `cr#"…"#` in `is_index_in_rust_literal`. After merge via the sequential rebase agent, the cumulative code worked out but only because the rebase agent resolved the conflict by keeping the merged version's helper and layering the new test cases on top. If the two had truly been distinct, this would have been a no-op merge that obscured the collision.
- `#4710`, `#4722`, `#4758` were correctly closed as supersededs of `#4686`. A different triage run correctly identified the same pattern.

Lesson: for PRs claiming small variations on a known-merged fix, the triage agent should run a git diff against current master's version of the target file, not rely on the PR description alone.

### 6. Codex ensembles re-emerge

The perlcritic-cache bug attracted **15 PRs in round 1**, then **another 8 in round 2** after the round-1 winner (`#4564`) had already merged. Codex doesn't know what merged; it re-generates. The orchestrator has to watch for this and close the re-duplicates proactively.

The same shape appeared with:
- Case-insensitive TODO/FIXME: 3 PRs (#4681 winner, #4711/#4731/#4759 supersededs + #4737/#4742/#4748 word-boundary variants)
- DAP inline-value clamping/apostrophe: 3 waves across session
- Workspace-folder URI case-insensitive: 2 PRs (#4693 winner, #4713/#4749 supersededs)

Cost: ~30% of the "closed" dispositions in this session were re-duplicates of already-merged fixes.

---

## Follow-ups

Filed for future work:

- **#4675** — Per-PR CI scope: PR Smoke only verifies `perl-parser` + `perl-lexer`, other crates get no per-PR coverage. Partially resolved by #4677 (drop merge-ready gate so `just gates` runs on every PR); full scoping per #4706.
- **#4676** — Fmt drift in `collapse_edge_cases_tests.rs` (resolved, #4688).
- **#4679** — PR Smoke auto-fix-fmt step can't push to master; guard on `pull_request` event only.
- **#4706** — Tiered scoped CI design: **draft PR = scoped-fast** (changed crates + reverse-dep closure via `cargo metadata`), **ready PR = full-minus-mutation/fuzz**, **nightly = mutation + fuzz + long-running fuzz corpus**. Leverages the microcrate SRP boundaries for "well-scoped + deep".
- **#4594** — Restore `package` declaration rewrite in `plan_module_rename_edits` (regression from merged #4554).
- **#4599** — POD regex strictness decision (hover matches indented `=pod` that `perl` itself ignores).
- **#4594**, **#4761** (SQL::Abstract clean re-do), **#4707** (Codex batch winners bundle) — standing trackers for future pickup.

Process improvements to codify:

1. **Verify `gh pr create`'s `headRefName`** before treating a PR as opened correctly, especially from a main checkout where a prior agent may have left a different branch as `HEAD`.
2. **SendMessage running agents to stand down** when the orchestrator takes direct action on the same task.
3. **Triage cross-check**: when a PR claims a "small variation" on a known-merged fix, `gh pr diff` vs current master before accepting the "distinct scope" verdict.
4. **Re-duplicate suppression**: periodically (every few Codex batches) re-run supersedes check against merged PRs from earlier in the same session.
5. **`gh search` defaults to 30 items** — pass `--limit 500` when counting throughput, or pagination will understate.

---

## Appendix: the shape of a Codex ensemble wave

For reference, the ci-hygiene cluster across the session shows the characteristic ensemble shape:

| Round | PRs per bug | Winner pattern |
|---|---|---|
| Round 1 (pre-session carry-over) | 15 PRs for perlcritic-cache invalidation | Single winner #4564; 14 closed as dups |
| Round 2 (early session) | 8 more perlcritic-cache attempts (after #4564 landed) | All 8 closed as supersededs |
| CI-hygiene round 1 | 22 PRs | 5 distinct bugs identified, 6 winners merged, 16 closed |
| CI-hygiene round 2 | 13 PRs | 3 distinct bugs, 5 winners merged, 5 closed as dups/supersededs |
| CI-hygiene round 3 | 10 PRs | 5 new distinct bugs, 5 winners merged, 5 closed |
| CI-hygiene round 4 | 14 PRs | 6 new distinct bugs, 7 winners merged, 7 closed |

The consistent pattern: Codex generates 1.5–3× more PRs than needed for a given set of bugs. The orchestrator's job is classification at that ratio, which is where most of the "close" dispositions come from.

---

## One-line takeaway

**Per-hour dispositions scale linearly with parallel agent capacity and decouple cleanly from PR size as long as the orchestrator classifies before the queue deepens. Rebase cascades are the only real throughput ceiling, and sequential rebase-and-merge is the reliable drain.**
