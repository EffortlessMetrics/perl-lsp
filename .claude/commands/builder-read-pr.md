---
description: Builder step 1 (continuation) — read an existing PR to understand what's done and what's left
user-invocable: false
---

# Builder Read PR (Continuation)

A previous builder started this PR but didn't finish. Read the existing
work and understand what's left.

## Steps

1. Read the PR description and diff:
   ```bash
   gh pr view <number> --json title,body --jq '{title: .title, body: .body}'
   gh pr diff <number>
   ```

2. Find the "What's next" section in the PR description.
   This is your spec for what to continue.

3. Check what tests exist — do they pass?
   ```bash
   gh pr checkout <number>
   cargo test -p <crate> 2>&1 | tail -10
   ```

4. Identify:
   - **What's already done** — which files were changed, what tests pass
   - **What's left** — from the PR description or review comments
   - **What's broken** — any failing tests or lint issues

## Output

Record in your task:
```
PR: #NNN
Already done: <summary of existing changes>
What's left: <from PR description>
Current state: tests PASS/FAIL, lint CLEAN/DIRTY
```

Then continue to `/builder-implement` with the "what's left" as your spec.
