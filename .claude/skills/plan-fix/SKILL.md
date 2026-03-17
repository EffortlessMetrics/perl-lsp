---
name: plan-fix
description: Turn a discovery into a builder-ready handoff packet. Use when a scout or coordinator has identified a concrete slice and needs exact scope, verification, and routing rather than code changes.
argument-hint: "<slice summary>"
---

# Plan Fix

Convert a finding into a builder-ready handoff. Context: **$ARGUMENTS**

## Goal

Produce or refresh one handoff packet that a fresh worktree worker can execute
without rediscovering the problem.

## Before Writing The Handoff

1. Confirm the slice is not already covered by an open PR, issue, or tracked task
2. Keep the slice to one dominant crate and one verification loop
3. Split the work if it touches multiple unrelated file surfaces or would exceed
   a small PR-sized change

## Handoff Must Include

- one-sentence goal
- why the slice matters now
- exact file surface
- one verification command
- suggested worker or entrypoints to invoke first
- out-of-scope notes and split criteria
- receipt expectations for review

## Suggested Handoff Shape

```markdown
# <slug>

## Goal
<one sentence>

## Why Now
<priority, regression, or trust reason>

## File Surface
- path/to/file.rs

## Verification
`<single command>`

## Entry Points
- /coding-standards
- /parser-fix

## Out Of Scope
- <explicit non-goals>

## Receipt Requirements
- <what the worker must report back>
```

## Task Routing

If task tools are active:

- create or update one task for the slice
- mark the owner lane
- route it to the next coordinator explicitly

Do not write production code in this skill. Stop once the handoff packet is
builder-ready.
