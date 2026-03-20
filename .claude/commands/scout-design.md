---
description: Scout step 5 — design 2-3 fix options with tradeoffs
user-invocable: false
---

# Scout Design

Now that you know the root cause, design the fix.

## Steps

1. **Option A** — The simplest fix. What's the minimal change?
   - Which file:line to change
   - What the change looks like (pseudocode or description)
   - Tradeoff: fast but maybe incomplete?
   - Effort: EASY / MEDIUM / HARD

2. **Option B** — A more thorough fix. What's the right fix?
   - Which file:line to change
   - What the change looks like
   - Tradeoff: more work but handles edge cases?
   - Effort: EASY / MEDIUM / HARD

3. **Pick a recommendation** — Which option and why?
   - Consider: effort, risk, completeness, future-proofing
   - Default to the simplest fix that handles the common case
   - **Be honest about confidence.** "I believe Option A is right because..." is better than asserting certainty. The plan-reviewer will verify and improve.

## Output

Record in your task:
```
Option A: <change X at file:line> — EASY, handles 80% of cases
Option B: <change Y at file:line> — MEDIUM, handles 95% of cases
Recommendation: Option A because <reason>
```
