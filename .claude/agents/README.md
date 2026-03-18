# Agent Definitions (Archived)

Agent definitions have been archived. The orchestrator uses inline prompt
templates and skills (e.g., `/swarm`, `/parser-fix`, `/verify`) instead of
loading agent definition files at runtime.

See `archive/` for historical agent definitions.

## Why archived

The 54 agent definition files in this directory were never loaded by the
orchestrator. Every agent spawn uses an inline prompt constructed from
CLAUDE.md context, skills, and handoff files. Keeping 54 unused files in the
active directory added noise to searches and context windows without
providing value.

## If you need an agent definition

The archived files are still useful as reference for:
- Understanding agent roles and lane ownership
- Seeing which skills each agent type invokes
- Reconstructing the original swarm roster

All files are intact in `archive/`.

Older agent generations (`agents2` through `agents6` and `agents-compat`) have
been removed. Historical tracked agent definitions now live only under
`archive/`; new docs should reference that archived roster instead of the
deleted generations.
