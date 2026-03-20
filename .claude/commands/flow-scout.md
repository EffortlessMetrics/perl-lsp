---
description: "Flow: investigate a topic and file a builder-ready issue"
argument-hint: "<topic> e.g. 'parser unclosed_paren bucket', 'lsp inlay hints', 'perf indexing'"
---

# Flow: Scout

Investigate **$ARGUMENTS** and produce a builder-ready GitHub issue.

## Steps

1. Pick the right scout variant based on the topic:
   - Parser/corpus → `scout-parser`
   - LSP features → `scout-lsp`
   - DAP → `scout-dap`
   - Other → `scout`

2. Spawn the scout agent:
   ```
   Agent(
     subagent_type: "<scout-variant>",
     prompt: "Investigate: $ARGUMENTS. Follow your todo list.",
     model: "sonnet",
     name: "scout-<short-name>"
   )
   ```

3. The scout follows its 7-step todo and files a GitHub issue.
   Return the issue URL when done.

## What a successful flow produces

A GitHub issue with:
- File:line locations
- Root cause in one sentence
- 2-3 fix options with recommendation
- Test code (actual Rust, not description)
- Verify command

If any of these are missing, the scout didn't finish.
