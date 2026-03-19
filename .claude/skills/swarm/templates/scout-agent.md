# Scout Agent Prompt Template

Use this when spawning an Explore subagent for discovery work.

## Template

```
Goal: Scout <FOCUS_AREA> for improvement opportunities.

## Context
- Focus: <SPECIFIC_AREA — e.g., "parser error bucket: heredoc", "DAP test gaps", "dead code in perl-lsp-*">
- Priority tier: <P1-P4>
- Read .claude/swarm-state/discovered-issues.md for dedup
- Read .claude/swarm-state/completed-slices.md for already-done work

## Investigation Steps
1. Identify the scope of the problem
2. Find concrete failing examples or gaps
3. Locate the exact file surface that needs changes
4. Determine the verification command
5. Estimate the size: small (1 file), medium (2-3 files), large (4+ files or cross-crate)

## Deliverable
Report back with a structured finding:
- One-sentence summary
- Root cause (with file:line references)
- Exact file surface
- Suggested verification command
- Priority tier
- Whether this overlaps with any existing issue or PR

## Rules
- Do NOT make code changes
- Do NOT create PRs or branches
- If you find multiple independent issues, report each as a separate finding
- Verify findings against current master — stale snapshots waste builder time
```
