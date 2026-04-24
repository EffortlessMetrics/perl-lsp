---
description: Oppositional planner step 1 — read the issue and understand the proposed approach
user-invocable: false
---

# Oppositional Planner: Read Issue

Read the scout-filed issue and all comments (including research-verifier and
accuracy-scout findings if present). Understand the proposed approach well
enough to argue against it.

## Steps

1. Read the issue:

   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Identify:
   - **The proposed approach** — what does the scout recommend building?
   - **The alternatives considered** — what options did the scout list and reject?
   - **The assumptions** — what does the spec take for granted?
   - **The scope** — how many files, crates, tests does this touch?
   - **The effort estimate** — EASY/MEDIUM/HARD and why

3. Check for related context:
   - Are there research-reviewed comments? Note what was confirmed vs. flagged.
   - Are there accuracy-scout comments? Note any corrections.
   - Are there other issues touching the same files? (`gh issue list --search "path/to/file"`)

## Output

```
Issue #NNN — Oppositional Read

PROPOSED APPROACH: <1-2 sentence summary>
ALTERNATIVES REJECTED: <list with scout's stated reasons>
KEY ASSUMPTIONS: <what the spec takes for granted>
SCOPE: <files/crates touched, estimated LOC>
VERIFIED CLAIMS: <what research-verifier confirmed, if any>
FLAGGED CLAIMS: <what research-verifier flagged, if any>
```
