---
name: merge-queue
description: Merge approved PRs in safe batches — checks CI, fixes policy failures, handles conflicts, ratchets corpus after parser merges.
user-invocable: true
argument-hint: "<PR numbers or 'all'> [--dry-run]"
---

# Merge Queue

Merge approved PRs safely in batches. Focus: **$ARGUMENTS**

## Pre-flight

1. Verify master CI is green before starting:
   ```bash
   gh run list --branch master --limit 1 --json status,conclusion
   ```
   If the latest run is not `completed`/`success`, **stop** — do not merge onto a red master.

2. Build the merge list:
   - If `$ARGUMENTS` is `all`: `gh pr list --state open --json number,title,mergeable,reviewDecision --limit 100`
   - Otherwise parse PR numbers from `$ARGUMENTS`
   - Filter to PRs where `mergeable == "MERGEABLE"` and checks are passing
   - Sort by priority: parser fixes first (corpus impact), then infra, then features

## Per-PR Check

For each PR in priority order:

3. Check all CI checks are green:
   ```bash
   gh pr checks <num>
   ```

4. **If `policy_checks` fails** (stale CURRENT_STATUS.md):
   ```bash
   git fetch origin && git checkout <branch>
   python3 scripts/update-current-status.py
   git add docs/project/CURRENT_STATUS.md
   git commit -m "chore: refresh computed metrics"
   git push
   ```
   Then re-check CI. If it still fails, skip this PR.

5. **If merge conflicts exist**:
   ```bash
   git fetch origin
   git checkout <branch>
   git rebase origin/master
   git push --force-with-lease
   ```
   Only rebase when there are actual conflicts. Do NOT rebase clean PRs —
   unnecessary rebases burn CI queue capacity and cancel in-flight runs.

6. **If CI is red for other reasons**: skip this PR and continue to the next.
   Never merge red.

## Batch Merge

7. Merge in batches of 3:
   ```bash
   gh pr merge <num1> --merge
   gh pr merge <num2> --merge
   gh pr merge <num3> --merge
   ```

8. After each batch, wait for master CI to complete:
   ```bash
   gh run list --branch master --limit 1 --json status,conclusion
   ```
   Poll every 30 seconds until `status == "completed"`. If `conclusion != "success"`,
   **stop the queue** — do not merge more PRs onto a red master.

## Post-merge

9. After merging parser fix PRs, ratchet the corpus baseline to lock in gains:
   ```bash
   just cpan-corpus-ratchet
   ```
   If the ratchet updates the manifest, commit and push to master.

10. After all merges complete, report:
    - How many PRs merged successfully
    - How many skipped (and why: red CI, conflicts, not mergeable)
    - Whether corpus ratchet was triggered
    - Current master CI status

## Rules

- **Never merge red.** If any check fails and cannot be fixed, skip the PR.
- **Rebase only for conflicts.** Unnecessary rebases cancel in-flight CI runs
  and waste queue capacity. GitHub handles non-rebased merges fine.
- **Batch size = 3.** Rapid merges cancel each other's CI runs. Three is the
  safe throughput ceiling before cascading cancellations.
- **Corpus ratchet after parser merges.** Parser fixes improve CPAN coverage;
  ratcheting the manifest prevents regressions.
- **`--dry-run` mode**: If `$ARGUMENTS` contains `--dry-run`, report what would
  be merged without actually merging. Useful for pre-flight review.
