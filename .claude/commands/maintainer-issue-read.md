---
description: Maintainer vision (issue) step 1 — read the issue, roadmap, and current priorities
user-invocable: false
---

# Maintainer Issue: Read

Understand what's proposed and how it relates to the project's direction.

## Steps

1. Read the issue AND its prior verification comments (accuracy, research, oppositional, diaboli, architecture):
   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```
   You run after the other verifiers. Their comments are inputs to your synthesis, not background noise.

2. **Read the parent tracker if this issue is part of one.** Look in the title/body for `(#NNNN)` or "part of #NNNN" or a named tracker:
   ```bash
   # if issue body references a tracker like #4410:
   gh issue view <tracker-number> --json title,body
   ```
   The parent tracker's commitment IS the roadmap-alignment evidence for wave / collapse / milestone work. A work item that implements decided tracker direction starts at ALIGNED by default — your job becomes checking whether NEW information changes the commitment, not re-running the tracker's original decision.

3. Read current priorities:
   ```bash
   cat docs/project/ROADMAP.md
   cat docs/project/status/index.md
   ```

4. Check `features.toml` for feature coverage context.

5. Check what's currently queued:
   ```bash
   gh issue list --label "builder-ready" --state open --limit 10
   ```
