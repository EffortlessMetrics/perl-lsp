# Perl Core Harness Status

The Perl core harness lane is scaffolded as an `xtask` control-plane command.
This status page is intentionally conservative until broader runtime execution
exists.

For the current green/yellow/red burndown and next PR order, see the
[Perl Core Harness Burndown](perl_core_harness_burndown.md).
For the post-harness runtime and provider-promotion phase, see the
[Compiler-Backed LSP Burndown](compiler_backed_lsp_burndown.md).

## Current Capability

| Capability | Status | Evidence |
|---|---|---|
| Prepared-tree discovery | Working | `cargo xtask perl-core-harness discover --perl-tree <prepared-perl5> --host-perl <perl> --profile base` writes discovery JSON |
| Parse mode | Working | `cargo xtask perl-core-harness run --mode parse --perl-tree <prepared-perl5> --host-perl <perl> --profile base` installs a `t/perl` compatibility wrapper, emits synthetic TAP through `perl-core-test-runner`, and writes a JSON report |
| Compile mode | Working | `cargo xtask perl-core-harness run --mode compile --perl-tree <prepared-perl5> --host-perl <perl> --profile base` parses clean files, lowers HIR, projects compile effects, fails on compile-effect dynamic boundaries, and writes a JSON report |
| Compile-mode base baseline | Ratcheted scaffold | `.ci/perl-core-harness/base-compile-baseline.json` protects the generated two-file base fixture against newly failing files, unknown buckets, bucket growth, and assertion regressions |
| Real upstream Perl base smoke | Advisory integrated | `perl-core-harness prepare --ref <pinned-ref>` prepares upstream Perl under `target/perl-core`, then `perl-core-harness smoke --profile base --modes parse,compile` writes discovery, parse, compile, gap-map, and smoke receipts |
| Real upstream Perl comp smoke | Advisory integrated | `perl-core-harness smoke --profile comp --modes parse,compile` uses the same receipt path for the `comp` profile, and run 28711942840 recorded 25 discovered files, parse 18/25, compile 8/25, and bucketed `parse_recovery` / `compile_effect` gaps |
| Real upstream Perl run smoke | Advisory integrated | `perl-core-harness smoke --profile run --modes parse,compile` uses the same receipt path for the `run` profile, and run 28726563803 recorded 28 discovered files, parse 18/28, compile 1/28, and bucketed `parse_recovery` / `compile_effect` gaps |
| Real upstream compile ratchets | Advisory integrated | `.ci/perl-core-harness/upstream-{base,comp,run}-compile-baseline.json` ratchets real upstream compile reports for schema/profile/mode drift, newly failing files, unexpected failures, bucket growth, unknown/unbucketed failures, and assertion regressions |
| Harness orchestration crate | Extracted | `crates/perl-core-harness` owns discovery, prepare, run, baseline, smoke, and gap-map orchestration; `xtask` remains CLI dispatch glue |
| Execute-one | Scaffolded | `perl-core-harness run --mode execute --profile base --test base/if.t` runs the first allowlisted upstream `base/if.t` path through `perl-core-test-runner`, emits real TAP, and writes an execute report |
| Execute-base | Ratcheted selected subset | `perl-core-harness run --mode execute --profile base --test base/if.t --test base/cond.t --test base/num.t --test base/pat.t --test base/translate.t --test base/while.t` runs explicit allowlisted `base` files and writes `target/perl-core/reports/base-execute.json`; `.ci/perl-core-harness/base-execute-baseline.json` ratchets 6/6 files and 325/325 TAP assertions, while profile-wide execute remains unsupported |
| Runtime bucket model | Published | The burndown board maps supported runtime buckets to workstreams, selected-file entry rules, candidate `base/*.t` files, and receipt-integrity rules before runtime burn-down starts |
| Provider promotion gates | Published | The burndown board names the evidence required before any compiler-backed provider cutover: harness receipts, semantic scorecard or oracle proof, shadow comparison, fallback / rollback strategy, and real-workspace receipts |
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
generation. Execute-one proves the allowlisted `base/if.t` receipt path, and
execute-base now has a ratcheted selected-subset receipt for allowlisted
`base/*.t` files. The lane does not claim full compiler or runtime conformance,
does not promote provider behavior, and is not a required PR or merge-queue
gate. Profile-wide execute remains fail-closed until runtime buckets are
reduced enough to widen safely. Provider-promotion gates are documented in the
burndown board and require provider-specific semantic scorecard or oracle
evidence, shadow comparison, fallback or rollback strategy, and real-workspace
receipts before any user-visible LSP cutover. The runtime bucket model is
documented in the burndown board and is constrained to bucket names already
understood by the shared harness receipt types; finer-grained runtime categories
can be added only with matching receipt classification support.

The first receipt shape is:

```text
target/perl-core/discovery/<profile>.json
target/perl-core/reports/<profile>-parse.json
target/perl-core/reports/<profile>-compile.json
target/perl-core/reports/base-execute.json
target/perl-core/prepare/<ref>/prepare.json
target/perl-core/smoke/<profile>/smoke.json
target/perl-core/smoke/<profile>/gap-map.json
.ci/perl-core-harness/base-compile-baseline.json
.ci/perl-core-harness/upstream-<profile>-compile-baseline.json
.ci/perl-core-harness/base-execute-baseline.json
```

The discovery receipt records the prepared tree, host Perl command, upstream
runner, staged profile, and normalized test paths. The parse and compile
reports record file-level TAP assertions and failure buckets such as
`parse_recovery`, `source_decode`, `compile_effect`, and `cli_switch`.
The baseline files are deterministic and order-independent; improvements are
allowed, but baseline tightening requires an explicit `perl-core-harness
baseline --accept` update. The generated fixture baseline, real-upstream
advisory compile baselines, and selected execute-base baseline are separate
files. The smoke summary records the discovery, parse, and compile report paths
plus per-mode totals and buckets. The gap map groups failures by bucket,
workstream, impacted LSP surface, and first failure per bucket. Bucketed
parse/compile failures are preserved as gap data; missing reports, unbucketed
failures, and `unknown` buckets fail the smoke receipt.

The current execute receipt commands are explicit and allowlisted:

```bash
cargo xtask perl-core-harness run \
  --perl-tree <prepared-perl5> \
  --host-perl perl \
  --runner test \
  --mode execute \
  --profile base \
  --test base/if.t

cargo xtask perl-core-harness run \
  --perl-tree <prepared-perl5> \
  --host-perl perl \
  --runner test \
  --mode execute \
  --profile base \
  --test base/if.t \
  --test base/cond.t \
  --test base/num.t \
  --test base/pat.t \
  --test base/translate.t \
  --test base/while.t
```
