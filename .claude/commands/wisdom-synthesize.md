---
description: Wisdom step 2 — synthesize patterns and learnings from the trail
user-invocable: false
---

# Wisdom: Synthesize

Look across the trail and find patterns that individual agents couldn't see.

## Questions to ask

1. **Process patterns:**
   - Did the scout's spec hold up, or did the builder have to deviate?
   - Did the plan-reviewer catch something important?
   - Where did the most value come from? Where was waste?
   - How many round-trips happened? Could they have been avoided?

2. **Code patterns:**
   - Is this the same kind of fix we've made before? Is there a pattern?
   - Did this fix reveal a deeper architectural issue?
   - Should a CLAUDE.md be updated with what was learned about this area?

3. **Quality patterns:**
   - Were the tests good? Did they test behavior or implementation?
   - Did the reviews catch real issues or just nits?
   - What edge cases keep coming up?

4. **Efficiency patterns:**
   - What context did the builder need that the scout didn't provide?
   - What did the reviewer check that /verify already confirmed?
   - Where could the pipeline be shorter without losing quality?

## Output

Record in your task:
```
Process insight: <what worked, what didn't>
Code insight: <pattern, architectural observation>
Quality insight: <test quality, review quality>
Efficiency insight: <what could be faster>
Actionable: <specific improvement to make>
```
