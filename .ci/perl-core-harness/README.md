# Perl core harness baselines

This directory stores checked-in Perl core harness baselines.

Initial scaffold:

```bash
cargo xtask perl-core-harness discover \
  --perl-tree /path/to/prepared/perl5 \
  --host-perl perl \
  --profile base
```

Parse/compile-mode reports are written to:

```text
target/perl-core/reports/<profile>-parse.json
target/perl-core/reports/<profile>-compile.json
```

Selected execute-mode reports are written to:

```text
target/perl-core/reports/base-execute.json
```

The pinned advisory upstream smoke config is:

```text
.ci/perl-core-harness/upstream.toml
```

Preparation receipts are written to:

```text
target/perl-core/prepare/<ref>/prepare.json
```

Real-tree smoke receipts are written to:

```text
target/perl-core/smoke/<profile>/discovery.json
target/perl-core/smoke/<profile>/parse.json
target/perl-core/smoke/<profile>/compile.json
target/perl-core/smoke/<profile>/gap-map.json
target/perl-core/smoke/<profile>/smoke.json
```

Run the advisory integrated base lane against the pinned upstream ref:

```bash
just perl-core-integrated-base
```

Run the advisory integrated comp lane against the pinned upstream ref:

```bash
just perl-core-integrated-comp
```

Run the advisory integrated run lane against the pinned upstream ref:

```bash
just perl-core-integrated-run
```

Check the advisory real-upstream compile ratchets after smoke receipts exist:

```bash
just perl-core-upstream-compile-ratchet
```

Check the selected execute-base ratchet after the explicit execute receipt
exists:

```bash
just perl-core-execute-base-ratchet
```

The scheduled/manual `Perl Core Harness` workflow prepares the pinned upstream
tree, emits advisory `base`, `comp`, and `run` smoke receipts under
`target/perl-core/smoke/<profile>/`, and checks the real-upstream compile
reports against the checked-in upstream baselines.

Or run the smoke against a user-supplied prepared upstream Perl tree:

```bash
cargo xtask perl-core-harness smoke \
  --perl-tree /path/to/prepared/perl5 \
  --host-perl perl \
  --profile base \
  --modes parse,compile
```

The first checked-in ratchet is:

```text
base-compile-baseline.json
```

It covers the generated two-file base compile fixture only. It does not claim a
real upstream Perl baseline or runtime execution. The real-upstream advisory
compile ratchets are separate:

```text
upstream-base-compile-baseline.json
upstream-comp-compile-baseline.json
upstream-run-compile-baseline.json
```

The selected execute-base ratchet is:

```text
base-execute-baseline.json
```

It covers only the explicit selected `base/if.t`, `base/cond.t`, `base/num.t`,
`base/pat.t`, `base/translate.t`, and `base/while.t` execute receipt. It does
not claim profile-wide execute, execute-base conformance, or a broad runtime
model.

Update any baseline explicitly with `perl-core-harness baseline --accept` after
reviewing an intentional change. The real-tree smoke and ratchets are
manual/advisory and produce receipts plus a gap map only; selected execute
ratchets run only allowlisted Perl programs and still do not claim runtime
conformance or promote a PR gate.
