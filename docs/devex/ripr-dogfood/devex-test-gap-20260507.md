# ripr dogfood note: DevEx test-gap PR 1

## Target

- Repo: perl-lsp
- Area: xtask DevEx planner routing
- PR: EffortlessMetrics/perl-lsp#8210
- Commit range: origin/master..working tree

## ripr run

- before: `target/ripr/dogfood/devex/before-routing.repo-exposure.json`
- after: `target/ripr/dogfood/devex/after-routing.repo-exposure.json`
- outcome: `target/ripr/dogfood/devex/routing-outcome.md`
- agent brief before: `target/ripr/dogfood/devex/agent-brief.before.json`
- agent brief after: `target/ripr/dogfood/devex/agent-brief.after.json`
- agent verify: `target/ripr/dogfood/devex/routing-agent-verify.json`
- agent receipt: `target/ripr/dogfood/devex/routing-agent-receipt.json`

## What ripr found correctly

- `ripr check` completed on the full repo and produced comparable before/after
  repo-exposure JSON artifacts.
- `ripr outcome` made the no-movement result explicit: moved `0`,
  regressed `0`, new `0`, removed `0`, unchanged `115920`.
- `ripr agent verify` produced a machine-readable advisory summary from the
  saved before/after snapshots.

## What ripr missed

- The focused test added a table-driven routing fixture matrix for
  `xtask/src/tasks/devex_plan.rs`, but `ripr outcome` reported no improved or
  resolved seams.
- The agent brief and verify artifacts did not surface the DevEx routing seam
  or the new table-driven test as a useful discriminator.
- Because verify had no changed DevEx seam, `ripr agent receipt` could only be
  exercised against an unrelated unchanged seam, so the receipt was not useful
  PR evidence for this dogfood slice.

## What was noisy

- `ripr pilot` was useful as a smoke run, but its top recommendation targeted
  archived heredoc parser code rather than the active DevEx changed area.
- The requested `ripr agent status` command is not available in installed
  `ripr` 0.4.0; the available agent surfaces are `brief`, `packet`, `verify`,
  and `receipt`. This dogfood run used `agent brief` instead.
- The repo-exposure artifacts are large, about 365 MB each, which makes the
  loop heavier than the focused DevEx test itself.

## Upstream ripr issues opened

- EffortlessMetrics/ripr#508:
  `dogfood(perl-lsp): table-driven DevEx routing test did not move repo exposure`
