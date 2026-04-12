# xlarge fixture workspace (10 000 files)

This fixture workspace is **generated on demand** and is not committed to the repository
(committing 10 000 files would bloat the git history).

## Generate

```bash
bash scripts/gen-xlarge-workspace.sh
```

The script creates:
- `lib/A/` through `lib/I/` — 1 100 `.pm` modules per subdirectory (9 900 lib files)
- `bin/` — 100 entry-point `.pl` scripts

## Purpose

The xlarge fixture is used for:
- Benchmarking initial index build time at 10k-file scale (`perl-workspace-index/benches/`)
- Discovering file-count ceilings and degradation thresholds
- Validating that `IndexResourceLimits::max_files = 10_000` is the actual ceiling

## SLO target

Initial index build P95 < 5 000 ms (from `perl-workspace-index-slo`).
Run `cargo bench -p perl-workspace-index --features workspace -- bench_batch_index_10k` to measure.
