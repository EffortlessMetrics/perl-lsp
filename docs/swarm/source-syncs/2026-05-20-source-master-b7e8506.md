# Source Sync Receipt: 2026-05-20 source master b7e8506

## Sync Identity

| Field | Value |
|---|---|
| Source repo | `EffortlessMetrics/perl-lsp` |
| Source branch | `master` |
| Source SHA | `b7e8506c4dd3977bcb0e2f1445f8b82a5b98ab15` |
| Source tip PR | `EffortlessMetrics/perl-lsp#9549` |
| Swarm repo | `EffortlessMetrics/perl-lsp-swarm` |
| Swarm target branch | `main` |
| Swarm sync PR | This PR; merge SHA is recorded by GitHub after merge |

## Included Source Changes

- `fix(dead-code): correct is_always_false docstring and rename postfix modifier constant (#9009) (#9549)`

## Claim Boundary

This is a content-lineage sync from the publishing repo into the swarm
development repo. It does not make `perl-lsp-swarm` a full-history mirror of
`perl-lsp`, and it does not repair GitHub contributor graph provenance.

`perl-lsp` remains the commit-history and release-lineage authority until a
separate history-preserving mirror/fork decision replaces the current content
sync model.

## Freeze Boundary

After this sync, routine feature, test, refactor, provider, parser, diagnostics,
and trust-lane development should target `EffortlessMetrics/perl-lsp-swarm`.

`EffortlessMetrics/perl-lsp` should receive only release/publish/signing work,
deliberate release-lineage syncs, and explicitly routed emergency release fixes.

## Verification

```bash
rtk git fetch git@github.com:EffortlessMetrics/perl-lsp.git master:refs/remotes/source/master
rtk git rev-parse source/master
rtk bash -lc 'MIN_FREE_GB=20 MAX_USED_PCT=95 CARGO_LOCK_WAIT=900 ./scripts/cargo-safe xtask fmt'
rtk bash -lc 'MIN_FREE_GB=20 MAX_USED_PCT=95 CARGO_LOCK_WAIT=900 ./scripts/cargo-safe test -p perl-parser --test dead_code_detector --profile agent --locked -- --nocapture'
rtk bash -lc 'MIN_FREE_GB=20 MAX_USED_PCT=95 CARGO_LOCK_WAIT=900 ./scripts/cargo-safe check -p perl-parser --all-targets --profile agent --locked'
rtk bash -lc 'MIN_FREE_GB=20 MAX_USED_PCT=95 CARGO_LOCK_WAIT=900 ./scripts/cargo-safe clippy -p perl-parser --profile agent --locked -- -D warnings -A missing_docs'
rtk git diff --check
rtk bash -lc './scripts/storage-doctor'
```
