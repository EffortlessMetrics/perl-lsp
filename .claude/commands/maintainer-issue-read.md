---
description: Maintainer vision (issue) step 1 — read the issue, roadmap, and current priorities
user-invocable: false
---

# Maintainer Issue: Read

Understand what's proposed and how it relates to the project's direction.

## Steps

1. Read the issue:
   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Read current priorities:
   ```bash
   cat docs/project/ROADMAP.md
   cat docs/project/status/index.md
   ```

3. Check `features.toml` for feature coverage context.

4. Check what's currently queued:
   ```bash
   gh issue list --label "builder-ready" --state open --limit 10
   ```
