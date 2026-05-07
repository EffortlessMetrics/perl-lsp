# ripr dogfood note: DevEx golden output PR

## Target

- Repo: perl-lsp
- Area: xtask DevEx output contracts
- PR: EffortlessMetrics/perl-lsp#8211
- Commit range: origin/master..working tree

## ripr run

- before: `target/ripr/dogfood/devex-goldens/before.repo-exposure.json`
- after: `target/ripr/dogfood/devex-goldens/after.repo-exposure.json`
- outcome: `target/ripr/dogfood/devex-goldens/outcome.md`
- agent brief after: `target/ripr/dogfood/devex-goldens/agent-brief.after.json`
- agent verify: `target/ripr/dogfood/devex-goldens/agent-verify.json`
- agent receipt: `target/ripr/dogfood/devex-goldens/agent-receipt.json`

## What ripr found correctly

- `ripr check` produced comparable before/after repo-exposure snapshots for
  the golden-output test slice.
- `ripr outcome` made the no-movement result explicit: changed `0`,
  improved `0`, resolved `0`, unchanged `115920`.
- `ripr agent verify` produced machine-readable before/after evidence from
  the saved snapshots.

## What ripr missed

- The focused tests added exact output goldens for `render_plan`,
  receipt JSON rendering, `render_cockpit`, and `render_pr_body`, but `ripr`
  did not surface those renderer or serializer seams.
- Searching the agent artifacts for `devex_plan`, `render_plan`,
  `render_cockpit`, and `render_pr_body` produced no useful DevEx seam.
- Because verify had no changed DevEx output seam, `ripr agent receipt` again
  had to be exercised against an unrelated unchanged seam.

## What was noisy

- The artifact size remained high: before/after repo-exposure JSON files were
  about 365 MB each for a small test-only diff.
- The agent receipt command needs a seam id, but the verify output had no
  changed seam relevant to the test diff, making receipt automation awkward for
  this class of dogfood run.

## Upstream ripr issues opened

- EffortlessMetrics/ripr#511:
  `dogfood(perl-lsp): DevEx output-contract goldens did not move repo exposure`
