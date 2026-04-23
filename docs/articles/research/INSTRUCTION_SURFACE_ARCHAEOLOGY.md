# Instruction Surface Archaeology
## How Repo Methodology Moved From Prompt Packs To Versioned Operating Doctrine

This note traces a specific pattern in the repository: guidance for humans and
agents did not stay as ad hoc prompts. It was repeatedly rewritten into
committed instruction surfaces that survived sessions, branches, and tool
changes.

The interesting part is the sequence. The repo starts with a direct orchestration
guide, then moves through project docs that define the agentic model, and later
codifies the operating doctrine into `.claude` commands, skills, swarm-state,
and the root `AGENTS.md`.

---

## 1. The First Durable Surface Was A Direct Orchestration Guide

[`.claude/ORCHESTRATION_GUIDE.md`](../../../.claude/ORCHESTRATION_GUIDE.md)
appears in `3341bebdb` as a concrete review-flow guide. It names agents, stages,
and handoffs directly:

- `pr-initial-reviewer`
- `test-runner-analyzer`
- `context-scout`
- `pr-cleanup-agent`
- `pr-finalize-agent`
- `pr-merger`
- `pr-doc-finalize`

The file is still very prompt-like. That is the point. The earliest durable
instruction surface is a written operating script for a specific PR review
flow, not a generalized policy document.

That early guide already shows the core shape of the repo's methodology:

- staged validation
- explicit decision points
- GitHub comments and labels as coordination surfaces
- local verification as the authoritative loop

---

## 2. Project Docs Turned The Method Into A Repo-Wide Model

The next layer is the project documentation.

[`docs/project/AGENTIC_DEV.md`](../../../docs/project/AGENTIC_DEV.md) gives the
shortest stable statement of the operating model:

- AI-assisted means the human writes and the AI suggests
- AI-native means the human reviews and accepts or rejects
- claims are receipt-based, not trust-based
- `just status-check` fails on drift

[`docs/project/AGENTIC_DEVELOPMENT.md`](../../../docs/project/AGENTIC_DEVELOPMENT.md)
expands that into a case study. The earlier version is still visibly
article-shaped, but it does something important: it turns the development
approach into a documented repo identity rather than a transient prompt.

The commit trail shows that this layer was not static:

- `d23eca31c` adds the agentic development history article
- later cross-reference fixes keep the article aligned with the rest of the
  docs
- `a48d2484d` aligns roadmap status and agent guidance, which is a sign that
  the instruction layer had become part of ordinary documentation maintenance

That is the transition from "how to run a flow" to "how this repo works."

---

## 3. `.claude` Became The Runtime Control Plane

The current `.claude` tree is where the instruction surface becomes durable
operations.

[`.claude/README.md`](../../../.claude/README.md) states the runtime shape
explicitly:

- `commands/` are slash entrypoints
- `skills/` are the canonical reusable procedures
- `swarm-state/` is the committed memory layer
- `settings.json` carries permissions and hook enforcement

That same file also preserves the archival lineage of older agent directories,
which matters historically: the repo did not erase prior surfaces. It retained
them as evolution evidence.

The commit history shows the control plane becoming increasingly explicit:

- `9cc2d3b9a` turns on continuous swarm infrastructure
- `1fd8f7e36` adds invocation-control frontmatter to skills
- `d17b84393` codifies the worktree-first control plane
- `31f7854e8` adds core worker skills
- `d9aab31bc` and `37ddcf56d` turn `swarm-state` into a schema-backed ledger
- `5c5816b78` preserves the coordinator model and worker boundaries

This is where prompt text becomes versioned operating doctrine. The repo stops
asking agents to remember the method and starts committing the method itself.

---

## 4. `AGENTS.md` Formalized The General Rules

[`AGENTS.md`](../../../AGENTS.md) is the broadest instruction surface in the
repo. It is not a swarm script. It is the default rulebook for coding agents:

- start from current truth
- use canonical docs before restating project facts
- keep metrics and release lines separated
- validate docs drift when computed status changes
- follow repository-specific coding expectations

That makes `AGENTS.md` historically distinct from the earlier orchestration
guide. The guide tells you how to run a flow. `AGENTS.md` tells you how to work
in the repo at all.

The commit trail shows this becoming part of normal maintenance rather than a
special artifact:

- `25f0b29a5` already ties the project to forensics, casebook, and lessons
- `a48d2484d` aligns roadmap status and agent guidance
- later docs updates keep the guidance synchronized with the repo truth

In other words, the repo eventually treats human-agent procedure the same way
it treats code: versioned, reviewed, and kept in sync with current truth.

---

## 5. Historical Meaning

The instruction surfaces evolve in four steps:

1. a direct orchestration guide for one flow
2. project docs that define the repo's AI-native development model
3. `.claude` commands, skills, swarm-state, and settings as the live control plane
4. `AGENTS.md` as the general-purpose repo contract

That progression is the interesting historical fact. The repo did not just use
prompts to get work done. It kept turning its methodology into committed
surfaces that could outlive the session that created them.

That is what makes this codebase unusual: the instruction layer is part of the
history, not just the tooling.

---

## Evidence Pointers

- [`.claude/ORCHESTRATION_GUIDE.md`](../../../.claude/ORCHESTRATION_GUIDE.md)
- [`.claude/README.md`](../../../.claude/README.md)
- [`AGENTS.md`](../../../AGENTS.md)
- [`docs/project/AGENTIC_DEV.md`](../../../docs/project/AGENTIC_DEV.md)
- [`docs/project/AGENTIC_DEVELOPMENT.md`](../../../docs/project/AGENTIC_DEVELOPMENT.md)
- [`docs/reference/SKILL_AND_AGENT_DESIGN.md`](../../../docs/reference/SKILL_AND_AGENT_DESIGN.md)
- `3341bebdb`, `d23eca31c`, `9cc2d3b9a`, `1fd8f7e36`, `d17b84393`, `31f7854e8`, `d9aab31bc`, `37ddcf56d`, `5c5816b78`, `a48d2484d`
