# ADR — work-eb418345

## Agent Definition Section Ordering: Todo List Must Be Final

## Status
**Accepted**

## Context

Issue #4382 reports that four agent definition files in `.claude/agents/` have informational sections appearing AFTER their `## Todo list` sections:

| File | Trailing sections after `## Todo list` |
| --- | --- |
| `scout-parser.md` | `## Domain context` |
| `scout-dap.md` | `## Domain context` |
| `accuracy-scout.md` | `## Invocation` |
| `scout-lsp.md` | `## Domain context`, `## Write to think...` |

The swarm architecture (established in issue #4087) states that `## Todo list` must be the final section of any multi-step agent definition. This ensures that when agents read top-to-bottom, they encounter terminal skills (`/scout-report`, `/agent-wrapup`) that are called at the end of the todo list. Without this ordering, agents may exit before completing the pipeline's institutional-memory-retention steps.

The canonical reference agents (`scout.md`, `builder.md`, `reviewer.md`, etc.) all end with `## Todo list` as their last section. These four files are outliers that violate the convention.

## Decision

Enforce that `## Todo list` is the final section in all multi-step agent definition files by moving trailing informational sections (`## Domain context`, `## Invocation`, `## Write to think...`) to appear immediately before `## Todo list`.

Specifically:
1. In `scout-parser.md`: move `## Domain context` before `## Todo list`
2. In `scout-dap.md`: move `## Domain context` before `## Todo list`
3. In `accuracy-scout.md`: move `## Invocation` before `## Todo list`
4. In `scout-lsp.md`: move `## Domain context` and `## Write to think...` before `## Todo list`

No content is modified — only the relative ordering of sections within each file changes.

## Consequences

**Benefits:**
- Agents will always encounter their todo list last when reading top-to-bottom
- Terminal skills (`/scout-report`, `/agent-wrapup`) will always be reached
- Institutional memory compounds across agent cycles per SWARM_ARCHITECTURE.md line 290
- Convention becomes self-reinforcing as the swarm scales

**Tradeoffs:**
- None. This is a pure documentation fix with no code or behavioral changes.

**Risks:**
- Low. This is a one-time mechanical reorder. Agent definitions are rarely edited concurrently.

## Alternatives Considered

1. **Leave as-is**: Reject the fix. This was rejected because the architectural convention is already established in #4087, and leaving the four files incorrect creates ongoing risk that agents will fail to complete terminal steps.

2. **Restructure all agents to match a new template**: More invasive. The existing reference agents already follow the correct pattern — only these 4 outliers need fixing.

3. **Add a CI check to enforce section ordering**: Would be valuable as a follow-on, but is orthogonal to this fix. This ADR addresses the current violations; CI enforcement prevents future regressions.
