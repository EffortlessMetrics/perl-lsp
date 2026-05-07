# ripr dogfood note: DevEx docs drift PR

## Target

- Repo: perl-lsp
- Area: xtask DevEx documentation drift guard
- PR: EffortlessMetrics/perl-lsp#8216
- Commit range: origin/master..working tree

## ripr run

- before: `target/ripr/dogfood/devex-docs-drift/before.repo-exposure.json`
- after: `target/ripr/dogfood/devex-docs-drift/after.repo-exposure.json`
- outcome: `target/ripr/dogfood/devex-docs-drift/outcome.md`
- agent verify: `target/ripr/dogfood/devex-docs-drift/agent-verify.json`
- agent receipt: `target/ripr/dogfood/devex-docs-drift/agent-receipt.json`

## What ripr found correctly

- `ripr check` completed and produced comparable before/after snapshots for
  the docs-drift guard slice.
- `ripr outcome` made the no-movement result explicit: changed `0`,
  improved `0`, resolved `0`, unchanged `116033`.
- `ripr agent verify` produced an advisory before/after summary from the saved
  snapshots.

## What ripr missed

- The focused guard validates contributor-facing toolchain facts against
  `rust-toolchain.toml`, including MSRV and pinned channel wording.
- The guard validates documented `just` and `cargo xtask` references in the
  contributor quickstart, first-PR guide, and command reference.
- During implementation, the guard caught two real drift cases:
  `just agent-*` was a wildcard rather than a real recipe, and
  `cargo xtask corpus --diagnose` omitted its legacy feature requirement.
- The agent artifacts did not surface the docs drift checker or documented
  command-reference scan as a useful DevEx seam.

## What was noisy

- This is another instance of the DevEx tooling/output oracle gap already
  tracked in EffortlessMetrics/ripr#511, so no duplicate upstream issue was
  opened.
- The before/after repo-exposure JSON files remained large for a small
  docs-and-xtask diff.

## Upstream ripr issues opened

- No new issue; tracked by EffortlessMetrics/ripr#511.
