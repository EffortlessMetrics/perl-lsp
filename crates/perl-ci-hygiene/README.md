# perl-ci-hygiene

Internal CI and release-hygiene command runner for the `perl-lsp` workspace.

## Problem it solves

The repository used to depend on a growing set of shell scripts for CI checks,
release prep, and local hygiene workflows. This crate consolidates those
operations into a native Rust CLI so the same checks can run predictably across
platforms.

## What it does

`perl-ci-hygiene` exposes workspace maintenance commands such as:

- TODO and documentation hygiene checks
- version-sync and parser-matrix checks
- missing-docs and ignored-test ratchets
- fatal-construct and lock-safety audits
- release-prep and packaging-adjacent validation helpers

## Usage

This crate is an internal tool and is normally invoked through `just` recipes
or `xtask`, not by end users directly.

```bash
cargo run -p perl-ci-hygiene -- check-version-sync
```

## Workspace role

Supports repository automation and CI policy enforcement. This crate is not
part of the published end-user product surface.

## License

MIT OR Apache-2.0
