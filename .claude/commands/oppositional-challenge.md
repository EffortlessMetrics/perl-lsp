---
description: Oppositional planner step 2 — generate objections, alternatives, and risk flags
user-invocable: false
---

# Oppositional Planner: Challenge

Generate concrete objections and alternatives to the proposed approach.
Every objection must be specific and actionable — vague concerns are noise.

## Challenge categories

Work through each category. Skip categories that don't apply, but try at
least 3 of the 7.

### 1. Rejected alternatives deserve a second look

Re-examine the options the scout dismissed. Were they dismissed for the right
reasons? Could a hybrid work better? Is there an Option D nobody mentioned?

If the scout only proposed one option, that's a red flag — generate at least
one concrete alternative yourself.

### 2. Scope and blast radius

- How many files does this touch? Could the scope be narrower?
- Does this create a follow-up chain? ("After this, we'll also need X, Y, Z")
- Could you get 80% of the value with 20% of the change?
- Check: `grep -r "function_name" --include="*.rs" -l` to count callers/consumers

### 3. Assumptions that could be wrong

List every implicit assumption. For each, state what happens if it's false.
Examples:
- "Assumes workspaces have <100 @INC paths" — what if someone has 500?
- "Assumes perlcritic is installed" — what's the failure mode if it's not?
- "Assumes this function is only called from one place" — grep to verify

### 4. Interaction risks

- What other open PRs touch the same files? (`gh pr list --search "filename"`)
- What issues are in-build for the same crate? (`gh issue list --label in-build`)
- Will this create merge conflicts with parallel work?

### 5. Performance implications

Be specific. "Scanning directories" → how many? How often? Cached?
- Will this run on every keystroke, every save, or once at startup?
- What's the worst-case input? (Largest workspace, most @INC paths, deepest nesting)

### 6. Maintenance burden

- Does this add a new config surface? New test fixtures? New CI gates?
- Who updates this when the upstream spec changes?
- Will this rot if nobody actively maintains it?

### 7. Simpler alternative

Can you propose a meaningfully simpler approach that handles the common case?
Not a strawman — a real option the plan-reviewer should consider.

## Output

```
## Oppositional Review: Issue #NNN

### Objections

O1: <specific objection with evidence>
O2: ...
O3: ...

### Alternatives not considered

A1: <concrete alternative with tradeoffs>
A2: ...

### Risk flags

R1: <interaction risk / performance risk / maintenance risk>
R2: ...

### Verdict

APPROACH IS: [SOUND | QUESTIONABLE | NEEDS RETHINK]
KEY QUESTION FOR PLAN-REVIEWER: <the one question that most needs answering>
```
