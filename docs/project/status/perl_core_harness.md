# Perl Core Harness Status

The Perl core harness lane is scaffolded as an `xtask` control-plane command.
This status page is intentionally conservative until run receipts and baselines
exist.

## Current Capability

| Capability | Status | Evidence |
|---|---|---|
| Prepared-tree discovery | Scaffolded | `cargo xtask perl-core-harness discover --perl-tree <prepared-perl5> --host-perl <perl> --profile base` writes discovery JSON |
| Parse mode | Not implemented | Future runner crate slice |
| Compile mode | Not implemented | Future HIR/compile-effects slice |
| Execute mode | Not implemented | Future runtime slice |
| Baselines | Not implemented | Future `.ci/perl-core-harness/*.json` slice |

## Claim Boundary

The current scaffold does not run Perl core tests through the Rust parser,
compiler, or runtime. It only asks upstream Perl's own harness to enumerate test
files from a user-supplied prepared Perl tree.

The first receipt shape is:

```text
target/perl-core/discovery/<profile>.json
```

It records the prepared tree, host Perl command, upstream runner, staged profile,
and normalized test paths.
