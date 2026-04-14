---
description: Diff auditor step 1 — review the complete PR diff for coherence and cleanliness
user-invocable: false
---

# Diff Audit: Check

Review the cumulative diff from all agents.

## Steps

1. Get the full diff stat and diff:
   ```bash
   gh pr diff <number> --stat
   gh pr diff <number>
   ```

2. Read the spec:
   ```bash
   gh pr checkout <number>
   cat .spec/*/acceptance.md 2>/dev/null
   ```

3. Check each acceptance criterion against the diff — is it implemented?

4. Search for leftover artifacts:
   ```bash
   git diff origin/master..HEAD | grep -iE "TODO|FIXME|HACK|XXX|dbg!|println!|eprintln!|#\[allow"
   ```

5. Check commit history coherence:
   ```bash
   git log origin/master..HEAD --oneline
   ```

6. Verify tests still pass (catch late-commit regressions):
   ```bash
   cargo test -p <crate>
   ```

7. Check PR metadata:
   ```bash
   gh pr view <number> --json title,isDraft,labels
   ```
