# LSP Churn Memory Reproduction

Investigation date: 2026-05-06

This harness reproduces open/change/close document churn against `perl-lsp`
and records RSS samples as JSON or CSV. For plateau gates, run it with
`DELETE_AFTER_CLOSE=1` so each closed file is also removed from the workspace;
close-only churn preserves workspace index entries for files that still exist
on disk. This is investigation and regression infrastructure: it measures
whether process memory plateaus after warmup, not whether RSS returns exactly
to baseline.

Build the server:

```bash
cargo build --release -p perl-lsp-rs
```

Run document churn:

```bash
N_FILES=500 \
N_CHANGES=10 \
DO_WORKSPACE_SYMBOL=0 \
DELETE_AFTER_CLOSE=1 \
BINARY=./target/release/perl-lsp \
python3 scripts/repro_lsp_storm.py \
  --json-out target/memory/doc_churn.json \
  --csv-out target/memory/doc_churn.csv

python3 scripts/assert_rss_plateau.py target/memory/doc_churn.json
```

Run churn plus workspace-symbol pressure:

```bash
N_FILES=300 \
N_CHANGES=10 \
DO_WORKSPACE_SYMBOL=1 \
DELETE_AFTER_CLOSE=1 \
BINARY=./target/release/perl-lsp \
python3 scripts/repro_lsp_storm.py \
  --json-out target/memory/workspace_symbol.json \
  --csv-out target/memory/workspace_symbol.csv

python3 scripts/assert_rss_plateau.py target/memory/workspace_symbol.json
```

Use `--summary-out target/memory/<name>.plateau.json` to persist the plateau
decision as a CI artifact. Use `--settle-seconds` to control how long the
harness waits after the last `didClose` or watched-file delete before taking
the final RSS sample.

Convert a plateau summary into a registered receipt:

```bash
cargo xtask metrics memory \
  --workload-json target/memory/doc_churn.json \
  --plateau-json target/memory/doc_churn.plateau.json \
  --scenario lsp_doc_churn_delete \
  --receipt target/memory/receipts/doc_churn.receipt.json \
  --commit "$(git rev-parse HEAD)" \
  --event local \
  --markdown
```

Validate it against the receipt registry:

```bash
cargo xtask gate-receipts validate target/memory/receipts/doc_churn.receipt.json
```

CI runs this in two tiers:

- PR smoke: `75` files, `5` changes, document churn only, loose plateau gate,
  artifacts retained for 14 days.
- Nightly or `ci:memory` label: `500` files document churn plus `300` files
  with workspace-symbol pressure, strict plateau gate, artifacts retained for
  30 days.

Acceptance should focus on shape:

- RSS can rise during warmup.
- RSS should flatten after warmup.
- Repeated closed-file churn should not ratchet linearly by file count.
- Workspace-symbol pressure should plateau or be materially reduced compared
  with the pre-fix baseline.

Allocator arenas and `HashMap` capacity can remain reserved, so exact return
to initial RSS is not required.

The harness is best supported on Linux and macOS. Linux uses `/proc` for RSS;
other Unix-like systems fall back to `ps`. Windows process and URI handling is
not part of this guard yet.
