# HERMES.md — Hermes Agent workspace instructions

This repository already maintains the canonical implementation-agent guide in
[`AGENTS.md`](AGENTS.md). Hermes Agent should follow that file for workflow,
scoping, verification, and PR conventions.

## Hermes compatibility note

If both files are visible, treat this `HERMES.md` as an entrypoint and use
`AGENTS.md` as the source of truth to avoid instruction drift.

## Quick path

1. Read `AGENTS.md` first.
2. Work one scoped change.
3. Run crate-level verification before opening a PR.
4. Keep PR title format: `type(scope): description (#NNNN)`.
