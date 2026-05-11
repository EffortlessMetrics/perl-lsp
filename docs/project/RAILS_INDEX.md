# Active rails

Each rail is an `@INC`-shaped burndown: existing substrate + small
connector PR(s) = user-visible upside for 0.14.0. See
[`RAIL_TEMPLATE.md`](RAIL_TEMPLATE.md) for the canonical shape every rail
follows.

Coworker agents (codex, factory-droid) and human contributors pick from
rails that have **`lane assignment`** matching them and **at least one
phase marked `builder-ready: yes`**.

> **Verification**: every row below references a verified doc path
> (existing on master) and/or a verified GitHub issue. Rails whose
> dedicated rail-doc has not yet been written are listed with their
> umbrella issue as the authoritative source; "Doc" reads
> `(umbrella only)` until a rail-doc lands.

## Index

| Rail | Doc | Umbrella | Lane | Open phases | Next action |
|---|---|---|---|---|---|
| `@INC` strictness | [`status/module_resolution.md`](status/module_resolution.md) | (closed; #8537 / #8544 / #8581 merged) | — | 0 | rail closed |
| Rust 1.95 / clippy cleanup | [`development/RUST_1_95_ROLLOUT.md`](../development/RUST_1_95_ROLLOUT.md) | #8508 closed → tracker #8584 | codex | 11 | #8561 (`collapsible_match`) |
| Strong clippy lints | [`development/STRONG_CLIPPY_LINTS_ROLLOUT.md`](../development/STRONG_CLIPPY_LINTS_ROLLOUT.md) | #8590 | codex | 1+ | #8601 (`manual_take`) |
| Codecov / evidence boundaries | [`ci/codecov-rollout.md`](../ci/codecov-rollout.md) | (doc only — file umbrella before pickup) | codex | 8 | Cov-1 (`codecov.yml` quiet + scope) |
| Module completion latency | (umbrella only) | #8514 | builder | 1 | TTL cache for prefix module scans |
| Perl-oracle subprocess env | (umbrella only) | #8551 | builder | 1+ | #8620 (ask-Perl subprocess seam inventory) |
| Literal require/import | (umbrella only) | #4280 | builder | 2 | #8639 (rail doc after #8618 spec closeout) |
| Real-workspace baseline | (umbrella only) | #7949 (suite), #7952 (editor-trust roadmap) | builder | 1+ | scope provider expectations for the baseline suite |
| File policy rollout | (umbrella only) | #8174 | factory-droid | 1+ | next PR in the 3–11 ladder (xtask / companion ledgers / gate wiring) |
| CI contributor UX | (umbrella only) | #4825 (sticky summary), #4826 (`just ci-doctor`) | builder | 2 | #4825 PR sticky comment |
| Freshness / issue-spec | (umbrella only) | #8546 | builder | 2 | #8619 (`cargo xtask freshness-check` implementation) |

## How rails are added or closed

- **New rail**: create `docs/<area>/<RAIL_NAME>.md` instantiating
  [`RAIL_TEMPLATE.md`](RAIL_TEMPLATE.md). File an umbrella issue using
  the user's tracking-issue body shape (current truth / required change /
  acceptance / claim boundary / do not combine). Add a row to this index
  in the same PR as the new rail-doc.
- **Rail closed**: leave the row in the index for one minor release with
  `Lane: —` and `Open phases: 0`, then prune. The closed `@INC` rail row
  above is the worked example.
- **Rail unverifiable**: do not list it. If the umbrella issue or doc
  path can't be confirmed via `gh api repos/EffortlessMetrics/perl-lsp/issues/<N>`
  or a worktree `Glob`, the row gets dropped from the index until it can.

## Claim boundary

Proves: every rail listed here references a doc that exists on master
and/or an issue number that resolves on GitHub. Does **not** prove:
anything about each rail's content, status, ladder ordering, or
readiness — those remain owned by the rail's own doc and umbrella issue.

## Do not combine

This index is structural scaffolding. Updates to it should be:

- **Pure index edits** (adding a new rail row, marking a rail closed,
  fixing a stale next-action) — single-purpose PR;
- **Never combined** with rail-content changes (substrate / connector /
  status table edits in the rail's own doc), with rail-PR-ladder row
  implementation, or with template changes to `RAIL_TEMPLATE.md`.
