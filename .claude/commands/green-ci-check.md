---
description: Green CI agent step 1 — verify all CI checks pass on current HEAD SHA
user-invocable: false
---

# Green CI: Check

Verify CI is genuinely green on the current HEAD.

## Steps

1. Get current HEAD SHA:
   ```bash
   HEAD_SHA=$(gh pr view <number> --json headRefOid --jq .headRefOid)
   echo "HEAD: $HEAD_SHA"
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", pullNumber:<number>)` → read `.headRefOid` field.

2. Check all CI status checks:
   ```bash
   gh pr checks <number>
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get_check_runs", pullNumber:<number>)` → returns name, status, conclusion, started_at, completed_at per check. Per-PR CI status is also available via `mcp__github__pull_request_read(method:"get_status")`.

3. Verify freshness — checks must be on the current SHA:
   ```bash
   gh api repos/{owner}/{repo}/commits/$HEAD_SHA/check-runs --jq '.check_runs[] | "\(.name) | \(.status) | \(.conclusion) | \(.head_sha[0:8])"'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get_check_runs", pullNumber:<number>)` — same check-run data; cross-reference the `head_sha` field in each result against the HEAD SHA from step 1 to verify freshness. Direct REST call by arbitrary SHA is not available via MCP.

4. Check PR state:
   ```bash
   gh pr view <number> --json isDraft,mergeable,mergeStateStatus --jq '{draft: .isDraft, mergeable: .mergeable, mergeState: .mergeStateStatus}'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", pullNumber:<number>)` → read `.isDraft`, `.mergeable`, `.mergeStateStatus` fields.

5a. Classify cancellations — for each check with `conclusion: cancelled`, extract
    `started_at` and `completed_at` from the check-runs API response:
    - If `started_at == completed_at` (zero-duration) → mark check **INFRA-NOISE**
      (GitHub concurrency-group kill; instantaneous, no work was done).
    - If `completed_at - started_at > 5s` → mark check **DEVELOPER-CANCEL**
      (manual cancel via GitHub UI or API; treat as RED).
    For each check with `conclusion: failure` → mark **RED** (ignore any cancel log
    content; failures are always RED).
    For each check with `conclusion: success` → no change (existing behavior).

5b. **Coverage-check-name guard** — when a coverage-named check (e.g., `Codecov / Patch 95`,
    `Code Coverage Report`) is RED, do NOT assume patch-coverage shortfall before reading
    the job log. Coverage jobs often run the test suite to gather LLVM-cov data. If a test
    fails inside the coverage job, the check fails, but the failure is a TEST failure, not
    a coverage-measurement failure. Before concluding patch % is the issue:
    - Read the full coverage job log.
    - Search for `FAILED`, `ERROR`, `panic`, or `test assertion` keywords.
    - If a test failure is found inside the coverage job, classify the failure as a
      test-correctness issue (not patch-coverage), and route back to pr-responder or builder
      with the note: "Coverage job failed on test failure in `<test_name>`, not patch coverage."
    - If no test failure is found, then diagnose as patch shortfall or tool error.
    Reference: [docs/learnings/2026-06-coverage-job-ran-tests.md](../../../docs/learnings/2026-06-coverage-job-ran-tests.md)

5c. Determine verdict — using classified checks from steps 5a and 5b:
   - All checks SUCCESS/NEUTRAL/INFRA-NOISE on current SHA + not draft + MERGEABLE → **GREEN**
   - Any check RED or DEVELOPER-CANCEL on current SHA → **RED** (list RED + DEVELOPER-CANCEL only; omit INFRA-NOISE from details)
   - Checks green but on old SHA → **STALE**
   - Draft or DIRTY or CONFLICTING → **BLOCKED**
