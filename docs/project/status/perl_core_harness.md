# Perl Core Harness Status

The Perl core harness lane is scaffolded as an `xtask` control-plane command.
This status page is intentionally conservative until real upstream Perl tree
automation and runtime execution exist.

For the current green/yellow/red burndown and next PR order, see the
[Perl Core Harness Burndown](perl_core_harness_burndown.md).

## Current Capability

| Capability | Status | Evidence |
|---|---|---|
| Prepared-tree discovery | Working | `cargo xtask perl-core-harness discover --perl-tree <prepared-perl5> --host-perl <perl> --profile base` writes discovery JSON |
| Parse mode | Working | `cargo xtask perl-core-harness run --mode parse --perl-tree <prepared-perl5> --host-perl <perl> --profile base` installs a `t/perl` compatibility wrapper, emits synthetic TAP through `perl-core-test-runner`, and writes a JSON report |
| Compile mode | Working | `cargo xtask perl-core-harness run --mode compile --perl-tree <prepared-perl5> --host-perl <perl> --profile base` parses clean files, lowers HIR, projects compile effects, fails on compile-effect dynamic boundaries, and writes a JSON report |
| Compile-mode base baseline | Ratcheted scaffold | `.ci/perl-core-harness/base-compile-baseline.json` protects the generated two-file base fixture against newly failing files, unknown buckets, bucket growth, and assertion regressions |
| Real upstream Perl base smoke | Advisory integrated | `perl-core-harness prepare --ref <pinned-ref>` prepares upstream Perl under `target/perl-core`, then `perl-core-harness smoke --profile base --modes parse,compile` writes discovery, parse, compile, gap-map, and smoke receipts |
| Real upstream Perl comp smoke | Advisory integrated | `perl-core-harness smoke --profile comp --modes parse,compile` uses the same receipt path for the `comp` profile, and run 28711942840 recorded 25 discovered files, parse 18/25, compile 8/25, and bucketed `parse_recovery` / `compile_effect` gaps |
| Real upstream Perl run smoke | Advisory integrated | `perl-core-harness smoke --profile run --modes parse,compile` uses the same receipt path for the `run` profile; the first real upstream receipt still needs to be recorded after the advisory workflow runs |
| Harness orchestration crate | Extracted | `crates/perl-core-harness` owns discovery, prepare, run, baseline, smoke, and gap-map orchestration; `xtask` remains CLI dispatch glue |
| Execute mode | Not implemented | Future runtime slice |
| Upstream Perl tree preparation | Linux advisory | Clone/configure/test_prep automation is Linux-only in this slice; Windows/macOS preparation is future work |

## Claim Boundary

The current scaffold can enumerate tests from a user-supplied prepared Perl tree
and run them through the native Rust parser in parse mode or through the
parser-core HIR / compile-effect substrate in compile mode. Compile mode is a
receipt path for compiler workstream gaps, and the base compile baseline now
protects the generated fixture receipt shape and current generated base
behavior. The advisory real-tree smoke verifies that actual upstream `base`,
`comp`, and `run` profile files from a pinned prepared Perl tree can flow
through discovery, parse, compile, smoke-summary, and gap-map receipt
generation. It does not claim full
compiler or runtime conformance, does not run Perl programs as runtime code,
does not promote provider behavior, and is not a required PR or merge-queue
gate. Execute mode remains fail-closed until the runtime slice lands.

The first receipt shape is:

```text
target/perl-core/discovery/<profile>.json
target/perl-core/reports/<profile>-parse.json
target/perl-core/reports/<profile>-compile.json
target/perl-core/prepare/<ref>/prepare.json
target/perl-core/smoke/base/smoke.json
target/perl-core/smoke/base/gap-map.json
.ci/perl-core-harness/base-compile-baseline.json
```

The discovery receipt records the prepared tree, host Perl command, upstream
runner, staged profile, and normalized test paths. The parse and compile
reports record file-level TAP assertions and failure buckets such as
`parse_recovery`, `source_decode`, `compile_effect`, and `cli_switch`.
The compile baseline is deterministic and order-independent; improvements are
allowed, but baseline tightening requires an explicit `perl-core-harness
baseline --accept` update. The smoke summary records the discovery, parse, and
compile report paths plus per-mode totals and buckets. The gap map groups
failures by bucket, workstream, impacted LSP surface, and first failure per
bucket. Bucketed parse/compile failures are preserved as gap data; missing
reports, unbucketed failures, and `unknown` buckets fail the smoke receipt.
