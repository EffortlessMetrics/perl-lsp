# perl-lsp-launcher

Microcrate for Perl LSP startup parsing and launch configuration.

## When to use this crate

Use `perl-lsp-launcher` when you need the startup and launch-contract layer for
the Perl language server.

It is the right crate for:

- parsing `perl-lsp` CLI flags and transport options
- selecting feature profiles consistently across binaries and tooling
- emitting startup timing and launch metadata
- sharing the same launch contract with tests and editor integrations

## Public surface

- Parse CLI arguments for the `perl-lsp` binary.
- Normalize feature profile selection.
- Provide transport mode and startup metadata as a typed API.
- Expose BDD-grid-compatible feature catalog output for a selected profile.

This crate intentionally keeps startup concerns separate from `perl-lsp` runtime logic
so editors and tooling can share the same contract without duplicating parsing rules.

## Main types

- `LspArgs`: CLI argument parser for the binary entry point
- `TransportArgs` and `TransportMode`: stdio/socket transport selection
- `FeatureProfile`: selected capability profile
- `StartupTimer` and `StartupReport`: startup instrumentation
