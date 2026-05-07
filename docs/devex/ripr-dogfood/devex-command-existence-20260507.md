# ripr dogfood note: DevEx command existence PR

## Target

- Repo: perl-lsp
- Area: xtask DevEx planner proof command validity
- PR: EffortlessMetrics/perl-lsp#8214
- Commit range: origin/master..working tree

## ripr run

- before: `target/ripr/dogfood/devex-command-existence/before.repo-exposure.json`
- after: `target/ripr/dogfood/devex-command-existence/after.repo-exposure.json`
- outcome: `target/ripr/dogfood/devex-command-existence/outcome.md`
- agent verify: `target/ripr/dogfood/devex-command-existence/agent-verify.json`
- agent receipt: `target/ripr/dogfood/devex-command-existence/agent-receipt.json`

## What ripr found correctly

- `ripr check` completed and produced comparable before/after snapshots for
  the command-existence guard slice.
- `ripr outcome` made the no-movement result explicit: changed `0`,
  improved `0`, resolved `0`, unchanged `116033`.
- `ripr agent verify` produced an advisory before/after summary from the saved
  snapshots.

## What ripr missed

- The focused test validates every planner-emitted proof command against a
  real local command surface: `just` recipes from the justfile, `cargo xtask`
  subcommands from clap metadata, or the explicitly allowed `git diff --check`.
- The test would catch a stale proof recommendation such as `just status-updat`
  or `cargo xtask check-memory-retained-owner-drift2`.
- The agent artifacts did not surface the command router or local-command
  inventory as a useful DevEx seam.

## What was noisy

- This is another instance of the DevEx tooling/output oracle gap already
  tracked in EffortlessMetrics/ripr#511, so no duplicate upstream issue was
  opened.
- The before/after repo-exposure JSON files remained large for a small
  test-only diff.

## Upstream ripr issues opened

- No new issue; tracked by EffortlessMetrics/ripr#511.
