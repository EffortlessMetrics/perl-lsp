# Canonical Agent Roster

This directory is the canonical runtime roster for the repo's Claude swarm.

The live split is:

- persistent coordinators: `scout`, `builder`, `reviewer`, `ops`, `improver`
- reusable workers: `bootstrapper`, `fixer`, `validator`, `pr-responder`,
  `research-web`, `research-docs`, `research-verify`

These files define who owns each lane. Procedures live in skills and commands.
Deterministic enforcement lives in hooks and settings. Task-specific context
lives in handoffs, worktrees, receipts, PRs, and queue files.

Agent design rules:

- keep a local todo list for the current lane or slice
- name the command or skill for each todo item
- retire workers when crate, file surface, branch, or verification loop changes
- keep coordinators thin and push code mutation into disposable workers
- treat receipts and handoffs as durable output; agent transcript is not proof

The older `swarm-*` files remain as donor material during the transition, but
new prompts, docs, and commands should reference the canonical names in this
directory first.
