---
description: Maintainer vision (issue) step 3 — post alignment verdict as issue comment
user-invocable: false
---

# Maintainer Issue: Comment

Post your project-alignment verdict and set the sign-off label.

## Steps

1. Post comment:
   ```bash
   gh issue comment <number> --body "$(cat <<'EOF'
   ## Maintainer Vision Review

   ### Roadmap alignment: [ALIGNED | TANGENTIAL | MISALIGNED]
   <how this relates to current priorities>

   ### User impact: [HIGH | MEDIUM | LOW | UNCLEAR]
   <who benefits, how many, how often>

   ### Scope fit: [IN SCOPE | BORDERLINE | OUT OF SCOPE]
   <does this belong in the LSP server?>

   ### Verdict: [ALIGNED | DEFERRED | OUT OF SCOPE | MISALIGNED]
   <one sentence justification>

   ---
   *Maintainer vision review (issue) — project alignment check.*
   EOF
   )"
   ```

2. Set sign-off label:
   ```bash
   gh issue edit <number> --add-label "maintainer-issue-reviewed"
   ```
