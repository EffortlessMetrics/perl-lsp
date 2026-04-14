---
description: Spec planner step 1 — read the issue, plan-review, and verification comments
user-invocable: false
---

# Spec Planner: Read

Read the issue and all pipeline comments to understand what needs to be built.

## Steps

1. Read the issue and comments:
   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Identify from the trail:
   - **Plan-reviewer spec** — the builder-ready specification (look for "builder-ready" label)
   - **Research verification** — any corrected claims or confirmed facts
   - **Oppositional review** — objections and alternatives to consider
   - **Diaboli verdict** — BUILD/DEFER/CLOSE decision
   - **Acceptance criteria** — what "done" looks like

3. Read the target files to verify current state:
   - For each file path in the spec, `read` it to confirm it exists and matches the spec's description
   - Note any drift (line numbers shifted, functions renamed, etc.)

## Output

Produce a mental model of:
- What crate(s) change
- What the acceptance criteria are
- What the plan-reviewer's spec says to do
- What objections were raised and resolved
- What the current state of the target files is
