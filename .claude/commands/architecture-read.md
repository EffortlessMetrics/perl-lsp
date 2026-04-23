---
description: Architecture reviewer step 1 — read the issue and understand proposed structural changes
user-invocable: false
---

# Architecture: Read

Read the issue to understand what structural changes are proposed.

## Steps

1. Read the issue:
   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Identify structural changes:
   - New crates being created?
   - New dependencies being added? Between which crates?
   - New public types or traits?
   - New feature flags?
   - Changes to `features.toml`?

3. Check the dependency graph around affected crates:
   ```bash
   cargo tree -p <crate> --depth 1
   cargo tree -p <crate> -i --depth 1  # inverse — who depends on this?
   ```

4. Read `features.toml` if LSP features are involved.
