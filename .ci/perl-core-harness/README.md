# Perl core harness baselines

This directory is reserved for checked-in Perl core harness baselines once run
receipts exist.

Initial scaffold:

```bash
cargo xtask perl-core-harness discover \
  --perl-tree /path/to/prepared/perl5 \
  --host-perl perl \
  --profile base
```

Parse-mode reports are written to:

```text
target/perl-core/reports/<profile>-parse.json
```

Future baselines will live here, for example:

```text
base-parse-baseline.json
base-compile-baseline.json
base-execute-baseline.json
```
