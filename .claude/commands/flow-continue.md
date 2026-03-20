---
description: "Flow: continue building on an incomplete PR"
argument-hint: "<pr-number>"
---

# Flow: Continue

A builder started PR **#$ARGUMENTS** but didn't finish. Spawn a
continuation builder to pick up where they left off.

## Steps

1. Check the PR has a "What's next" section:
   ```bash
   gh pr view $ARGUMENTS --json body --jq '.body'
   ```

2. Spawn a builder with the PR context:
   ```
   Agent(
     subagent_type: "builder",
     prompt: "Continue work on PR #$ARGUMENTS. Use /builder-read-pr as step 1 instead of /builder-read-spec. Then /builder-implement, /verify, update the PR, /agent-wrapup.",
     isolation: "worktree",
     model: "sonnet",
     name: "builder-continue-$ARGUMENTS"
   )
   ```

3. When done → chain to /flow-review on the PR.

## When to use this

- Builder created a draft PR with "here's what I got done, here's what's left"
- Review found issues and sent back for a second pass
- A PR has been open and needs someone to finish it
