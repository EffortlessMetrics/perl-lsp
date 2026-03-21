---
description: Builder step 1 — read the spec, then build
user-invocable: false
---

# Builder Read Spec

Read the issue and figure out what to build. Be proactive and fix forward.

## Steps

1. Read the issue:

   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comment_count: (.comments | length)}'
   ```

2. If there are comments (scout reports, plan-review feedback), read them too:

   ```bash
   gh issue view <number> --json comments --jq '.comments[-3:][].body'
   ```

3. Check for plan-review signal:
   - Has a `builder-ready` label? → **Proceed to build.**
   - Has plan-review comments on the issue? → **Proceed to build** using those comments as your spec.
   - No `builder-ready` label AND no plan-review comments? → **Route to plan-reviewer first** unless the task is obviously simple (bug fix with clear repro, test addition, doc change, one-file tweak).

4. If routing to plan-reviewer:
   - Add `needs-plan-review` label: `gh issue edit <number> --add-label needs-plan-review`
   - Report back: "Recommend plan-reviewer for #NNN"
   - STOP — let the pipeline do its job

5. If proceeding to build, claim the issue immediately to prevent double-assignment:
   ```bash
   gh issue edit <number> --add-label "in-build" --remove-label "builder-ready"
   ```
   The `in-build` label tells the orchestrator this issue is taken. The `--remove-label "builder-ready"` removes it from the builder queue. (`--remove-label` is a no-op if the label is absent, so this is always safe.)

   Note: this label is informational, not a mutex. If two builders race before either sets `in-build`, both may proceed. The orchestrator should check `in-build` issues before spawning new builders to detect this condition.

   After claiming, write a version-bound receipt:
   ```
   /label-receipt-write issue <number> in-build builder
   ```

6. Fill any spec gaps yourself:
   - **File:line** — if not provided, use Grep/Glob to find the right files
   - **Change** — if vague, read the code and figure out the right approach
   - **Test code** — if not provided, write your own based on the description
   - **Verify command** — default to `cargo fmt && cargo clippy -p <crate> --tests && cargo test -p <crate>`

7. If you get stuck mid-implementation on an architectural question:
   - Post your **specific questions** as a comment on the GitHub issue
   - Remove `in-build` and add `needs-plan-review` so the orchestrator sees a clean state:
     ```bash
     gh issue edit <number> --remove-label "in-build" --add-label "needs-plan-review"
     ```
   - Report back: "Recommend plan-reviewer for #NNN — questions posted on issue"

   Removing `in-build` here is important: the issue is no longer being actively built,
   and leaving it would cause the orchestrator to treat it as a stalled builder.

## Output

```text
Spec assessment:
  Plan-reviewed: yes/no
  File: <path:line or "researched: ...">
  Change: <one sentence>
  Test: <function name or "will write">
  Verify: <command>
  Decision: proceeding | routing to plan-reviewer
```
