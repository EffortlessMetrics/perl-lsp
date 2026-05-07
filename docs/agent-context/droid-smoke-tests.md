# Droid Smoke Test Expectations

## Automated Review Smoke Test

When a PR is opened or updated in the same repository, Droid auto-review should:

1. **Trigger on same-repo PRs.** Only PRs where `head.repo.full_name == repository` run auto-review.
2. **Skip drafts.** Auto-review does not run on draft PRs.
3. **Post comments within 2–5 minutes.** Under normal load, Droid posts findings within this window.
4. **Use MiniMax-M2.7 model.** Verify via `review_model: custom:MiniMax-M2.7-0` in job logs.
5. **No raw debug artifacts.** The artifact named `droid-review-debug-<run_id>` should never appear.

Expected artifacts:
- Review comments on the PR
- (Optional) Issue created for critical security findings
- No `droid-review-debug-*` files

## Manual @droid Review

When a trusted contributor (OWNER, MEMBER, or COLLABORATOR) comments `@droid review`:

1. **Only trusted actors can trigger.** Public comments are ignored.
2. **Executes within 1–3 minutes.** Manual review is faster than auto-review.
3. **Uses shallow depth.** Fast, focused review of PR surface.
4. **Contents permission is read-only.** Manual Droid review reads but does not modify files.

## Manual @droid Security

When `@droid security` is commented:

1. **Requires trusted actor.** Same OWNER/MEMBER/COLLABORATOR gate as `@droid review`.
2. **Focuses on security rules.** Checks for injection, secrets exposure, unsafe patterns.
3. **May create issues.** Critical findings are filed as separate issues.
4. **Blocks on high + critical if configured.** perl-lsp config: block on critical, warn on high.

## Scheduled Security Scan

Every Monday at 08:00 UTC:

1. **Scans the repository.** Analyzes committed code for security issues.
2. **Reports in issues.** Creates or updates a tracking issue with findings.
3. **Uses medium severity threshold.** Balances coverage and false-positive rate.
4. **Runs even with no PR activity.** Scheduled scans are independent of PR lifecycle.

## Expected Clean Review Structure

When Droid finds no actionable issues, it posts:

```
No actionable findings emitted.

Inspected surfaces:
- [files and subsystems checked]

Checks performed:
- [analysis steps taken]

Why no comments:
- [brief explanation]

Residual risk:
- [uncovered areas if any]

Validation signal:
  Observed:
    - [test signals]
  Reported:
    - [CI/tool output]
  Not verified:
    - [things Droid cannot check]
```

## Expected Finding Structure

When Droid finds issues, it uses:

```
[P0|P1|P2] Short title

Failure mode:
- What breaks

Fix direction:
- What to change

Validation:
- How to verify the fix works

Confidence:
- Why this is correct
```

## MiniMax Key Validation

After review runs, the MiniMax dashboard should show:
- API calls to `api.minimax.io/anthropic`
- Model: `MiniMax-M2.7`
- Timestamps matching the workflow run time

If no MiniMax calls appear, the BYOK configuration is not loaded correctly.

## Failure Modes and Recovery

| Symptom | Diagnosis | Recovery |
|---------|-----------|----------|
| "Droid could not post comment" (continues-on-error) | Rate limit or network issue | Retry manual `@droid review` |
| No artifact named `droid-review-debug-*` | Expected (this is correct) | N/A — working as designed |
| `droid-review-debug-<run_id>` appears | Bug in safe action or override | Investigate and file issue |
| Review uses wrong model (e.g., gpt-4) | BYOK not configured | Check `$HOME/.factory/settings.local.json` |
| MiniMax calls don't appear in dashboard | Key not passed to action | Verify `MINIMAX_API_KEY` secret exists |
| PR not reviewed despite being opened | Draft PR check or fork detection | Check job logs for `if:` condition |
