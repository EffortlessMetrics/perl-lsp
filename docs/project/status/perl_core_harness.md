# Perl Core Harness Status

The Perl core harness lane is scaffolded as an `xtask` control-plane command.
This status page is intentionally conservative until run receipts and baselines
exist.

## Current Capability

| Capability | Status | Evidence |
|---|---|---|
| Prepared-tree discovery | Scaffolded | `cargo xtask perl-core-harness discover --perl-tree <prepared-perl5> --host-perl <perl> --profile base` writes discovery JSON |
| Parse mode | Scaffolded | `cargo xtask perl-core-harness run --mode parse --perl-tree <prepared-perl5> --host-perl <perl> --profile base` installs a `t/perl` compatibility wrapper, emits synthetic TAP through `perl-core-test-runner`, and writes a JSON report |
| Compile mode | Not implemented | Future HIR/compile-effects slice |
| Execute mode | Not implemented | Future runtime slice |
| Baselines | Not implemented | Future `.ci/perl-core-harness/*.json` slice |

## Claim Boundary

The current scaffold can enumerate tests from a user-supplied prepared Perl tree
and run them through the native Rust parser in parse mode. It does not claim
compiler or runtime conformance yet. Compile and execute modes remain
fail-closed until those slices land.

The first receipt shape is:

```text
target/perl-core/discovery/<profile>.json
target/perl-core/reports/<profile>-parse.json
```

The discovery receipt records the prepared tree, host Perl command, upstream
runner, staged profile, and normalized test paths. The parse report records
file-level TAP assertions and failure buckets such as `parse_recovery`,
`source_decode`, and `cli_switch`.
