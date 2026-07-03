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
target/perl-core/smoke/base/discovery.json
target/perl-core/smoke/base/parse.json
target/perl-core/smoke/base/compile.json
target/perl-core/smoke/base/gap-map.json
target/perl-core/smoke/base/smoke.json
```

Run the advisory integrated base lane against the pinned upstream ref:

```bash
just perl-core-integrated-base
```

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
real upstream Perl baseline or runtime execution. Update it explicitly with
`perl-core-harness baseline --accept` after reviewing an intentional change. The
real-tree smoke is manual/advisory and produces receipts plus a gap map only; it
does not run Perl programs, claim runtime conformance, or promote a PR gate.
