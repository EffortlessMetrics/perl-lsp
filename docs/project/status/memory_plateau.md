# Memory Plateau Receipts

> Human-owned baseline summary. Runtime receipts are generated under
> `target/memory/receipts/` by `cargo xtask metrics memory`.

## Current Baseline

Source run: `CI (Nightly)` workflow dispatch `25444427692` on
`e58ab60848bae119c182740c482948de0fd357c4`.

| Scenario | Files | Changes/file | Tail growth KB | Median tail slope KB/file | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| `lsp_doc_churn_delete` | 500 | 10 | 152 | 0.690 | passed |
| `lsp_workspace_symbol_churn_delete` | 300 | 10 | 872 | 4.764 | passed |
| `workspace_index_remove_reindex_cycle` | n/a | n/a | n/a | n/a | covered by `memory_leak_regression` |

## Receipt Command

```bash
cargo xtask metrics memory \
  --workload-json target/memory/nightly-doc-churn.json \
  --plateau-json target/memory/nightly-doc-churn.plateau.json \
  --scenario lsp_doc_churn_delete \
  --receipt target/memory/receipts/nightly-doc-churn.receipt.json \
  --commit "$GITHUB_SHA" \
  --event push \
  --markdown
```

The receipt is registered as `memory-plateau` in
`.ci/receipts/registry.toml` and validates through:

```bash
cargo xtask gate-receipts validate target/memory/receipts/nightly-doc-churn.receipt.json
```

## Trend Command

Render the current plateau trend table from local plateau summaries, registered
receipts, and the committed baseline:

```bash
cargo xtask memory-trends render \
  --input-dir target/memory \
  --output docs/project/status/memory_plateau_trends.md
```

Use `--history-dir <path>` to include archived receipt directories. The command
is evidence-only: it does not run a memory workload or participate in PR gates
unless a workflow invokes it explicitly.

## Interpretation Rules

- Close-only churn may retain workspace-index entries for files that still exist.
- Close+delete churn must remove file-backed workspace-index entries.
- RSS is allowed to warm up and hold allocator arenas; the plateau gate tracks
  tail growth and median tail slope rather than exact return-to-baseline.
- Runtime receipts are comparable evidence. Logs remain supporting artifacts,
  not the source of trend truth.
