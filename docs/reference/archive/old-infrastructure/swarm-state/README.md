# Swarm State

Tracked swarm state lives here. These files are committed and survive across
sessions, worktrees, and operators.

Use this directory for durable coordination knowledge:

- `swarm-queue.json` — active overlap tracking
- `completed-slices.md` — dedup log for work that already exists
- `discovered-issues.md` — leads noticed during other slices
- `known-pitfalls.md` — reusable traps and lessons
- `findings.json` — durable control-plane findings and decisions
- `findings.schema.json` — machine-readable contract for `findings.json`

## Which File To Update

- Product or codebase work you noticed outside the current slice:
  `discovered-issues.md`
- Reusable lesson from a failure or bad fix:
  `known-pitfalls.md`
- Branch or slice lifecycle status:
  `completed-slices.md`
- Active overlap / ownership state:
  `swarm-queue.json`
- Durable swarm conclusion about the control plane, roster, workflow, or docs:
  `findings.json`

`findings.json` is not a bug tracker and not a handoff file. It records stable
findings that should change how the repo describes or operates the swarm.

## Findings Contract

Each finding in `findings.json` captures:

- a stable ID
- what kind of finding it is
- whether it is active, landed, or superseded
- the conclusion the repo should follow
- the live surfaces affected
- evidence pointers
- follow-up PRs or notes

Validate the ledger with:

```bash
python3 scripts/validate_swarm_findings.py
```

An empty `findings` array is a valid bootstrap state. Add entries only when the
repo learns a durable swarm-control finding worth carrying forward.
