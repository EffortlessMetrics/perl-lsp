# Orchestration Counterintuitions: Rules That Don't Match Intuition

*Follow-up notes from the 2026-04-22 Codex review session — observations where the obvious rule was wrong, the obvious metric was lying, or the obvious cost structure was upside down.*

Companion to [`CONTINUOUS_REVIEW_PATTERNS.md`](./CONTINUOUS_REVIEW_PATTERNS.md). That one covers patterns that worked; this one covers observations where the orchestrator's *initial instinct* turned out to be wrong in ways that matter.

---

## Counterintuition 1: Bigger PR doesn't mean more complete

When triaging two Codex PRs for the same issue, instinct says "whichever is bigger has done more of the work." Verified wrong repeatedly this session:

| Pair | Smaller | Larger | Winner | Why |
|---|---|---|---|---|
| @INC completion (#4314) | #4390 (3322 lines) | #4472 (4516 lines) | **#4390** | #4472 touches zero implementation files; only tests (and its own body admits 11/11 tests fail at runtime); carries hermes contamination across 5+ unrelated crates |
| PL304 POD lint (#3405) | #4436 (829 lines) | #4439 (5377 lines) | **#4436** | #4439 bundles unrelated hash_slice parser tests, rose_db_object tests, semantic-analyzer symbol.rs changes, and stray top-level `adr.md`/`specs.md`/`%{...}` shell-glob artifacts |
| Ensemble DAP (#4650) | most candidates | #4643 (109 lines) | **#4643** | Exception to the rule: when the LARGER one is the most-focused feature and the others are partial subsets, larger wins |

Refined heuristic: **in Codex ensembles, bigger usually means contamination picked up, not scope improved**. Smaller, single-file-at-a-time implementations with a focused scope are more likely to be the keeper. The exception is when the larger PR is a logical superset of all the smaller ones — that case does occur but is rarer.

---

## Counterintuition 2: ~50% Codex throughput is not a bug

Out of ~150 Codex-generated PRs this session, ~116 landed useful fixes and ~120 were closed. That's a **~50% landing rate** if you count distinct PRs. Intuition says "50% rejected is a failure mode."

It's not. Adjusted analysis:

- Most of the "closes" were **re-attempts at bugs Codex had already fixed earlier in the session** — Codex has no memory of what landed
- Within each cluster, the ~3 distinct bugs each spawned ~5 variant PRs — the 5→1 winnowing *is the pattern*, not a defect
- The 50% landing rate multiplied by the **~$0.07/attempt** generation cost produces a net **cost per merged fix of ~$0.14** — cheaper than any review-with-iteration alternative

The economic shape: **Codex spray + Claude filter** is profitable specifically *because* generation is cheap enough that a 50% filter ratio is acceptable. If Codex were 10× more expensive per attempt, the calculus would flip toward requiring higher first-pass quality. At current prices, spray-and-filter wins.

**Updated figures** from the session close: Codex Pro consumed **~26% of a 5-hour session budget + ~7% of the weekly budget** for the ~150 attempts. Combined with Claude at **~31% of 5-hour session + ~5% weekly**, both tools ran at near-matched intensity. The 5-hour-session slice suggests one session this intense burns **~5–6× a typical day's consumption simultaneously on each tool** — unusual-but-sustainable for focused review pushes, not a steady-state rate.

---

## Counterintuition 3: Context *is* the source of truth

Mid-session the user reminded: *"the full run is in your context history right now."*

The orchestrator's instinct is to re-fetch state from GitHub before drafting a summary. But the session had already put every meaningful fact into the conversation context: counts, PR numbers, triage verdicts, rebase outcomes. Re-fetching costs tokens and risks stale data mid-session.

**Operational rule:** trust the conversation context for within-session state; re-fetch only for cross-session facts or when the user explicitly asks for "current" state. This cuts round-trips substantially for summary-generation and analysis tasks.

---

## Counterintuition 4: Squash-merge content can disagree with PR title

The PR title is a *label*; the squash-merge content is *whatever's on `headRefName` at merge time*. These can disagree in subtle ways:

- **#4769** was titled `Revert "docs: clarify VS Marketplace badge deprecation..."` but its `headRefName` was `ci/run-merge-gate-on-all-prs` (a leftover branch from prior session). The squash-merge commit landed with **the CI-gate work as content**, and the commit message read `ci: run full merge-gate on every PR (closes #4675) (#4769)` — a Frankenstein of #4677's title verbatim + GitHub's auto-appended PR number #4769
- Result: a merge commit whose title said it was a revert, whose content was CI-workflow edits already applied, and whose diff was therefore empty
- Master kept the bogus deprecation notice despite "successfully merging the revert"

**Operational rule:** after `gh pr create`, verify `gh pr view <N> --json headRefName` matches the intended branch before trusting the PR as "opened." Especially when working in a main checkout where prior agent branches may still be live.

This is a rare class of bug — silent drift between PR intent and actual landed content — but it's invisible from the PR list and only detectable by reading the diff or the destination file.

---

## Counterintuition 5: `validate-title` accepts closed-issue refs

`validate-title` checks that the PR title ends with `(#NNNN)` and that the issue exists. **It does NOT require the issue to be open.** This is slightly counterintuitive because a closed issue doesn't look like a legitimate tracking reference.

This matters in practice:
- A PR retitled to reference a closed issue still passes CI
- Reopening an issue for accuracy is optional if the goal is just to unblock merge
- But the *presence* of the ref in the title auto-closes the issue on merge (if it was open) via GitHub's default "closes #NNNN"-like keyword handling

Net effect: you can retitle in bulk without worrying whether each referenced issue is open or closed. The cost is that searching for "what PR closed issue N" on a closed issue may turn up nothing useful.

---

## Counterintuition 6: `gh search` silently truncates at 30

Throughout the session the orchestrator reported merge counts in the 30s and occasionally called out "gh search index lag." The actual count was **116 — 3.7× higher**. The cause: `gh search prs` defaults to `--limit 30` without telling you it capped.

**Operational rule:** any throughput-counting invocation should pass `--limit 200` or `--limit 500`. This applies to:
- `gh search prs --repo X --state closed --merged-at ...`
- `gh search prs --repo X --state closed --closed ...`
- `gh search issues --repo X --created ...`
- `gh search issues --repo X --state closed --closed ...`

A corollary: **don't trust aggregate counts from any pagination-friendly API without explicitly checking the limit.** This is the same class of bug as the infamous `find / | wc -l | less` "only shows first page" phenomenon — except here it silently reports the truncated number as the answer.

---

## Counterintuition 7: Cherry-pick through crate-rename is re-implementation

The cherry-pick extraction pattern (pull feature commits from a stale branch onto current master) works cleanly *when the target files still exist*. When the target crate has been renamed or absorbed since the branch diverged, cherry-pick cannot apply the patches — the destination paths are gone.

In those cases, the extraction agent ends up doing something more like:

1. Read the stale branch's feature commits to extract **intent**
2. Read the stale branch's tests as **specification**
3. Write new code under the current crate layout that passes those tests

That's re-implementation, not salvage. This happened to **#4488** (unreachable code detection) — the original crate `perl-lsp-diagnostics` was absorbed into `perl-lsp-rs-core` during Wave G1b. The agent correctly re-implemented the lint under the new layout. All 728 lib + integration tests pass.

**Important to name this distinction:** if the feature needs to match the original author's implementation exactly (for reviewability, for reproducing a reported bug's fix verbatim), re-implementation through an absorption is *not* the same as salvage. Usually it's fine; rarely it isn't.

---

## Counterintuition 8: Tiered CI is cost-curve optimization, not effort ranking

Naïve CI design: "run more tests, get more confidence." This ignores that the **cost per signal** varies wildly across check types:

| Check | Runtime | Catches | Cost per signal |
|---|---|---|---|
| `cargo fmt --check` | seconds | style drift | $0.001/check, ~0.1% of actual regressions |
| `cargo check -p <crate>` | seconds to a minute | compile errors | $0.01/check, ~30% of actual regressions |
| `cargo clippy -p <crate>` | minutes | lint drift, banned patterns | $0.05/check, ~20% of actual regressions |
| `cargo test --lib` (workspace) | 5-10 min | logic bugs | $0.25/check, ~40% of actual regressions |
| `just gates` (API surface, tautology, publish dry-run, …) | 10-15 min | subtle regressions | $0.50/check, ~8% of actual regressions |
| Mutation testing | 30+ min | weak tests | $2/check, <2% of actual regressions |
| Long fuzz corpus | hours | rare edge cases | $10+/check, <1% of actual regressions |

Given this shape, the optimal strategy isn't "run everything always" — it's **stratify by cost-per-signal**:

- **Draft PR (fast feedback, Jules/Codex polling):** cheap checks only — fmt, scoped clippy/test on changed crates
- **Ready-for-review (draft→ready transition):** moderate checks — full workspace clippy/test, API surface, publish dry-run, tautology
- **Nightly / release:** expensive checks — mutation, long fuzz, full coverage

The tiered model (tracked as [#4706](https://github.com/EffortlessMetrics/perl-lsp/issues/4706)) captures this. It's not a throughput optimization — it's paying-for-signal at the right price point per PR state.

---

## Counterintuition 9: Label-gates are silent-failure-prone

Pre-session, the repo's `merge-gate` was gated on the `merge-ready` label. This *looks* safe — "only merge PRs that have been labelled ready." In practice, the absence of the label silently downgrades the verification to PR Smoke (clippy + test on 2 crates). Anyone who merged via `gh pr merge --squash` without setting the label ran with reduced checks and nobody saw.

**Operational rule:** label-gated CI is a silent-failure vector unless the orchestration *also* enforces the label at merge time. The alternative — always run the heavy gate, cancel on concurrent push — is more expensive on compute but eliminates the silent-failure class.

The session's resolution was to drop the label gate (#4677) and accept CI-minute churn. This matches the user's direction: *"frequent commits churns the CI a bit, but we definitely need it earlier than that."*

---

## Counterintuition 10: No coordination between parallel agents means ADR-0042 collisions

Codex spawns multiple agents on adjacent issues. Each one looks at the `docs/adr/` directory and picks the next unused ADR number. If two agents pick simultaneously, both become "0042."

This session saw **three different PRs each wanting to be ADR-0042**:

- `#4522` tree-sitter-perl-rs PerlLanguage descriptor (closed — brought its own `0042-no-assertion-test-triage-classification.md`)
- `#4495` no-assertion test triage (closed — had TWO files both numbered 0042)
- `#4452` announcement accuracy (merged — its ADR renumbered to 0043)

**Operational rule:** ADR numbering on a branch is inherently racy under parallel agents. Two options:
1. **Assign ADR numbers at PR-open time** based on an authoritative count of `docs/adr/0*.md` on master
2. **Renumber on conflict resolution**, treating ADR files as content (not name) identities

The session used option 2 ad-hoc (renumber when merge conflict). Option 1 would be a cleaner process — enforced by the orchestrator or pre-push hook. Low priority but worth capturing.

---

## Counterintuition 11: "File opened in IDE" is a user signaling channel

Twice this session the user opened a file in their IDE without typing anything:

1. `docs/forensics/2026-04-11-pragma-phase-block-case-study.md` — implicit signal: *"look at this for style when writing the new forensic"*
2. `c:\Users\steven\.claude\settings.json` — ambiguous signal: *"I may tweak CI/hooks/permissions after this"*

The harness surfaces these as system reminders. They're **user communication without words** — a way to direct attention without spending the bandwidth of typing a sentence.

**Operational rule:** treat "file opened in IDE" as a hint worth acknowledging. It's low-signal relative to a typed message, but ignoring it entirely wastes useful bandwidth. The right response is usually one of:
- "I see you opened X — is there something specific there?"
- Silently use the opened file as context for the current task (if relevance is clear)
- Acknowledge the file and proceed (if ambiguous)

Not acknowledgment-required; just worth noticing.

---

## Counterintuition 12: The main-checkout is hostile territory for agents

Worktree-isolated agents are safe — they can create, destroy, or forcibly reset branches in their own worktree without affecting anything else. But when an agent uses the main checkout (intentionally or as a bug), the following contamination patterns occur:

- Agent leaves its branch checked out; next orchestrator command runs on the wrong branch
- Agent commits changes; those changes stay in the main working directory
- Agent's `gh pr create` picks up the currently-checked-out branch instead of the intended branch (the #4769 bug)
- Agent's `git rebase` leaves merge-in-progress state; next command fails with "you need to resolve your current index first"

This session had all four shapes at various points. The main checkout ended up on branch `rebase-4763` (left by a rebase agent), then `fix/remove-vs-marketplace-deprecation-v2` (left by the direct-edit agent).

**Operational rule:**
- **Always use `isolation: "worktree"` for agents that modify anything**
- At session close, **verify `git rev-parse --abbrev-ref HEAD` is `master`** in the main checkout
- If contaminated, `git checkout master` carefully (handles in-progress merges safely) — do NOT use `git reset --hard` as a shortcut

---

## Counterintuition 13: Concurrent agents racing on the same fix produce wasted work

When the user intervenes mid-task and the orchestrator takes direct action, any running agent doing the same work should be told to stand down *immediately* via `SendMessage`. Otherwise the agent completes its task, opens a redundant PR, and its run is wasted.

This session had one such case (`#4771` as a duplicate of direct-edit `#4770`). The `SendMessage` *was* eventually sent — but only after the direct work was already done. By then the agent was ~90% through its run and opened the PR anyway.

**Operational rule:** `SendMessage` before starting the direct work, not after. Treat the agent's work as parallelizable — if you're going to race it, race from the start rather than duplicating at the finish line.

---

## Counterintuition 14: Agent-type economics aren't monotonic with "power"

Intuition: "more powerful agent = better per-dollar outcome." Reality across this session:

| Agent type | Best use | Cost per run | Throughput shape |
|---|---|---|---|
| **general-purpose (triage)** | 10–20-PR classification passes | Low (~$0.05/batch) | Hundreds of dispositions/hour |
| **builder (single PR, worktree)** | Rebase + small code fix | Medium (~$0.30/run) | 1 PR per run but reliable |
| **builder (sequential drain)** | 12+ PRs through a conflict cascade | High (~$1.50 for a long run) | The *only* way to drain a cascade reliably |
| **reviewer-deep** | Correctness pass on substantive feature PRs | Medium (~$0.40/run) | Catches real bugs, rarely wrong |
| **direct orchestrator edit** | Tiny surgical fix | Near-zero | Risky for branch contamination |

The **sequential drain agent** is the most expensive single run in the session but has no substitute — parallel builders on the same file produce thrash, not progress. The **general-purpose triage** is the cheapest per decision but only works on classification-heavy tasks.

Picking the right agent type per job is more important than picking the most-powerful agent. The orchestrator's real job is matching shape-of-work to shape-of-agent.

---

## The meta-counterintuition

The session's biggest observation isn't any single pattern. It's that **the orchestrator's role under continuous-Codex-input is fundamentally classification, not coding**. Ninety-eight percent of the value came from:

1. Deciding which PRs to merge vs. close
2. Picking the right agent to handle the ones that need rebasing
3. Sequencing merges so they don't cascade into conflicts
4. Closing duplicates/supersededs promptly so the queue doesn't grow

The orchestrator wrote **zero lines of production code** all session. It wrote a handful of commit messages, PR descriptions, issue bodies, and these docs. Everything else was classification, routing, and bookkeeping.

If that's the role: **optimize the orchestrator for fast, accurate classification decisions. Everything else delegates.**
