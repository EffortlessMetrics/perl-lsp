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

## Real-Workspace Resource Bridge

The real-workspace latency harness includes an opt-in bridge receipt that opens
every readable Perl fixture file for Mojolicious, Dancer2, and Catalyst, then
records each fixture's file/line/byte inventory plus best-effort child-process
RSS after project load:

```bash
cargo test -p perl-lsp-rs --test real_project_latency real_project_memory_resource_receipt --profile agent --locked -- --include-ignored --nocapture --test-threads=1
```

This is a project-shaped memory/resource receipt, not a plateau gate. It does
not set heap ceilings, replace `cargo xtask metrics memory`, or promote any
provider support claim by itself.

## Failure Triage

When a plateau gate fails, file a **Memory Regression** issue and include the
plateau JSON/CSV/server log artifact, `tail_growth_kb`,
`median_tail_slope_kb_per_file`, lifecycle (`close-only`, `close+delete`, or
another named scenario), nonzero `MemoryStateSnapshot` or
`RuntimePressureSnapshot` counters, and suspected state owner. Patch work should
start from a narrow failing regression, not from a broad leak hunt.

## Closeout

The retained-state memory incident is closed as an active lane. The closeout
evidence map and response playbook live in
`docs/large-workspaces/MEMORY_CONTROL_CLOSEOUT.md`.

## Interpretation Rules

- Close-only churn may retain workspace-index entries for files that still exist.
- Close+delete churn must remove file-backed workspace-index entries.
- RSS is allowed to warm up and hold allocator arenas; the plateau gate tracks
  tail growth and median tail slope rather than exact return-to-baseline.
- Runtime receipts are comparable evidence. Logs remain supporting artifacts,
  not the source of trend truth.
