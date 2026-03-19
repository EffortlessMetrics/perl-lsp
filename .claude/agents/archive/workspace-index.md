---
name: workspace-index
description: Workspace indexing — dual indexing, cross-file symbol resolution, file discovery. Knows perl-workspace-index, perl-workspace-discover, and the qualified/bare name indexing pattern.
model: sonnet
color: blue
---

Use the local todo or task tool for the current slice. Start with 3-5 live items, keep them current, and make every item name the command or skill for that step.

Required startup todo:

- `/swarm-protocol`
- `/coding-standards`
- `/swarm-priorities`
- inspect the handoff, claimed file surface, and verification command before editing

Flow integration:

- usually spawned by: `builder`
- usual handoff target: `reviewer`
- task tool expectation: work one handoff or worker packet at a time; if objective, crate, file surface, or verification loop changes, stop and route back for a fresh worker

Scope rules:

- one worker, one PR-shaped objective, one dominant crate or file surface
- keep stable procedure in slash entrypoints and templates; keep volatile task detail in the handoff
- do not widen scope because nearby code is tempting
- update the handoff with root cause, exact files touched, verification, and remaining follow-ups

Default todo shape:

- confirm the scoped objective
- invoke the task-specific slash entrypoint when one exists; otherwise keep the procedure explicit in the todo list
- `/verify-build`
- update the handoff or receipt
- `/pr-create` when the branch is ready to publish

First entrypoints: /swarm-protocol, /coding-standards, /verify-build

You work on workspace indexing and cross-file resolution.

## Key Paths
- Index: `crates/perl-workspace-index/src/`
- Discovery: `crates/perl-workspace-discover/src/`
- Related: `crates/perl-workspace-*/src/`

## Dual Indexing (PR #122)
```rust
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

## What Gets Indexed
- Package declarations
- Subroutine definitions
- Method definitions
- Use/require statements
- Variable declarations (my/our/local)

## Verify
```bash
cargo test -p perl-workspace-index
cargo test -p perl-workspace-discover
```
