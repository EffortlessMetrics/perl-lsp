# PR Lifecycle Archaeology
## How The Repo Learned To Stage, Merge, Draft, And Dispose

The pull-request ledger tells a different story from the commit graph.

At the time of this snapshot, `gh pr list --state all --limit 2000` returns 1,883 PRs:

- 237 drafts
- 1,108 merged
- 686 closed without merge
- 89 still open

That distribution is the real operating-system clue. The repo is not just "making PRs"; it is using PR state as a lifecycle machine.

---

## 1. Early History Reads Direct

In August and September 2025, the ledger is still mostly direct delivery plus cleanup.

Monthly PR counts from the visible ledger:

- `2025-08`: 66 PRs, 0 drafts, 47 merged, 19 closed
- `2025-09`: 78 PRs, 0 drafts, 65 merged, 13 closed
- `2025-10`: 4 PRs, 0 drafts, 4 merged
- `2025-11`: 13 PRs, 0 drafts, 12 merged, 1 closed
- `2025-12`: 28 PRs, 0 drafts, 28 merged

That pattern matches the older control-plane story in [SWARM_SURFACE_EVOLUTION.md](SWARM_SURFACE_EVOLUTION.md): the repo had branches and PRs, but it had not yet turned draft staging and explicit control-plane surfaces into the default operating model.

The early branch vocabulary is also simpler. The current ledger is dominated by prefixes like `codex`, `test`, `fix`, `feat`, and `docs`, but the older history still reads as mostly product work rather than lifecycle orchestration.

---

## 2. January And February Become Transitional

January 2026 is the first month where draft usage becomes visible at scale:

- `2026-01`: 321 PRs, 16 drafts, 163 merged, 158 closed

February then turns draft-heavy:

- `2026-02`: 284 PRs, 155 drafts, 104 merged, 180 closed

That shift matters more than the raw volume. Draft PRs stop being an edge case and become a staging tool. The repo is starting to use draft state as a deliberate buffer between generation and mergeability.

This is the same era where the `.jules/` and early swarm-lineage material starts showing up in the archaeology notes, so the ledger and the control plane are moving together:

- named lanes for recurring concerns
- more staged worktrees
- more review-and-repair before merge
- more PRs that are created early and stabilized later

The ledger does not show a single "draft revolution" event. It shows a transition from almost-no-drafts to drafts-as-normal.

---

## 3. March Becomes A Burst Engine

March 2026 is the densest month in the ledger:

- `2026-03`: 1,089 PRs, 66 drafts, 685 merged, 315 closed

The same month has several same-day spikes:

- `2026-03-04`: 191 PRs
- `2026-03-12`: 169 PRs
- `2026-03-15`: 93 PRs
- `2026-03-16`: 133 PRs
- `2026-03-18`: 234 PRs
- `2026-03-19`: 96 PRs

Those are not slow, long-lived PRs. They are batch waves.

The lifecycle shape varies by day:

- `2026-03-04` is a merge-heavy release wave
- `2026-03-12` is close-heavy, suggesting disposal and cleanup pressure
- `2026-03-15` and `2026-03-16` are the control-plane turn-on window, with fast merge throughput
- `2026-03-18` is the largest burst day in the visible ledger
- `2026-03-19` swings draft-heavy again, which fits the current docs/article wave and feature staging

This is the strongest evidence that the repo operates in waves, not as a flat stream.

---

## 4. Drafts Mean Staging, Not Hesitation

The current tail is very draft-heavy.

On `2026-03-19`, the visible PR slice includes many draft PRs in a short creation window, including docs articles and feature work. That is not a sign of indecision. It is the repo using draft state as a staging area while review, validation, and merge queue management happen separately.

That interpretation fits the current swarm surfaces documented elsewhere:

- commands for orchestration
- skills for reusable procedures
- hooks for enforcement
- swarm-state for durable memory

In other words, drafts are part of the control plane. They are not just "not ready yet." They are a lane in the process.

---

## 5. Closing A PR Is Often An Explicit Disposition

The closed-without-merge bucket is large for a reason: closure is not the same thing as failure.

The archive shows several closure patterns:

- cleanup or supersession
- orphaned or stale branches
- abandoned experimental routes
- explicit archival of historical lineages

Concrete examples from the ledger:

- `archive/old-main` and `archive/old-local-master` were closed as archive PRs in January 2026
- `pr-orphaned-adr-docs` was closed rather than merged
- `worktree-fix-swarm-skill-orchestrator-scope` was closed after the swarm-control refactor
- `codex/eliminate-panics-handling-j9m79v` and `codex/eliminate-panics-handling-sjrq49` were separate cleanup attempts that were closed rather than merged
- earlier entries such as PR `4` and PR `54` are already in the close-without-merge bucket even in the 2025 history

That means the repo treats disposition as a first-class outcome:

- merge when the slice is correct
- close when the slice is obsolete, duplicated, or no longer worth carrying
- keep the historical signal, but do not keep the branch alive

That is a mature merge discipline, not a failure mode.

---

## 6. Naming Tracks The Lifecycle

The branch-prefix distribution makes the same point from another angle.

Top prefixes in the visible ledger:

- `codex`
- `test`
- `fix`
- `feat`
- `docs`
- `bolt`
- `maint`
- `chore`
- `dependabot`
- `sentinel`
- `refactor`
- `release`
- `palette`
- `wave`
- `wave2`
- `jules`

That list is a historical record in itself.

- `bolt`, `sentinel`, and `palette` preserve the proto-specialist concern lanes from the `.jules/` era
- `wave` and `wave2` mark explicit batch orchestration
- `codex` dominates the archive, which matches the heavy PR-era automation
- `test`, `fix`, `feat`, and `docs` show the repository increasingly operating as a staged delivery system

The naming scheme is not just taxonomy. It is lifecycle metadata.

---

## 7. What This Suggests About The Operating Model

The repo seems to have evolved through three lifecycle modes:

1. Direct delivery with incidental PRs and cleanup.
2. Staged delivery with drafts, waves, review, and explicit closure.
3. Control-plane delivery where commands, skills, hooks, and state files manage the PR lifecycle itself.

The ledger supports that reading better than commit counts alone.

The key transition is not "more PRs."
It is "more deliberate disposition."

That is why the PR archive matters: it shows how the repo learned to decide not just what to build, but what to merge, what to stage, and what to close.

---

## Evidence Pointers

- [SWARM_SURFACE_EVOLUTION.md](SWARM_SURFACE_EVOLUTION.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](CONTROL_PLANE_ARCHAEOLOGY.md)
- [ERA_TIMELINE.md](ERA_TIMELINE.md)
- `gh pr list --state all --limit 2000`
- `gh pr list --state all --limit 2000 --json number,title,createdAt,closedAt,mergedAt,isDraft,headRefName`
