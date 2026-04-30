# tokmd PR Review Cockpit

`tokmd` runs in `mode: review` for pull requests targeting `master`.

## Generated artifacts

Each run writes a review packet in `.tokmd/review/`:

- `comment.md`
- `review-map.md`
- `review-map.json`
- `cockpit.json`
- `analysis-risk.json`
- `duplication.json`
- `complexity.json`
- `evidence.json`
- `manifest.json`

## perl-lsp workspace-oriented review areas

The review config (`tokmd.review.toml`) maps changed files into architecture areas:

- `parser`: parser, lexer, token, tree-sitter crates
- `lsp`: LSP host, core, and formatter integration crates
- `workspace`: workspace index, semantic analyzer, and module crates
- `dap`: debug adapter crate
- `ci`: workflow and xtask automation
- `docs`: user and contributor documentation

Each area has a weight and suggested verification command. This makes the review map prioritize files by architectural risk, not only by line count.

## Expected review output shape

The review comment and map should surface:

- risk and health summary
- contract/change-surface changes (API, CLI, schema)
- complexity and near-duplicate findings
- evidence gaps (for example missing mutation or diff-coverage evidence)
- prioritized `P1/P2/P3` review path with file-level reasons and suggested checks

## Local reproduction

```bash
tokmd review \
  --base origin/master \
  --head HEAD \
  --out-dir .tokmd/review \
  --config tokmd.review.toml \
  --near-dup \
  --detail-functions \
  --gate advisory
```
