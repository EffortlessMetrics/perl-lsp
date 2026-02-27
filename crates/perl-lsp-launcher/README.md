# perl-lsp-launcher

Microcrate for Perl LSP startup parsing and launch configuration.

## Purpose

- Parse CLI arguments for the `perl-lsp` binary.
- Normalize feature profile selection.
- Provide transport mode and startup metadata as a typed API.
- Expose BDD-grid-compatible feature catalog output for a selected profile.

This crate intentionally keeps startup concerns separate from `perl-lsp` runtime logic
so editors and tooling can share the same contract without duplicating parsing rules.
