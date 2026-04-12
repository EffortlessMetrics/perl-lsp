# Fixture Workspaces

Four-scale fixture workspaces for the workspace & indexing scorecard (issue #4068).

| Scale | Directory | Files | Purpose |
|-------|-----------|-------|---------|
| small | `small/` | 10 | Smoke + SLO P95 baseline (committed) |
| medium | `medium/` | 100 | Typical project scale (committed) |
| large | `large/` | 1 000 | Enterprise scale (committed) |
| xlarge | `xlarge/` | 10 000 | Stress / limit discovery (generated on demand) |

## Regenerate xlarge

The xlarge fixture is **not committed** to avoid bloating git history.
Run the generation script to populate it locally:

```bash
bash scripts/gen-xlarge-workspace.sh
```

## Structure

Each workspace has:
- `lib/` — Perl module files (`.pm`) with a simple `package` + `sub new` + one data method
- `bin/` — Entry-point scripts (`.pl`) that `use` a module from `lib/`

## Tests

The scorecard test at `crates/perl-workspace-index/tests/workspace_scorecard.rs` verifies:
- small/medium/large directories exist with expected file counts
- xlarge directory exists (but does not count files, since it is generated)

## Benchmarks

```bash
# Initial index build at each scale
cargo bench -p perl-workspace-index --features workspace -- bench_initial_index
cargo bench -p perl-workspace-index --features workspace -- bench_batch_index_1000
cargo bench -p perl-workspace-index --features workspace -- bench_batch_index_10k
```
