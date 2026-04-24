---
description: Advocatus diaboli step 1 — read the issue and understand what's proposed
user-invocable: false
---

# Advocatus Diaboli: Read Issue

Read the issue to understand what's being proposed. Your goal is to
understand the *what* and *why* well enough to argue against *whether*.

## Steps

1. Read the issue:

   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Identify:
   - **What user problem does this solve?** (If no clear user problem, that's your first objection)
   - **How many users would benefit?** (All? Power users? Only the maintainer?)
   - **What's the maintenance surface?** (New config? New tests? New CI gates? New docs?)
   - **Where does this sit on the roadmap?** Check `docs/project/ROADMAP.md` and `features.toml`
   - **Is this solving a real problem or a theoretical gap?**

3. Quick codebase check:
   - Is there already a partial solution? (`grep` for keywords)
   - Does the ecosystem handle this? (editor, build tool, CPAN module)
   - How much code is this adding vs. the total crate size?

## Output

```
Issue #NNN — Diaboli Read

PROBLEM: <what user problem this claims to solve>
BENEFICIARIES: <who benefits and how many>
MAINTENANCE SURFACE: <what this adds to maintain>
ROADMAP ALIGNMENT: <where this fits — or doesn't>
EXISTING SOLUTIONS: <what already partially or fully handles this>
```
