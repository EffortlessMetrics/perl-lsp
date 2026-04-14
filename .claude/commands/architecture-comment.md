---
description: Architecture reviewer step 3 — post alignment findings as issue comment
user-invocable: false
---

# Architecture: Comment

Post your architectural alignment findings and set the sign-off label.

## Steps

1. Post comment:
   ```bash
   gh issue comment <number> --body "$(cat <<'EOF'
   ## Architecture Review

   ### Dependency direction: [OK | VIOLATION]
   <details or "No new cross-layer dependencies">

   ### Crate boundaries: [OK | CONCERN]
   <details or "Changes scoped to appropriate crates">

   ### Type placement: [OK | MISPLACED]
   <details>

   ### Pattern consistency: [OK | NEW PATTERN]
   <details — if new pattern, is it justified?>

   ### Feature catalog: [OK | MISSING | N/A]
   <details>

   ### Verdict: [ALIGNED | CONCERN | MISALIGNED]
   <one sentence summary>

   ---
   *Architecture reviewer — structural alignment check.*
   EOF
   )"
   ```

2. Set sign-off label:
   ```bash
   gh issue edit <number> --add-label "architecture-reviewed"
   ```
