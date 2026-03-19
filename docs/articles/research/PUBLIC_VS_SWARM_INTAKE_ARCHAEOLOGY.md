# Public vs Swarm Intake Archaeology

## Question

What is visible at the public GitHub boundary, and what only exists once the
swarm moves the work into committed control-plane surfaces?

## Short Answer

The repo exposes a deliberately small public intake surface, but the swarm
internally splits that intake into separate memory and control layers.

Public GitHub captures the handoff event:

- issue form submission
- PR template metadata
- labels and titles
- issue/PR discussion

The swarm captures the operating knowledge:

- dedup state
- discovered leads
- reusable pitfalls
- durable findings
- active queue ownership

That asymmetry is the point. Public GitHub is where work arrives. The swarm
state is where work becomes reusable knowledge.

## 1. The Public Surface Is Structured, But Thin

The discovered-issue entry point is
[`.github/ISSUE_TEMPLATE/swarm_discovered.yml`](../../../.github/ISSUE_TEMPLATE/swarm_discovered.yml).
It is public-facing and intentionally resumable, but it stays narrow:

- `Discovering Agent`
- `Context`
- `Relevant Files`
- `Suggested Approach`
- `Category`

That is five fields plus a label. It records enough context for another agent to
pick up the issue, but it does not try to be the full swarm memory model.

The PR side is similarly thin.
[`.github/PULL_REQUEST_TEMPLATE.md`](../../../.github/PULL_REQUEST_TEMPLATE.md)
has four sections:

- `Summary`
- `Changes`
- `Verification`
- `Agent`

That is enough to publish a PR-shaped result, not enough to store the swarm's
own operating conclusions.

## 2. The Swarm Surface Is Split By Job

The tracked state lives in
[`.claude/swarm-state/README.md`](../../../.claude/swarm-state/README.md),
which explicitly says the directory survives across sessions, worktrees, and
operators.

The files are job-specific:

- [`swarm-queue.json`](../../../.claude/swarm-state/swarm-queue.json) tracks
  active overlap and ownership
- [`completed-slices.md`](../../../.claude/swarm-state/completed-slices.md) is
  a dedup log
- [`discovered-issues.md`](../../../.claude/swarm-state/discovered-issues.md)
  stores leads noticed outside the current slice
- [`known-pitfalls.md`](../../../.claude/swarm-state/known-pitfalls.md) stores
  reusable failure lessons
- [`findings.json`](../../../.claude/swarm-state/findings.json) stores durable
  control-plane conclusions

That split matters more than the labels do. The public issue form has one entry
shape. The swarm state has several distinct memory types.

## 3. The Ledger Is Smaller Than The Story

The machine-readable findings ledger is intentionally compact.
[`.claude/swarm-state/findings.json`](../../../.claude/swarm-state/findings.json)
contains 6 findings, and its schema
([`findings.schema.json`](../../../.claude/swarm-state/findings.schema.json))
forces each finding to record:

- an ID
- a kind
- a status
- a recorded date
- a summary
- a decision
- affected surfaces
- evidence
- follow-up

That is a real operating ledger, not a general-purpose issue tracker.

By contrast, the append-only text files are still basically bootstrap stubs:

- [`discovered-issues.md`](../../../.claude/swarm-state/discovered-issues.md)
- [`known-pitfalls.md`](../../../.claude/swarm-state/known-pitfalls.md)
- [`completed-slices.md`](../../../.claude/swarm-state/completed-slices.md)

They define the formats, but they do not yet carry much historical payload.

## 4. The Repo Explicitly Treats The Export Bundle As Derived

The asymmetry is also documented in the control-plane docs themselves.

[`.claude/README.md`](../../../.claude/README.md) says the canonical runtime
surfaces are `.claude/agents/`, `.claude/skills/`, `.claude/commands/`,
`.claude/settings.json`, and `.claude/swarm-state/`.

[`docs/handoff/agent-swarm-workflow/README.md`](../../../docs/handoff/agent-swarm-workflow/README.md)
goes further and calls the portable bundle a historical/exportable pattern, not
the primary source of truth for this repo.

[`docs/handoff/SWARM_DESIGN.md`](../../../docs/handoff/SWARM_DESIGN.md)
describes the same shape more directly:

- tracked swarm state is committed and persistent
- GitHub issues are a permanent searchable backlog
- handoffs are volatile execution state
- the export bundle is derived

That means the repo is not trying to keep one giant notebook. It is separating
public intake, volatile execution, and durable memory by design.

## 5. Why The Asymmetry Matters

The public side is optimized for:

- low-friction filing
- resumable context
- human readability
- GitHub-native discoverability

The swarm side is optimized for:

- deduplication
- overlap control
- reusable lessons
- durable conclusions
- machine-readable orchestration

So the asymmetry is not a gap. It is a division of labor.

The public issue is the visible event. The swarm-state files are the compound
memory that makes the next event cheaper.

## Strongest Evidence-Backed Claims

1. The public intake surface is intentionally small and structured, not a full
   operating ledger.
2. The swarm state splits memory into five different jobs instead of one blob.
3. `findings.json` is a real durable-conclusion ledger, and it currently holds
   6 entries.
4. The append-only text files define formats first and remain lightly populated.
5. The repo explicitly treats the `.claude/` runtime as canonical and the
   `docs/handoff/swarm-pack/` export as derived.

## See Also

- [SIGNAL_INTAKE_ARCHAEOLOGY.md](SIGNAL_INTAKE_ARCHAEOLOGY.md)
- [SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](CONTROL_PLANE_ARCHAEOLOGY.md)
- [SWARM_STATE_ARCHAEOLOGY.md](SWARM_STATE_ARCHAEOLOGY.md)
