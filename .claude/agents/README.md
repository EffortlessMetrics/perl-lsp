# Canonical Agent Roster

This directory is the canonical runtime roster for the repo's Claude swarm.

The live split is:

- persistent coordinators: `scout`, `builder`, `reviewer`, `ops`, `improver`
- reusable workers: `bootstrapper`, `fixer`, `validator`, `pr-responder`,
  `research-web`, `research-docs`, `research-verify`
- specialist workers: tracked domain, review, docs, quality, and infrastructure
  agents cataloged in `AGENT_CATALOG.md`

These files define who owns each lane. Procedures live in skills and commands;
the slash-entrypoint surface is interchangeable unless frontmatter or bundled
resources make a specific skill-only behavior necessary. Deterministic
enforcement lives in hooks and settings. Task-specific context lives in
handoffs, worktrees, receipts, PRs, and queue files.

If a file lives in `.claude/agents/`, it is part of the active tracked swarm
surface. The canonical active inventory is `AGENT_CATALOG.md`.

Agent design rules:

- use the local todo or task tool for the current lane or slice
- start with 3-5 live items and keep them current
- name the command or skill for each todo item
- preload stable startup skills in frontmatter when the lane uses them on
  every run
- retire workers when crate, file surface, branch, or verification loop changes
- keep coordinators thin and push code mutation into disposable workers
- treat receipts and handoffs as durable output; agent transcript is not proof
- catalog every tracked active agent with spawned-by, handoff-to, and
  first-entrypoint metadata in `AGENT_CATALOG.md`

Compatibility donor material lives in
[`.claude/agents-compat/`](../agents-compat/). New prompts, docs, and commands
should reference the canonical names and the tracked catalog in this directory
first.
