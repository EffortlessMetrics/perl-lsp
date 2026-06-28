# Queue Convergence Doctrine

Durable rules distilled from the 2026-06 swarm convergence. Each rule carries
a one-line incident citation so the lesson stays attached to its evidence.

---

## Rule 1 — Evidence requires a merge-base check

A claim that a PR is "already merged" requires `git merge-base --is-ancestor`
with pasted output, not a label or comment assertion.

*29-PR false-close incident: PRs were closed as merged based on label state
that did not reflect the actual merge-base of the destination branch.*

## Rule 2 — Consolidation merges diff against merge-time master

When assembling a consolidation PR from a queue of candidates, the diff to
review is against the master branch at the time of merge, not against the base
branch when the consolidation was cut.

*#9918 reverts → #9921/#9922: the consolidation's base had drifted; a guard
that existed at cut-time was absent at merge-time, requiring two follow-on
fixes.*

## Rule 3 — Claim-guarded files need a live guard run before deletion

Before removing code marked as "dead" or "unused", run the guard (dead-code
report, coverage check, or reference scan) and paste the output. A claim that
code is unused is not sufficient.

*#9917 markers: consolidation deleted code that appeared unused at the surface
but was reached through a conditional compilation path the scanner did not see.*

## Rule 4 — Agent worktrees publish via `git push origin HEAD:<branch>`

In a linked worktree, `git push` without an explicit refspec pushes the
worktree's local branch, which may not be the target branch for the PR. Always
spell out the destination.

```bash
git push origin HEAD:docs/convergence-to-release-plan
```

## Rule 5 — Merge the train before closing its constituents

Close constituent issues or PRs only after the train that carries them has
landed on master. Closing before landing produces a misleading queue state and
loses the receipt that the constituent work is actually in the tree.

## Rule 6 — Automation must filter required contexts; advisory failures are invisible

A sequencer or merge-queue bot that checks for "all required checks pass" will
not see advisory jobs marked `continue-on-error: true`. A failing advisory job
does not block merge and does not appear in the required-context list. Document
which jobs are advisory and which are required; do not infer advisory status
from silence.

*Sequencer-v5 bug: a context-filter gap allowed the queue to treat an invisible
advisory failure as a green signal.*

## Rule 7 — Feature-gated tests need parallel `--lib` coverage

When a feature flag gates a module, `cargo test` with the default feature pack
does not exercise that module. Run `cargo test --lib` in parallel to measure
coverage of gated code separately from the default pack.

*#1217, 28.57% coverage delta: a gated module had no coverage reported because
the CI lane only ran the default feature pack.*

## Rule 8 — Reruns lose secrets; verdicts come from local receipts; uploads non-fatal

A CI job that is rerun after secrets rotation may not receive the secrets needed
for upload steps (e.g., Codecov token). The correctness verdict must come from
a local receipt file written before the upload step. Mark the upload step
`continue-on-error: true` so a failed upload does not fail the required check.

*#1230/#1231: Codecov's "Patch 95" required check was failing because the
upload step failure propagated to the verdict job.*

## Rule 9 — Shared self-hosted runners need three guards

On shared self-hosted runners:

1. **Pre-checkout ownership recovery** — `git config --global --add safe.directory`
   or equivalent before any git operation.
2. **Right-sized disk guards** — size the disk-space check to the actual job
   profile (ripr+ jobs need more than small-Rust jobs).
3. **Scoped scratch self-clean** — each job cleans only its own scratch
   directory, not shared state.

*#1196 series: ripr+ jobs were killed at exit 143 (OOM/disk) on runners sized
for smaller jobs; self-clean was removing shared state.*

## Rule 10 — `merge_group.base_ref` is a full ref; strip `refs/heads/`

When a GitHub Actions workflow reads `github.event.merge_group.base_ref` to
derive a branch name, the value includes the `refs/heads/` prefix. Strip it
before passing to `git` commands that expect a bare branch name.

```bash
BASE_BRANCH="${{ github.event.merge_group.base_ref }}"
BASE_BRANCH="${BASE_BRANCH#refs/heads/}"
```

*#1249: the sequencer crashed because `git merge-base refs/heads/main HEAD`
failed where `git merge-base main HEAD` would have succeeded.*

## Rule 11 — `core.longpaths=true` required on Windows clones

Rust crates with long path names (especially insta snapshot files) exceed
Windows's default MAX_PATH limit. Set `core.longpaths=true` in the git config
before cloning or checking out on Windows runners.

```bash
git config --global core.longpaths true
```

---

## Patterns That Worked

### Merge trains

Assembly → compile gate → CI → merge-commit landing → constituents closed with
receipts.

The train receipt must include: candidate list with PR numbers and SHAs,
planned order, executed checks and outcomes, and final verdict. See
[docs/ci/merge-train-protocol.md](../ci/merge-train-protocol.md).

### Merge queue (ALLGREEN)

Entry requires a green head. ALLGREEN policy means any failing required check
blocks the entry from merging. Advisory jobs (`continue-on-error: true`) do
not block. The queue bisects automatically: a broken entry is dropped and the
next entry is attempted.

Sizing: build+merge 3 (matches the 3-PR overlap profile from the train
protocol — small enough that a broken entry does not cascade into a long
bisect chain).
