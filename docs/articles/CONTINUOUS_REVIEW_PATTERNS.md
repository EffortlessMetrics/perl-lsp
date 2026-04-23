# Continuous Review Patterns: Running a Codex Stream at 150 Dispositions/Hour

*Session learnings from 2026-04-22 — a study of triage, extraction, and rebase-cascade drainage when Codex is spraying PRs faster than humans can read them.*

---

## The session shape

One afternoon, Codex sent ~150 PRs into the perl-lsp repo over the course of a few hours. The orchestrator (Claude Code) processed them — not by writing code directly, but by triaging, rebasing, extracting real content from contaminated branches, and landing the survivors.

By the close of the 5-hour Claude session window (roughly 1.5 hours of effective compute used), the numbers looked like this:

- **116 PRs merged**
- **120 PRs closed** (duplicates, supersededs, scope-drifted)
- **236 dispositions total**
- **31 tracking issues filed**, 22 closed

Rate: ~150 dispositions per Claude-session-hour. Cost breakdown (per user at session close):

- **Claude Code 20× Max**: ~31% of a 5-hour session window + ~5% of the weekly budget
- **Codex Pro**: ~26% of a 5-hour session window + ~7% of the weekly budget

Both sides ran at near-matched session intensity (~26–31%) — a session this intense burns roughly **5–6× a typical day's consumption on each tool simultaneously**, which is how 150 dispositions/hour becomes possible.

The interesting parts aren't the numbers. They're the patterns that emerged for *staying ahead of* a queue that's being pushed to faster than any single reviewer can absorb.

---

## Pattern 1: Let the spray, catch the mist

Codex's default behavior when pointed at a bug is to generate **1.5–3× more PRs than are needed**. Faced with "scanner false-positives on `#` inside quoted strings," it will produce:

- A version that adds `in_quote` state tracking
- A version that extracts a helper
- A version that changes the regex
- A version that adds word boundaries (bonus scope)
- A version that does all four
- Several near-identical copies of the above

The orchestrator's job isn't to review each. It's to identify the distinct bugs (usually 3–5 inside a 15-PR cluster), pick the cleanest winner per bug, and close the rest with a short message pointing to the winner. This ran about **10× faster** than reading each diff.

Operational shape: a single general-purpose agent given the full PR list plus context on already-merged work, returning a compact `#NNNN | VERDICT | reason` table. The orchestrator then executes the table directly through `gh pr close` / `gh pr edit` / `gh pr merge`.

Example verdict:

```
#4686 | MERGE                | extends is_index_in_rust_literal for c"…" / cr#"…"#
#4695 | CLOSE-DUP of #4686   | same bug, same tests, brittle hash-offset inversion
#4710 | CLOSE-SUPERSEDED     | landed in master via merged #4686
```

After six rounds of this shape across the session, the cost per decision stabilized in the low single-digit cents.

---

## Pattern 2: Hermes folders are tool metadata, not contamination

Early in the session the orchestrator closed several PRs whose only apparent "contamination" was a single `.hermes/conveyor/work-<id>/` directory carrying Codex's own ADR, specs, and agent findings. That was wrong. The user corrected: `.hermes/` is Codex's equivalent of this repo's `.spec/` folders — it's the agent's working-artifact directory and belongs with the PR.

Revised classification rule, which stuck for the rest of the session:

| File shape | Verdict |
|---|---|
| Single work-id under `.hermes/conveyor/work-XXXX/` | **Fine** — tool metadata, let it merge |
| Multiple unrelated work-id dirs in one PR | **Bad** — cross-contamination, close |
| Root-level `adr.md` / `specs.md` / `task_list.md` (NOT under `.hermes/`) | **Bad** — shell-glob escape, close |
| Shell-glob garbage like `%{...}` / `{...}` at repo root | **Bad** — close |
| Duplicate ADR numbering with useful content | Rename, don't close |

Two PRs — `#4525` and `#4480` — were reopened after this correction and merged cleanly. The lesson: the tool's own footprint is not noise.

---

## Pattern 3: Cherry-pick extraction beats re-implementation

Several draft PRs had been closed as "contaminated" earlier (before the rule above was refined), but contained real feature code buried under scope-drift commits. The extraction pattern that worked:

1. Reopen the PR
2. `git log origin/master..HEAD --oneline` on the branch
3. Identify commits touching the feature files (as opposed to commits touching stray ADRs or unrelated tests)
4. `git checkout -b extract/... origin/master`
5. `git cherry-pick <feature-commit-SHAs>`
6. `cargo check` on the affected crate
7. Push, let CI run

This rescued real features from **#4432** (parser `unexpected_comma_expr`), **#4480** (PL701 @INC), **#4485** (async/await LSP — required manual port because the target crate was absorbed post-branch), **#4488** (unreachable code detection), **#4525** (ADR-008), **#4470** + **#4471** (nix + DAP docs).

One caveat: when the target crate has been absorbed or renamed after the branch diverged, cherry-pick becomes **re-implementation** — the agent writes the feature fresh using the branch's tests as spec. That succeeded too (all 728 lib tests + integration tests passed on #4488), but it's worth naming the distinction: extraction through a crate absorption is not salvage, it's re-write guided by the original author's intent.

---

## Pattern 4: Sequential rebase drains merge cascades

When many PRs all touch the same file (ci-hygiene's `crates/perl-ci-hygiene/src/main.rs` attracted 15+ PRs across the session), **each merge invalidates its siblings**. Attempting to batch-merge 10 of them either fails on 8 with CONFLICTING, or merges them in an order that creates structural conflicts.

The approach that worked was a **single builder agent given the full queue in priority order**, told to run a strict loop:

```
for each PR in queue:
  gh pr checkout <N>
  git rebase origin/master
  resolve conflicts preserving this PR's intent
  cargo check -p <crate>
  git push --force-with-lease
  gh pr merge <N> --squash
  next
```

The key is that **the PR is merged before the next PR's rebase happens**, so each subsequent PR sees the fully updated master as its base. One such agent cleared **12 ci-hygiene PRs in ~1 hour of wall-clock**, with every rebase needing real conflict resolution (4 conflicts per PR on average). Attempting this in parallel produces a thrash.

Orchestration cost: one long-running builder agent. CI cost: one ci-gate run per merge × 12. Both were acceptable for the throughput gained.

---

## Pattern 5: Bump verification earlier, accept CI churn

Pre-session, the repo's `merge-gate` (the full `just gates` run — clippy full workspace, test full workspace, API surface check, publish dry-run, tautology detection) was **gated on the `merge-ready` label**. Effect: bulk merges via `gh pr merge --squash` skipped the labeling step and landed PRs with only `PR Smoke` (clippy + test on 2 core crates) and `UX Regression Tests` covering them.

This was insufficient: a PR touching `perl-dap`, `perl-workspace-index`, or `perl-ci-hygiene` got zero clippy + zero test coverage per PR. The fix (`#4677`) was to drop the label condition entirely — `merge-gate` now runs on every PR push. The concurrency group absorbs churn from rapid pushes (new pushes cancel in-flight runs).

The follow-up refinement (`#4706`) is tiered scoping driven by PR state:

- **Draft PR** → *scoped-fast*: clippy + test on **changed crates + reverse-dep closure** via `cargo metadata` graph. Fast feedback for Codex and Jules.
- **Ready for review** → *full minus mutation/fuzz*: full workspace clippy + test + publish dry-run + API surface check. Gates the draft→ready transition.
- **Nightly / workflow_dispatch** → *mutation + fuzz* separately.

This shape exploits SRP microcrate boundaries: touching a single leaf crate doesn't re-verify the universe, but every PR gets real coverage on what it actually changed.

---

## Curiosity 1: `gh pr create` picks up the currently-checked-out branch

A revert agent created a branch `revert/4474-vs-marketplace-deprecation-notice`, pushed it, then ran `gh pr create`. The resulting PR **opened against the wrong branch** (`ci/run-merge-gate-on-all-prs`, left over from a prior session's work in the same checkout). When squash-merged, it produced an empty commit because the diff against master was already applied. Master kept carrying the bogus deprecation notice until the user noticed by reading the README directly.

Operational rule: **verify `gh pr view <N> --json headRefName` matches the branch you intended** when working in a main checkout where prior agent branches may still be live. Worktree-isolated agents don't have this problem; main-checkout work does.

---

## Curiosity 2: `PR Smoke`'s auto-fix-fmt can't push to master

The `pr-smoke` job runs `cargo xtask fmt` and, if drift is detected, commits and pushes. On PR branches it works — it pushes to the PR branch. On **master push events** (post-merge), it tries to push directly to `master` and hits the branch-protection rule "Changes must be made through a pull request." Job fails. `merge-gate` has `needs: pr-smoke`, so it cascades to skipped.

This silently broke master CI for many commits until the underlying fmt drift in `collapse_edge_cases_tests.rs` was fixed via `#4688`. Fix for the workflow bug itself is tracked as `#4679`: guard the auto-fix step with `if: github.event_name == 'pull_request'`.

---

## Curiosity 3: Codex re-emits the same ensemble after its winner merged

The perlcritic-cache invalidation bug attracted 15 PRs in round 1 (winner `#4564` merged). Hours later, Codex emitted **8 more PRs for the same bug** — having no memory that the fix had landed. All 8 were closed as `CLOSE-SUPERSEDED #4564`.

Same shape occurred for case-insensitive TODO detection, DAP inline-value clamping, workspace-folder URI case-insensitivity, Rust C-string literal handling, and more. About **30% of the session's "close" dispositions** were re-duplicates of already-merged fixes.

There's no silver bullet for this — Codex doesn't know what landed. The orchestrator just has to watch for it, close proactively, and accept the ~30% noise ratio as the cost of generation.

---

## Curiosity 4: `gh search` defaults to 30 items

Throughout the session, the orchestrator was reporting "30+ merged" as the session tally, thinking the GH search index was lagging. It wasn't. `gh search prs --state closed --merged-at ">=…"` defaults to `--limit 30`. The real number was **116 merged**.

Operational rule: always pass `--limit 200` or `--limit 500` to `gh search` when counting session throughput. This applies to issues too (`gh search issues`).

---

## Curiosity 5: Concurrent agents racing on the same fix

While a direct-edit revert agent was working in its worktree, the orchestrator (after the user intervened) took direct action in the main checkout and landed the fix as `#4770`. The revert agent finished ~5 minutes later and opened `#4771` with a functionally identical diff. `#4771` was closed as redundant. Cost: one wasted builder-agent run.

The rule that should have been invoked: **when the user intervenes and the orchestrator takes direct action, `SendMessage` the running agent to stand down** before doing the work yourself. The message was sent too late in this case — after the direct work was already finished.

---

## Economics snapshot

| Side | 5h session | Weekly | Output |
|---|---|---|---|
| **Claude Code (orchestrator + agents, 20× Max)** | ~31% | ~5% | 116 merges, 120 closes, 31 issues filed |
| **Codex Pro (generation)** | ~26% | ~7% | ~150 PRs generated (50% useful, 50% dup/drift) |
| **Master CI** | — | — | Many cancelled runs (concurrency group) + ~100 successful gate runs; green master at session close |

Both sides ran at near-matched 5-hour session intensity (**~26–31% each**) and near-matched weekly share (**~5–7% each**), which is how 150 dispositions/hour became achievable. The balance is worth naming: a session that's only-Claude or only-Codex won't reach these rates; the **matched-intensity parallel burn on both tools** is what makes spray-and-filter work at this scale.

The ratio that matters: **Codex is cheaper per attempt; Claude is cheaper per decision**. The orchestration pattern exploits this asymmetry by letting Codex spray variation attempts and letting Claude filter to the best one.

For the 236 dispositions processed:
- **Codex side**: ~$0.07 per attempt (at 7%/week ÷ ~150 PRs)
- **Claude side**: ~$0.05 per decision (at 5%/week ÷ 236 dispositions)
- **Combined per *landed* merge**: roughly **$0.25–$0.50 amortised**

For comparison, a typical enterprise code-review process runs $50–$200 per PR reviewed by a senior engineer. The agentic stack delivers comparable rigor (full clippy, test, API surface, security audit on every merged PR) at a 100–1000× cost reduction. The quality of decisions doesn't match a senior engineer's individually, but the *composition* of checks — triage + lightweight review + deep review + diff auditor + CI gates — closes the gap substantially on aggregate.

---

## Follow-ups from this session

- `#4675` / `#4706` — Tiered scoped per-PR CI design
- `#4679` — Guard `pr-smoke` auto-fix-fmt on `pull_request` events only
- `#4594` — Restore package-declaration rewrite in `plan_module_rename_edits` (regression from #4554)
- `#4599` — POD regex strictness decision (hover matches indented `=pod`)
- `#4761` — SQL::Abstract LSP support clean re-do (after #4736 got closed as scope-drifted)
- `#4707` — Codex batch winners bundle tracker

---

## The one-line takeaway

**Per-hour dispositions scale with parallel agent capacity. Rebase cascades are the only real throughput ceiling, and sequential rebase-and-merge is the reliable drain. Most of the cost is generation, not review; most of the filter value is triage, not code-read.**
