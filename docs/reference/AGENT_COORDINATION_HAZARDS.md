# Agent Coordination Hazards

Four recurring patterns where multiple agents acting in parallel produce friction or wasted work, with detection signals and mitigations.

## 1. Convergent reasoning collision

**Pattern**: Two agents, dispatched independently to similar problems, reason correctly to the same fix and implement it in two different PRs. The fixes are byte-equivalent or semantically identical.

**Example**: 2026-04-30 — A pr-responder fixing #7561 (UX receipt workflow) and a pr-responder fixing #7540 (UX receipt routing) both independently identified that the missing `xtask ux-regression-receipt` command needed to be registered. Both produced PRs (the standalone #7569 and the bundled fix in #7540's branch). After #7569 merged, #7540 rebased to find all 3 of its commits dropped as already-upstream.

**Why it happens**: Agent prompts are scoped to "fix this PR" without visibility into "what other agents are also working on similar issues."

**Detection signals**:
- Two recent PRs touching the same uncommon file
- Multiple agents in flight with overlapping issue references
- PR rebase produces unexpectedly empty diff

**Mitigations**:
- Before dispatching a code-mutating agent, search for other recent PRs touching the same files: `gh pr list --search "in:files <path>" --limit 5`
- For ensemble-shaped problems (one common cause, multiple symptomatic PRs), dispatch one fixer with explicit cross-PR scope rather than per-PR fixers.
- When two fixes converge, prefer the standalone PR; close the bundled-fix PR after rebase reveals its scope drift.

## 2. Premature monitor exit

**Pattern**: An agent dispatched to do work-and-wait arms a Monitor on an event source, emits a short status message, and exits — the orchestrator wakes on the Monitor event but the agent never returns to act on it.

**Example**: 2026-04-30 — A "cascade-update + ci-green wave" agent armed a Monitor on 8 PRs' CI completion, emitted a setup confirmation, and exited. The Monitor fired ~15 events as CI progressed. None of the events resulted in label application or merge action.

**Why it happens**: The agent reads its instructions ("monitor and act"), arms the Monitor, and treats the Monitor's existence as the work being done.

**Detection signals**:
- Agent terminates with very short runtime (<2 minutes) on a multi-PR task
- Agent's final message describes a Monitor that's now armed but doesn't describe the per-event handler logic
- Subsequent Monitor events arrive but no PR labels change

**Mitigations**:
- Prefer synchronous fast-track agents for batch work: "verify scope, apply 11 labels, admin-merge, repeat for each PR."
- If event-watching is genuinely needed, use `Monitor` from the orchestrator directly (not delegated to an agent) so the orchestrator handles each event.
- For agents that *must* watch events, instruct them to not exit until they've handled at least one — and document the per-event action explicitly in the prompt.

## 3. Label-application silent failure

**Pattern**: An agent reports "applied label X" in its verdict but the label doesn't land on the PR. Observed silent-fail rate ~80% on some agent families per memory.

**Detection**: After agent completion, verify directly: `gh pr view <N> --json labels --jq '.labels[].name'`.

**Mitigation**: Build label verification into agent wrap-up. The skills that include `gh pr edit ... --add-label X` followed by `gh pr view <N> --json labels` (verifying X is present) have ~zero silent-fail rate. The skills that don't verify often miss.

## 4. Worktree-share contamination on Windows

**Pattern**: When worktree spawn fails (e.g., Windows MAX_PATH on deep snapshot file paths), agents fall back to sharing the main checkout. Multiple agents performing `gh pr checkout` switch the main checkout's branch out from under each other.

**Example**: 2026-04-30 — A pr-responder for #7561 doing Edit operations had its working tree silently switched to a different feature branch by a concurrent agent's `gh pr checkout`. The first Edit was lost; the agent recovered by staging file contents to a gitignored path (`target/cp7429/`) before each branch switch.

**Detection signals**:
- Edit tool fails with "file not found" on a file that exists at HEAD
- `git status` shows uncommitted changes from a different branch's work
- Reflog shows multiple branch switches in a short window

**Mitigations**:
- Serialize agents that need filesystem state — do not run two `gh pr checkout`-using agents in parallel against the main checkout.
- Parallelize agents that only touch `gh` API. The bulk of fast-track verification work is API-only; reserve the main checkout for code-mutating agents.
- For multi-branch agents (e.g., one cascade-update for N PRs), use `gh pr update-branch` (which doesn't require a local checkout) instead of `gh pr checkout` + push.
- When MAX_PATH is the blocker, enabling Windows long-path support (`git config --system core.longpaths true` + registry change) restores worktree isolation.

## Related

- Memory: `feedback_concurrent_worktree_contamination.md`
- Memory: `feedback_label_skill_silent_failure.md`
- Memory: `feedback_unauthorized_dup_agent_push.md`
- 2026-04-30 forensics: [`docs/forensics/2026-04-30-cascade-day.md`](../forensics/2026-04-30-cascade-day.md)
