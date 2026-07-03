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

The first checked-in ratchet is:

```text
base-compile-baseline.json
```

It covers the generated two-file base compile fixture only. It does not claim a
real upstream Perl checkout or runtime execution. Update it explicitly with
`perl-core-harness baseline --accept` after reviewing an intentional change.
