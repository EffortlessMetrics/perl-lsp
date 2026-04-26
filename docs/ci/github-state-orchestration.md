# GitHub-state orchestration

This document defines the control-plane contract for disconnected maintainership:

- GitHub-visible state is the coordination bus.
- Agents claim bounded work by emitting `agent_lease` records.
- Agents emit `agent_receipt` records as durable evidence.
- Reconciliation projects labels/routes from canonical state; agents do not mutate labels directly.

## Commands

- `cargo xtask queue snapshot --out target/queue/open-prs.json`
- `cargo xtask queue snapshot --fixture <fixture.json> --out target/queue/open-prs.json`
- `cargo xtask agent lease acquire --task <task.json> --out <lease.json>`
- `cargo xtask agent lease verify --lease <lease.json> --current <snapshot.json>`
- `cargo xtask agent receipt validate --receipt <receipt.json>`

## Constraints

These primitives do not:

- merge pull requests,
- apply labels,
- push branches,
- coordinate through local worktree leases.
