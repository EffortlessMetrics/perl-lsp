# 2026-05-03 — `Validate Release` vs. Post-Merge Master CI Timing Race

**Lens**: `release-orchestration.yml`'s `Validate Release` step queries combined-status on the just-merged commit, which races with master CI's report on that new SHA. Failed once during v0.13.3 dispatch.

## What failed

Sequence:

1. `#7872` (release-prep for v0.13.3) merged to master at `06fc1443d`.
2. ~1 minute later, `release-orchestration.yml` was dispatched for v0.13.3.
3. `Validate Release` step ran:
   ```bash
   CI_STATE="$(gh api repos/EffortlessMetrics/perl-lsp/commits/${HEAD_SHA}/status --jq '.state')"
   if [ "$CI_STATE" != "success" ]; then
       echo "::error::Commit ${HEAD_SHA} is not in a successful CI state (${CI_STATE})"
       exit 1
   fi
   ```
4. `CI_STATE` returned `pending` because master CI on `06fc1443d` had not finished reporting yet.
5. Validation failed; all downstream jobs (Build, Publish, etc.) were skipped.

The failure was workflow run `25273613531`. The recovery was to wait for master CI to report `success` on `06fc1443d`, then re-dispatch as `25274134830` — which succeeded and produced the v0.13.3 release.

## The race window

Master CI on a squash-merge commit:

- Triggered immediately on push (the merge commit becomes master HEAD).
- Takes 5-15 minutes to fully complete depending on the gate complexity.
- Combined-status (`/commits/$SHA/status`) reports `pending` while *any* required check is still running.

Release dispatch can land in the same minute as the merge. The validation step was hitting combined-status before master CI had a chance to even start reporting.

## What's wrong with the validation logic

The current step is fail-fast on first non-`success`:

```bash
gh api repos/.../commits/$SHA/status --jq '.state' returns one of: pending | success | failure | error
```

`pending` and `failure` are not the same thing. `pending` means "checks are still running, no information yet"; `failure` means "checks ran and at least one failed."

Treating `pending` as a failure conflates "no signal yet" with "negative signal." Under release-dispatch timing pressure, `pending` is the *expected* state, not a failure.

## The fix (workflow-side)

Two options, either viable:

**Option A: poll with timeout.**

```bash
DEADLINE=$(($(date +%s) + 300))  # 5 minute timeout
while [ "$(date +%s)" -lt "$DEADLINE" ]; do
  CI_STATE="$(gh api repos/.../commits/$SHA/status --jq '.state')"
  if [ "$CI_STATE" = "success" ]; then break; fi
  if [ "$CI_STATE" = "failure" ] || [ "$CI_STATE" = "error" ]; then
    echo "::error::CI failed on $SHA: $CI_STATE"
    exit 1
  fi
  sleep 30
done
[ "$CI_STATE" = "success" ] || { echo "::error::CI didn't reach success in 5min on $SHA"; exit 1; }
```

**Option B: inspect check-runs directly, accept "no required checks failed yet" as pass-through.**

Query individual check-runs rather than combined-status, accept the run if no required check has reported `failure` and the dispatch was made within ~5 minutes of merge. This is more permissive but requires defining "required checks" explicitly.

Option A is simpler and matches the existing combined-status semantics. Option B is more flexible but more code.

## Why this is "workflow timing race," not "product failure"

The individual check-runs that compose master CI all eventually reported `success`. The product (parser, LSP, extension) was correct. The *orchestration's expectation about when to query CI state* was wrong.

This is the canonical "releases fail at seams" pattern — see `../articles/RELEASES_FAIL_AT_SEAMS.md`.

## Follow-up worth filing

Issue / PR title:

```
fix(release): validate-release polls combined-status with timeout instead of fail-fast on pending
```

Scope: 5-line change in `.github/workflows/release-orchestration.yml`'s `Validate Release` step. Replace single-shot status query with a poll loop with timeout. Add a regression test if the workflow has any local test surface.

This is one of the two highest-priority release-engineering follow-ups identified in the v0.13.3 receipt.

## Detection signal

Future occurrence: release-orchestration validates and fails within ~30 seconds of merge with `is not in a successful CI state (pending)`. Recovery is to wait for master CI to report and re-dispatch. The same workflow run will succeed on retry without any code change.

If the same workflow fails twice in a row with a *different* CI state (`failure` rather than `pending`), the issue is in master CI, not the validation seam.

## Related

- Forensics: `2026-05-03-chatgpt-claude-protocol-drift.md` (this race was the real cause behind the misquoted GraphQL error)
- Articles: `../articles/RELEASES_FAIL_AT_SEAMS.md`
- Reference: `../reference/FAILURE_CLASSIFICATION.md` (workflow class)
