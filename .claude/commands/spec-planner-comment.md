---
description: Spec planner step 5 — post the checklist as an issue comment with branch name
user-invocable: false
---

# Spec Planner: Comment

Post the implementation checklist to the issue and note the branch.

## Steps

1. Post comment with branch name and checklist summary:

   ```bash
   gh issue comment <number> --body "$(cat <<'EOF'
   ## Implementation Spec

   **Branch:** `impl/<issue#>-<specslug>`
   **Spec files:** `.spec/<issue-number>/` (checklist.md, acceptance.md, context.md)

   ### Checklist summary

   <numbered steps from checklist.md, condensed to one line each>

   ### Scope boundary

   **IN scope:** <files list>
   **OUT of scope:** everything else

   ### Flags for builder

   <any ambiguities or decisions noted>

   ---
   *Spec planner — implementation roadmap on branch, spec files committed.*
   EOF
   )"
   ```

2. Add label:
   ```bash
   gh issue edit <number> --add-label "spec-reviewed"
   ```
