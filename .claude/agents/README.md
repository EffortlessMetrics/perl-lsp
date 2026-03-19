# Agent Definitions (Archived)

Agent definitions have been archived. The orchestrator uses inline prompt
templates and skills (e.g., `/swarm`, `/parser-fix`, `/verify`) instead of
loading agent definition files at runtime.

See `archive/` for historical agent definitions.

Legacy sibling directories such as `../agents-compat`, `../agents2`,
`../agents3`, `../agents3 - to update`, `../agents4`, `../agents5`, and
`../agents6` are also intentionally preserved as historical reference and
analysis material. They are archived content, not accidental leftovers.

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
