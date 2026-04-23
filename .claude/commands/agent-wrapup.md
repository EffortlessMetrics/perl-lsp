---
description: Final step for every agent — retrospective, documentation, and clean handoff
user-invocable: false
---

# Agent Wrapup

Every agent invokes this as their last action before stopping.
Captures what was learned, documents the ending state, and ensures
clean handoff to whoever picks up next.

## Steps

1. **Summarize what you did.** One line maximum:
   ```
   Summary: [Role] completed [action]. [Outcome: N PRs | issues | tests | files].
   ```

2. **Document the ending state.** Use the block for your role:

   **Scouts:**
   - Issue filed: [URL]
   - Verified: [list of exact file:line paths checked]
   - Uncertain: [what wasn't verified, why]
   - Confidence: [high | medium | low]
   - Next step: plan-review recommended [yes | no]

   **Builders:**
   - PR created: [URL]
   - Tests: [N added/modified] (e.g., crate::module::test_name1, crate::module::test_name2)
   - Files changed: [exact list: crate/src/file.rs, ...]
   - Tests passing: [cargo test result summary]
   - Next step: [ready for review | blocked on X]

   **Reviewers:**
   - PR improved: [URL]
   - Changes made: [exact count of suggestions applied]
   - Test artifacts: [list of files touched during review]
   - Status: [approved | needs revision]
   - Next step: [ready to merge | ping author]

   **Ops:**
   - Merged SHAs: [sha1, sha2, ...] (from PR URLs)
   - PR count: [N PRs in this batch]
   - Queue status: [N PRs still in progress]
   - Master status: [CI passing | blocked]
   - Next step: [merge batch 2 | wait for CI]

   **Plan-reviewers:**
   - Issue reviewed: [URL]
   - Root cause corrected: [yes | no — was unchanged]
   - Gaps filled: [list what you added that wasn't in the scout's spec]
   - Label added: [builder-ready | needs-more-investigation]
   - Next step: [route to builder | route to scout for follow-up]

   **Wisdom:**
   - Trail read: [issue URL + PR URL]
   - Patterns surfaced: [N patterns, e.g. "dispatch table ordering came up in 3 issues"]
   - Memory files updated: [list of .md files written or updated]
   - Follow-up issues filed: [URL list, or NONE]
   - Next step: [close the loop | file follow-up issues]

3. **Retrospective — what did you learn?** This is the most valuable part.
   Write 2-3 sentences about:
   - What was harder or easier than expected?
   - What would you do differently next time?
   - What surprised you about the code or the problem?
   - What context would have helped you work faster?

4. **Breadcrumbs for the next agent.**
   - Logical next: [explicit next issue or task]
   - Related issues: [#123, #456] (group with these)
   - Gotchas: [thing A in crate B behaves like X]
   - Traps: [file Z is generated — don't edit directly]
   - Confidence for next: [high | medium | low]

5. **Update task status.** Mark your tasks as completed with the summary
   from step 1.

6. **File your wrapup on GitHub.** Post your retrospective using the right gh command.
   Use single-quoted heredoc (`<<'WRAPUP_EOF'`) so backticks and `$variables` in your
   summary are not interpreted by the shell:

   - **Scouts:**
     ```bash
     gh issue comment <NUMBER> --body "$(cat <<'WRAPUP_EOF'
     ## Scout Wrapup

     <summary from steps 1-4>
     WRAPUP_EOF
     )"
     ```

   - **Builders:**
     ```bash
     gh pr comment <NUMBER> --body "$(cat <<'WRAPUP_EOF'
     ## Builder Wrapup

     <summary from steps 1-4>
     WRAPUP_EOF
     )"
     ```

   - **Plan-reviewers:**
     ```bash
     gh issue comment <NUMBER> --body "$(cat <<'WRAPUP_EOF'
     ## Plan-Review Wrapup

     <summary from steps 1-4>
     WRAPUP_EOF
     )"
     ```

   - **Reviewers:**
     ```bash
     gh pr comment <NUMBER> --body "$(cat <<'WRAPUP_EOF'
     ## Review Wrapup

     <summary from steps 1-4>
     WRAPUP_EOF
     )"
     ```

   - **Ops:**
     ```bash
     gh pr comment <NUMBER> --body "$(cat <<'WRAPUP_EOF'
     ## Ops Wrapup

     <summary from steps 1-4>
     WRAPUP_EOF
     )"
     ```

   - **Wisdom:**
     ```bash
     gh issue comment <NUMBER> --body "$(cat <<'WRAPUP_EOF'
     ## Wisdom Wrapup

     <summary from steps 1-4>
     WRAPUP_EOF
     )"
     ```

   > If you don't file the update on GitHub, the work is invisible.

## Release the control-plane lock (if you acquired it)

If you acquired the lock during this session, release it now:

```bash
scripts/control-plane-lock.sh release <your-agent-id> || true
```

The `|| true` makes this safe to run regardless of whether you acquired the lock. Always include it as your final bash action before stopping. If your agent crashed mid-edit, the orchestrator can run `scripts/control-plane-lock.sh force-release`; the lock also auto-expires after 30 minutes.

## Where to write this

- **Scouts:** Add retrospective to the issue as a closing comment
- **Builders:** Add retrospective as a comment on the PR
- **Reviewers:** Add retrospective to the review comment
- **Ops:** Add retrospective to a brief merge summary comment
- **Wisdom:** Add retrospective to follow-up issues

## Why this matters

Each agent's retrospective makes the NEXT agent faster. If a scout notes
"the dispatch table in statements.rs is ordered by token kind, not by
frequency — check this first next time," the next scout saves 10 minutes.

These observations compound across cycles. They're the swarm's learning
mechanism.
