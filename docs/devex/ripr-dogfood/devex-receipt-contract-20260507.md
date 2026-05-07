# ripr dogfood note: DevEx receipt contract PR

## Target

- Repo: perl-lsp
- Area: xtask DevEx local proof receipt contract
- PR: EffortlessMetrics/perl-lsp#8213
- Commit range: origin/master..working tree

## ripr run

- before: `target/ripr/dogfood/devex-receipt-contract/before.repo-exposure.json`
- after: `target/ripr/dogfood/devex-receipt-contract/after.repo-exposure.json`
- outcome: `target/ripr/dogfood/devex-receipt-contract/outcome.md`
- agent brief after: `target/ripr/dogfood/devex-receipt-contract/agent-brief.after.json`
- agent verify: `target/ripr/dogfood/devex-receipt-contract/agent-verify.json`
- agent receipt: `target/ripr/dogfood/devex-receipt-contract/agent-receipt.json`

## What ripr found correctly

- `ripr check` completed and produced comparable before/after snapshots for
  the receipt-contract test slice.
- `ripr outcome` made the no-movement result explicit: changed `0`,
  improved `0`, resolved `0`, unchanged `115926`.
- `ripr agent verify` produced an advisory before/after summary from the saved
  snapshots.

## What ripr missed

- The focused test locks the JSON handoff contract for `DevexReceipt`,
  including top-level fields, proof command object fields, surfaces, hints,
  worktree cleanliness, and generated timestamp.
- The agent artifacts did not surface the receipt serializer or contract test
  as a useful DevEx seam.
- Because verify had no changed receipt seam, `ripr agent receipt` had to be
  exercised against an unrelated unchanged seam.

## What was noisy

- This repeated the output/serialization oracle gap already captured in
  EffortlessMetrics/ripr#511, so no duplicate upstream issue was opened.
- The before/after repo-exposure JSON files remained about 365 MB each for a
  small test-only diff.

## Upstream ripr issues opened

- No new issue; tracked by EffortlessMetrics/ripr#511.
