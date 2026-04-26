# Specifications — work-eb418345

## Feature: Reorder Agent Definition Sections So Todo Lists Are Final

## Feature/Behavior Description

Reorder section headings in four agent definition files so that `## Todo list` is the final section. This enforces the swarm architecture convention established in issue #4087: when agents read top-to-bottom, their todo list (and its terminal skills like `/scout-report`, `/agent-wrapup`) must be the last thing they encounter.

## Acceptance Criteria

1. **scout-parser.md**: `grep "^## " .claude/agents/scout-parser.md | tail -1` returns `## Todo list`
2. **scout-dap.md**: `grep "^## " .claude/agents/scout-dap.md | tail -1` returns `## Todo list`
3. **accuracy-scout.md**: `grep "^## " .claude/agents/accuracy-scout.md | tail -1` returns `## Todo list`
4. **scout-lsp.md**: `grep "^## " .claude/agents/scout-lsp.md | tail -1` returns `## Todo list`

## Non-Goals

- This fix does not change any agent charter, principles, domain context, or invocation content — only section ordering
- This fix does not add CI enforcement (deferred to a follow-on work item)
- This fix does not change `lead-*` or `research-web.md` agents, which use `## Rules` instead of `## Todo list` and are intentionally structured differently as routing agents

## Dependencies

- Issue #4382 (the bug report identifying the 4 files)
- Issue #4087 (the prior work establishing the "todo list final" convention)
- Reference agents: `scout.md`, `builder.md`, `reviewer.md`, `plan-reviewer.md`, `ops.md`, `wisdom.md`, `research-verifier.md`, `reviewer-deep.md` (all already correct)

## Files Affected

| File | Sections to move | Destination |
| --- | --- | --- |
| `.claude/agents/scout-parser.md` | `## Domain context` | Before `## Todo list` |
| `.claude/agents/scout-dap.md` | `## Domain context` | Before `## Todo list` |
| `.claude/agents/accuracy-scout.md` | `## Invocation` | Before `## Todo list` |
| `.claude/agents/scout-lsp.md` | `## Domain context` + `## Write to think...` | Before `## Todo list` |
