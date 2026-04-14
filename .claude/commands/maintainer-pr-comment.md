---
description: Maintainer vision (PR) step 3 — post alignment verdict as PR comment
user-invocable: false
---

# Maintainer PR: Comment

Post your project-fit verdict on the PR and set the sign-off label.

## Steps

1. Post comment on the PR:
   ```bash
   gh pr comment <number> --body "$(cat <<'EOF'
   ## Maintainer Vision Review (PR)

   ### Scope discipline: [CLEAN | DRIFT]
   <matches spec or list extra files>

   ### Pattern introduction: [NONE | JUSTIFIED | CONCERN]
   <new patterns and whether they're warranted>

   ### Quality bar: [MET | GAP]
   <test quality, documentation, consistency>

   ### Verdict: [ALIGNED | SCOPE DRIFT | PATTERN CONCERN | QUALITY GAP]
   <one sentence>

   ---
   *Maintainer vision review (PR) — project fit check before deep review.*
   EOF
   )"
   ```

2. Set sign-off label on the PR:
   ```bash
   gh pr edit <number> --add-label "maintainer-pr-reviewed"
   ```
