---
description: Scout step 4 — trace the root cause of the problem
user-invocable: false
---

# Scout Root Cause

Understand WHY the code fails at the location you found in step 2.

## Steps

1. **Read the failing code path** from step 2's file:line locations
2. **Trace the logic**: What does the code do? Where does it diverge from correct behavior?
3. **Identify the specific point of failure**: Is it a missing branch? Wrong condition? Missing case?
4. **State the root cause in one sentence**

## Good root cause examples

- "parse_phase_block at declarations.rs:845 matches CHECK keyword before checking if next token is Colon, so `CHECK:` labels are treated as phase blocks"
- "parse_prototype at variables.rs:776 doesn't handle `&` sigil in prototype position, causing the prototype to be parsed as an expression"
- "The completion provider at completions.rs:200 returns items without commitCharacters, so editors don't know when to auto-commit"

## Bad root cause examples

- "The parser doesn't handle this correctly" (too vague)
- "Needs investigation" (incomplete — go back to step 2)
- "The code is complex" (not a root cause)

## Output

Record in your task:
```
Root cause: <one sentence naming the function, file:line, and what's wrong>
```
