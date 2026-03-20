---
description: "Flow: implement a GitHub issue in an isolated worktree"
argument-hint: "<issue-number>"
---

# Flow: Build

Implement issue **#$ARGUMENTS** and create a draft PR.

## Steps

1. Read the issue to check if it's been plan-reviewed:
   ```bash
   gh issue view $ARGUMENTS --json labels --jq '[.labels[].name] | if index("builder-ready") then "READY" else "NEEDS PLAN REVIEW" end'
   ```
   If not labeled `builder-ready` → spawn plan-reviewer first:
   ```
   Agent(
     subagent_type: "plan-reviewer",
     prompt: "Review plan for issue #$ARGUMENTS. Follow your todo list.",
     model: "sonnet",
     name: "plan-review-$ARGUMENTS"
   )
   ```
   Wait for plan-reviewer to add the `builder-ready` label.

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
