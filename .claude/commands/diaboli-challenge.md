---
description: Advocatus diaboli step 2 — argue against building this issue
user-invocable: false
---

# Advocatus Diaboli: Challenge

Argue that this issue should NOT be built. Be honest — if it should be
built, say so. But make the case against it first so the plan-reviewer
sees both sides.

## Challenge framework

Work through these questions. Not all apply to every issue.

### 1. Does anyone actually need this?

- Is there evidence of user demand? (GitHub issues, forum posts, user requests)
- Or is this a gap spotted by automated scanning with no user impact?
- Would a Perl developer notice this is missing? In what workflow?

### 2. Is the LSP the right place for this?

- Should the editor handle this? (Folding, formatting, syntax highlighting often belong in the editor)
- Should a build tool handle this? (Dependency detection, compilation, testing)
- Should a CPAN module handle this? (Perl-side analysis, linting via perlcritic)
- Is the LSP duplicating work that already exists elsewhere?

### 3. Is this yak-shaving?

Count the degrees of separation from user value:
- 0: "User sees better completions" — direct value
- 1: "Parser handles X correctly, which improves completions" — one hop
- 2: "Scorecard tracks parser accuracy, which drives parser improvements, which improve completions" — two hops
- 3+: Probably yak-shaving

If >2 hops, challenge explicitly: "This is N degrees from user value. Is
that justified?"

### 4. What's the maintenance cost?

- Does this add code that will rot without active maintenance?
- Does it depend on an external spec or API that changes? (LSP versions, Perl versions, CPAN module APIs)
- How many test fixtures does it add?
- Who will update this in 6 months?

### 5. Is the timing right?

- Check `docs/project/ROADMAP.md` — is this aligned with current priorities?
- Are there prerequisite issues that should land first?
- Would this block or conflict with higher-priority work?
- Is there a release freeze or other constraint?

### 6. What's the opportunity cost?

- Builder time spent here is builder time not spent on something else
- What's the most impactful open `builder-ready` issue right now?
- Is this more important than that?

### 7. Could we do nothing?

- What happens if we never build this? Who suffers?
- Is the current workaround acceptable? (Config option, manual step, third-party tool)
- Does the cost of building outweigh the cost of the gap?

## Output

```
## Advocatus Diaboli Review

### Case against building

<2-5 concrete arguments against, with evidence>

### Strongest counter-argument

<the best reason TO build this — be fair>

### Verdict: [BUILD | DEFER | CLOSE]

<1-2 sentence justification>

If DEFER: <what should come first>
If CLOSE: <specific reason with evidence>
```
