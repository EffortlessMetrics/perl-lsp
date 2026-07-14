# Worktree Protocol

Multi-box worktree safety rules for parallel agent execution.

**Cross-references**: [ADR-0033](../adr/0033-worktree-first-disposable-workers.md) | [CLAUDE.md](../../CLAUDE.md#architecture-patterns) | Issue [#7100](https://github.com/EffortlessMetrics/perl-lsp/issues/7100) (claim/lease protocol)

---

## Why worktrees

Git worktrees give each agent a fully isolated filesystem view of the repository
while sharing the same `.git` object store. Every agent gets its own working
tree, its own branch, and its own `CARGO_TARGET_DIR` — without the overhead of
a separate clone.

The key properties the swarm depends on:

- **Write isolation**: concurrent agents cannot accidentally modify the same file
  at the same time. Each worktree has its own working-tree state.
- **Branch isolation**: each worktree tracks exactly one branch. Pushing from a
  worktree always targets that branch and never clobbers another agent's work.
- **Main-checkout preservation**: the main checkout stays on `master` and
  accumulates no agent edits. Reviewers, ops, and CI always have a clean
  reference point.

ADR-0033 formalizes the decision: all code mutation in swarm mode happens in
disposable workers running in isolated git worktrees. Persistent coordinators
own routing, review, merge control, and system improvement. One PR per
worktree, one worktree per PR, discard when done.

---

## One PR per worktree

**Rule: never reuse a worktree for a second PR.**

A worktree is the filesystem scope of exactly one PR. Once that PR is merged,
closed, or abandoned:

1. Remove the worktree (`git worktree remove --force <path>`).
2. Delete the tracking branch (`git branch -D <branch>`).
3. Prune dangling metadata (`git worktree prune`).

Reusing a worktree for a second PR risks:

- Leftover files from the first PR bleeding into the second diff.
- Branch state mismatch when the worktree branch diverges from the remote.
- Stale `CARGO_TARGET_DIR` artifacts producing phantom test failures.

The worktree-manager script (`scripts/worktree-manager.py`) tracks named slots
to make slot reuse predictable and auditable — but a slot must be fully released
and cleaned before it can be legitimately reallocated.

---

## Main checkout invariants

The main checkout (`master` or the designated session branch) is a coordination
point, not a workspace. Agents must never:

| Prohibited action | Why |
|---|---|
| Edit any file in the main checkout | Leaks into all worktrees via shared git object store; contaminates future clones |
| Run `git stash` anywhere | The stash list is **shared across all worktrees**. `git stash pop` in any worktree can silently restore another agent's uncommitted changes (see [CLAUDE.md Architecture Patterns](../../CLAUDE.md)) |
| Switch the main checkout's branch | Moves the coordination reference point; confuses ops agents watching for `master` state |
| Run `rm -rf .git/worktrees/` | Corrupts the main `.git` metadata for all registered worktrees simultaneously; see the incident documented in project memory `feedback_agent_damaged_main_checkout` |
| Create a local branch literally named `origin/master` | Makes object lookups ambiguous; `git worktree add ... origin/master` fails with "ambiguous object name"; detected by `git branch -a | grep master` |

When an agent needs to discard a change:

```bash
git restore <file>          # discard unstaged changes to a single file
git restore .               # discard all unstaged changes in the worktree
git checkout -- <file>      # alternative (older git)
```

When an agent needs to save work-in-progress before stopping:

```bash
git add -p                  # stage selectively
git commit -m "wip: <description>"
```

Never save WIP with `git stash`.

---

## Worktree creation patterns

### Fresh branch (new work)

```bash
git worktree add -b impl/<issue#>-<slug> \
    .claude/worktrees/agent-<id> \
    origin/master
```

- `-b impl/<issue#>-<slug>` creates a new branch off `origin/master`.
- The path `.claude/worktrees/agent-<id>` keeps worktrees under a single,
  `.gitignore`-friendly directory.

**Windows MAX_PATH warning**: Windows has a 260-character path limit that
`cargo` hits before you expect it. Keep worktree paths short. Prefer
`.claude/worktrees/<slot>` over deeply nested paths.
Confirmed pattern: `feedback_wave1_collapse_gotchas` — collapse waves hit
`os error 206` (path too long) when worktree paths exceeded ~230 characters.
Enable long paths in Git with `git config core.longpaths true` (repo-level).

### Checkout existing branch

When a spec-planner or red-tdd agent has already created a branch:

```bash
git worktree add .claude/worktrees/agent-<id> impl/<issue#>-<slug>
```

This checks out the named branch into a fresh worktree without creating a new branch.

### Target-dir isolation is automatic — do not export CARGO_TARGET_DIR

Cargo's default (unconfigured) `target-dir` resolves to
`<workspace-root>/target`. For a `git worktree` checkout, the workspace root
*is* the worktree's own directory, so every worktree already builds into its
own isolated `<worktree>/target` — no setup step required.

**Precedence trap (issue #3854):** the `CARGO_TARGET_DIR` environment variable
overrides cargo's per-worktree default (env > config > default). If a shell
profile (`~/.bashrc`, `~/.zshrc`) carries a stale `export CARGO_TARGET_DIR=...`
left over from a prior session — or a different worktree/branch — every new
shell in *every* worktree inherits that stale value regardless of which
worktree it's actually in, silently defeating the per-worktree isolation and
causing agents to build against, and read, another agent's stale binaries
(the "stale-binary trap"). This is exactly what a previous version of this
protocol recommended (manually exporting `CARGO_TARGET_DIR` into a
`/tmp/agent-<branch>-target` path) — that convention is retired. Never
`export CARGO_TARGET_DIR` in a persistent shell profile. If you inherit one
from an old profile line, remove it; don't add a new one.

`scripts/agent-preflight.sh` reports whether `CARGO_TARGET_DIR` is already set
in your shell — if it is, that's a red flag to investigate (likely a stale
profile export), not something to leave in place.

---

## Push patterns

### Standard push (worktree branch matches remote branch)

```bash
git push origin HEAD
```

When the local branch name matches the remote tracking branch, this is
sufficient.

### Diverged branch name push

When the worktree branch name differs from the PR branch (e.g., after a
cherry-pick recovery or slot reuse):

```bash
git push origin HEAD:refs/heads/<original-pr-branch>
```

This pushes the local HEAD to the named remote ref without requiring the local
branch to be renamed.

### Force-push with lease (never naked `--force`)

When a force push is required (rebase, amended history):

```bash
git push --force-with-lease=<branch>:<expected-old-sha> origin HEAD
```

Naked `--force` is banned. It silently clobbers concurrent pushes from other
agents or CI runs. Use `--force-with-lease` with an explicit expected SHA so the
push aborts rather than clobbering if the remote moved.

When the tracking ref is ambiguous (a known failure mode after slot reuse):

```bash
git push --force-with-lease origin HEAD:refs/heads/<branch>
```

The `HEAD:refs/heads/<branch>` form bypasses tracking-ref ambiguity.

---

## Complete lifecycle: create, work, push, cleanup

The following is a worked example of a clean worktree lifecycle.

### 1. Create

```bash
# From the main checkout
git worktree add -b impl/7042-rename-provider \
    .claude/worktrees/agent-abc123 \
    origin/master

# Enter the worktree
cd .claude/worktrees/agent-abc123

# Build artifacts are isolated automatically (cargo's default target-dir is
# per-worktree) — do NOT export CARGO_TARGET_DIR.

# Verify isolation before any edit
bash scripts/agent-preflight.sh
```

### 2. Work

```bash
# Edit files, run tests
cargo test -p perl-lsp-rs
cargo xtask fmt
cargo clippy -p perl-lsp-rs

# Commit incrementally (never stash)
git add crates/perl-lsp-rs/src/providers/rename.rs
git commit -m "feat(rename): add qualified-name split for cross-package rename"
```

### 3. Push

```bash
# First push: set upstream tracking
git push -u origin impl/7042-rename-provider

# Subsequent pushes
git push origin HEAD
```

### 4. Cleanup (after PR merges or closes)

```bash
# Return to main checkout
cd /path/to/perl-lsp

# Remove the worktree
git worktree remove --force .claude/worktrees/agent-abc123

# Delete the local branch
git branch -D impl/7042-rename-provider

# Prune dangling metadata
git worktree prune
```

---

## Anti-patterns

### git stash — shared contamination

```bash
# NEVER
git stash
git stash pop
git stash apply

# Instead: discard changes
git restore <file>

# Instead: save WIP
git commit -m "wip: checkpoint before rebasing"
```

The stash list is a single shared list across all worktrees and the main
checkout. `git stash pop` in worktree A can restore changes left by worktree B.
This has caused silent corruption of in-flight agent work. See project memory
`feedback_swarm_worktree_contamination`.

### Nested worktrees inside a worktree

```bash
# NEVER — creates path confusion and main-branch switching side effects
cd .claude/worktrees/agent-abc123
git worktree add nested-sub origin/master
```

When an agent running in a worktree spawns sub-agents that create further
worktrees, path resolution breaks and the original worktree's branch can be
silently switched. See project memory `feedback_nested_worktree_main_switch`.
All worktrees must be created from the main checkout.

### Branch switches in main checkout from agent context

```bash
# NEVER — moves the coordination point for all other agents
git -C /path/to/perl-lsp checkout feature-x
```

Ops agents, CI, and other running agents observe the main checkout as `master`.
Switching it breaks their assumptions and can leave the main checkout on a
feature branch after the agent finishes.

### rm -rf on .git internals

```bash
# NEVER
rm -rf .git/worktrees/
rm -rf .git/worktrees/agent-abc123/

# Instead: use git's own machinery
git worktree remove --force .claude/worktrees/agent-abc123
git worktree prune
```

Directly removing `.git/worktrees/` entries corrupts the git state for all
currently-registered worktrees simultaneously. The 2026-04-25 incident
(`feedback_agent_damaged_main_checkout`) resulted from an agent attempting
"repo recreation" recovery that touched `.git/worktrees/` directly. If the git
state is corrupted, stop and report — do not attempt self-repair.

### Local branch named `origin/master`

```bash
# Creates an ambiguous object name
git checkout -b origin/master   # NEVER

# Detect
git branch -a | grep master

# Fix
git branch -D origin/master
```

A local branch literally named `origin/master` shadows the remote-tracking ref.
Every subsequent `git worktree add ... origin/master` or `git push origin/master`
fails with "ambiguous object name". This blocked an entire wave of worktree
spawns in the 2026-04-25 session. See project memory
`feedback_ambiguous_origin_master_branch`.

### Rebase interactive or `--continue` on Windows worktrees

```bash
# Hangs on Windows worktrees
git rebase -i HEAD~3         # NEVER in a worktree on Windows
git rebase --continue        # Can get stuck in state machine on Windows

# Instead: cherry-pick from a fresh branch
git checkout -b fix/<branch>-rebased origin/master
git cherry-pick <commit-sha>
```

`git rebase --continue` on Windows worktrees can enter a state-machine loop
even with a clean index because the editor invocation path differs from the
interactive TTY expectation. See project memory
`feedback_windows_worktree_rebase_hangs`. Use cherry-pick for topical commits
onto a fresh branch.

---

## Multi-box implications

When multiple machines run agents in parallel against the same GitHub queue, two
agents can independently allocate the same worktree slot or work the same PR.
The local worktree manager (`scripts/worktree-manager.py`) provides slot
allocation with owner tracking:

```bash
# Box A — claim slot before creating worktree
python3 scripts/worktree-manager.py allocate \
    --slot issue-7042 \
    --branch impl/7042-rename-provider \
    --owner builder-box-a

# Box A — release when done
python3 scripts/worktree-manager.py release \
    --slot issue-7042 \
    --owner builder-box-a
```

State is stored in `.ops-perl-lsp/worktree-manager/state.json`. A mismatch
between `--owner` on `release` and the recorded owner is rejected unless
`--force` is set.

For durable, cross-machine coordination, the claim/lease protocol described in
issue [#7100](https://github.com/EffortlessMetrics/perl-lsp/issues/7100) stores
claim records as structured GitHub PR comments. Until that protocol is
implemented, multi-box operators must coordinate slot allocation manually or
restrict each box to a disjoint set of issue numbers.

---

## Stale worktree hygiene

Run at the start of each orchestration session:

```bash
just clean-worktrees      # prune + remove worktrees with no uncommitted changes and no open PR
git worktree prune        # remove references to deleted worktree directories
git worktree list         # verify remaining worktrees
```

The `just clean-worktrees` recipe checks for open PRs before removing a
worktree directory. Never delete a worktree directory that has an open PR
without first confirming the PR has landed or been closed.

Also verify the main checkout is on `master` after pruning:

```bash
git -C /path/to/perl-lsp branch --show-current   # should print "master"
```

If the main checkout has drifted to a feature branch (caused by a nested
worktree spawn bug), restore it:

```bash
git -C /path/to/perl-lsp checkout master
git -C /path/to/perl-lsp branch -D <stale-feature-branch>
```

---

## Checklist for builders

Before starting work:

- [ ] `bash scripts/agent-preflight.sh` passes all 6 checks
- [ ] `CARGO_TARGET_DIR` is NOT exported in your shell (isolation is automatic per-worktree; a leftover export from an old profile line defeats it)
- [ ] Worktree path is short enough for Windows (under 230 characters total)
- [ ] Slot recorded in worktree-manager if multi-box session is active

Before pushing:

- [ ] No staged changes in the main checkout (`git status` from main root is clean)
- [ ] `git log origin/master..HEAD` shows only this PR's commits — no cross-PR bleed
- [ ] Pushing with `HEAD:refs/heads/<branch>` if branch name differs from remote

After PR merges:

- [ ] `git worktree remove --force <path>`
- [ ] `git branch -D <branch>`
- [ ] `git worktree prune`
- [ ] `python3 scripts/worktree-manager.py release --slot <slot> --owner <owner>`
