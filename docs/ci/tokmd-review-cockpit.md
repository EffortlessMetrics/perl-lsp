# tokmd Review Cockpit

`tokmd` runs in review mode on pull requests and publishes a reviewer map tailored to the
`perl-lsp` workspace layout.

## Workflow behavior

The CI workflow in `.github/workflows/tokmd.yml` runs:

```bash
tokmd review \
  --base origin/<base-ref> \
  --head HEAD \
  --out-dir .tokmd/review \
  --config tokmd.review.toml \
  --gate advisory \
  --near-dup \
  --detail-functions
```

Artifacts are written to `.tokmd/review/` and include:

- `comment.md`
- `review-map.md`
- `review-map.json`
- `cockpit.json`
- `analysis-risk.json`
- `duplication.json`
- `complexity.json`
- `evidence.json`
- `manifest.json`

## perl-lsp area model

`tokmd.review.toml` maps changed files into weighted review areas:

- `parser`: parser/lexer/token/tree-sitter crates (highest weight)
- `lsp`: LSP server and providers
- `workspace`: indexing, module resolution, and semantic analysis
- `dap`: debugger adapter surface
- `ci`: workflows, xtask, and scripts
- `docs`: docs and contributor-facing markdown

Each area can suggest targeted verification commands so reviewers can run the smallest
high-signal checks first.

## Gate posture

`tokmd-review-gate.toml` is advisory and checks reviewer-map quality signals instead of
repository-size metrics:

- review path exists
- risk score visibility
- complexity findings visibility
- duplication findings visibility
- evidence gap visibility
- health grade not `F`

Warnings are surfaced in review artifacts and PR comments without blocking merges.
