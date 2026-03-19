# Swarm State Archaeology
## How The Repo Learned To Remember The Swarm

The `swarm-state` directory is where the repo stopped treating coordination as transient chat memory and started treating it as a committed control-plane ledger.

That shift is visible in both the files and the git history:

- `2026-03-15` `9cc2d3b9a` introduced continuous swarm infrastructure with agent teams
- `2026-03-17` `d9aab31bc` added durable findings tracking with a schema
- `2026-03-17` `37ddcf56d` immediately hardened the ledger by validating empty findings
- later March commits adjusted archive and lineage handling around the same control plane

This is not a bug-tracker directory. It is institutional memory for how the swarm should operate.

---

## What Each File Is For

`README.md` is the contract. It says the files are committed, survive across sessions, and are meant for durable coordination knowledge rather than ephemeral task notes.

`swarm-queue.json` tracks active overlap and ownership. It is the live coordination map: who is touching what, and where work conflicts.

`completed-slices.md` is the dedup log. Scouts check it before inventing more work so the swarm does not rediscover already-finished slices.

`discovered-issues.md` is the spillover lane for observations outside the current slice. The file explicitly says every agent is a passive scout, which makes the whole swarm a discovery surface, not just the formal scout role.

`known-pitfalls.md` is the reusable failure-memory file. It is append-only during swarm operation and captures lessons that should prevent repeated mistakes.

`findings.json` is the durable conclusion ledger. It records stable control-plane findings that should change how the repo describes or operates the swarm.

`findings.schema.json` makes that ledger machine-readable and validates the shape of each finding entry.

---

## The Contract Emerges In March

The history shows a compact escalation from "state files exist" to "state files are governed."

On `2026-03-15`, the swarm infrastructure commit created the new stateful control plane.

On `2026-03-17`, the repo added the schema-backed findings ledger. The file already encodes the repo's current philosophy:

- findings have stable IDs
- findings have kinds and lifecycle status
- findings need evidence pointers
- findings must carry follow-up notes
- an empty findings array is a valid bootstrap state

That last point matters. The repo does not assume that a ledger must be full to be useful. It assumes the ledger must be structurally valid first, then accrete durable conclusions over time.

The committed findings themselves show the same idea in practice:

- active versus landed conclusions are distinguished
- control-plane findings are separated from product bugs
- commands, skills, hooks, roster surfaces, and worktree policy are all treated as things worth remembering

This is why `findings.json` is more than a scratchpad. It is the repo's memory of how to run the swarm.

---

## The Layers Of Memory

The swarm-state files form a stack:

1. `discovered-issues.md` records live observations
2. `known-pitfalls.md` records repeatable lessons from failures
3. `completed-slices.md` records dedup and lifecycle status
4. `findings.json` records stable control-plane conclusions
5. `findings.schema.json` enforces that the conclusion ledger stays valid

That layering is important. It shows the repository separating:

- temporary signals
- reusable lessons
- lifecycle bookkeeping
- durable doctrine

The effect is that the swarm can forget less while still staying bounded.

---

## What The Findings Say About The Repo

The existing findings make the intended use of `swarm-state` explicit:

- the active tracked swarm surface lives under `.claude/agents/`
- commands and skills are a shared slash-entry surface unless frontmatter says otherwise
- the main `.claude/` tree is canonical, while export packs are derived
- hooks and worktree ownership are deliberate control boundaries
- control-plane conclusions should be documented in the ledger, not left in chat

This means the repo is not just preserving incidents. It is preserving rules.

That is the institutional-memory jump: the system is starting to remember not only what happened, but what should be true next time.

---

## Why This Matters For Archaeology

`swarm-state` is the sharpest evidence that the repo has become self-describing.

Earlier history lived in prompts, journals, and branch names. `swarm-state` adds a committed operational memory layer that survives across sessions, worktrees, and operators.

In practical terms, that means future agents can recover:

- what was already attempted
- what failed in a reusable way
- what the current swarm boundaries are
- which control-plane changes are settled doctrine

That makes the repo less dependent on any one session transcript and more resilient as an operating system for parallel work.

---

## Evidence Pointers

- `.claude/swarm-state/README.md`
- `.claude/swarm-state/findings.json`
- `.claude/swarm-state/findings.schema.json`
- `.claude/swarm-state/known-pitfalls.md`
- `.claude/swarm-state/completed-slices.md`
- `.claude/swarm-state/discovered-issues.md`

Key commits:

- `9cc2d3b9a` - continuous swarm infrastructure with agent teams (`2026-03-15`)
- `d9aab31bc` - durable findings schema and ledger (`2026-03-17`)
- `37ddcf56d` - empty-ledger validation hardening (`2026-03-17`)
- `99d2b17f0` - lineage preservation after attempted cleanup (`2026-03-19`)
