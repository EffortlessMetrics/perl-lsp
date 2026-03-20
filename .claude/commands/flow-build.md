---
description: "Flow: implement a GitHub issue in an isolated worktree"
argument-hint: "<issue-number>"
---

# Flow: Build

Implement issue **#$ARGUMENTS** and create a draft PR.

## Steps

1. Read the issue to verify it's builder-ready:
   ```bash
   gh issue view $ARGUMENTS --json body --jq '.body'
   ```
   Check for: file:line, root cause, test code, verify command.
   If missing any → invoke `/flow-scout` to complete the spec first.

2. Spawn the builder agent in a worktree:
   ```
   Agent(
     subagent_type: "builder",
     prompt: "Implement issue #$ARGUMENTS. Follow your todo list.",
     isolation: "worktree",
     model: "sonnet",
     name: "builder-$ARGUMENTS"
   )
   ```

3. The builder follows its 5-step todo:
   read-spec → write-test → implement → verify → pr-create

4. When builder completes, it returns the PR number.
   Auto-chain: invoke `/flow-review` on the PR.

## What a successful flow produces

A draft PR that:
- Links the issue
- Has a failing test that now passes
- Has a minimal diff matching the spec
- Passes `cargo fmt` and `cargo clippy`
- Documents what changed, what was considered, what's next
