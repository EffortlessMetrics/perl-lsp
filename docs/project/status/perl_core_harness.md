# Perl Core Harness Status

The Perl core harness lane is scaffolded as an `xtask` control-plane command.
This status page is intentionally conservative until real upstream Perl tree
automation and runtime execution exist.

## Current Capability

| Capability | Status | Evidence |
|---|---|---|
| Prepared-tree discovery | Working | `cargo xtask perl-core-harness discover --perl-tree <prepared-perl5> --host-perl <perl> --profile base` writes discovery JSON |
| Parse mode | Working | `cargo xtask perl-core-harness run --mode parse --perl-tree <prepared-perl5> --host-perl <perl> --profile base` installs a `t/perl` compatibility wrapper, emits synthetic TAP through `perl-core-test-runner`, and writes a JSON report |
| Compile mode | Working | `cargo xtask perl-core-harness run --mode compile --perl-tree <prepared-perl5> --host-perl <perl> --profile base` parses clean files, lowers HIR, projects compile effects, fails on compile-effect dynamic boundaries, and writes a JSON report |
| Compile-mode base baseline | Ratcheted scaffold | `.ci/perl-core-harness/base-compile-baseline.json` protects the generated two-file base fixture against newly failing files, unknown buckets, bucket growth, and assertion regressions |
| Execute mode | Not implemented | Future runtime slice |
| Real upstream Perl tree | Not automated | User-supplied prepared trees are supported; clone/build/test_prep automation is future work |

## Claim Boundary

The current scaffold can enumerate tests from a user-supplied prepared Perl tree
and run them through the native Rust parser in parse mode or through the
parser-core HIR / compile-effect substrate in compile mode. Compile mode is a
receipt path for compiler workstream gaps, and the base compile baseline now
protects the generated fixture receipt shape and current generated base
behavior. It does not claim full compiler or runtime conformance, does not run
Perl programs, and does not yet cover a real upstream Perl checkout. Execute
mode remains fail-closed until the runtime slice lands.

The first receipt shape is:

```text
target/perl-core/discovery/<profile>.json
target/perl-core/reports/<profile>-parse.json
target/perl-core/reports/<profile>-compile.json
.ci/perl-core-harness/base-compile-baseline.json
```

The discovery receipt records the prepared tree, host Perl command, upstream
runner, staged profile, and normalized test paths. The parse and compile
reports record file-level TAP assertions and failure buckets such as
`parse_recovery`, `source_decode`, `compile_effect`, and `cli_switch`.
The compile baseline is deterministic and order-independent; improvements are
allowed, but baseline tightening requires an explicit `perl-core-harness
baseline --accept` update.
