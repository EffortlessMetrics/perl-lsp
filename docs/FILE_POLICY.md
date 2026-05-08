# File policy

Rust and `xtask` are the default implementation surfaces for this repository.
Non-Rust files are valid when they are intentional, owned, and covered by an
allowlist or companion policy.

## Legitimate non-Rust surfaces

The Rust 1.95 / 0.14.0 rollout identifies these expected non-Rust surfaces:

- Perl fixtures and corpus data;
- tree-sitter C/native parser bindings;
- VS Code extension assets and TypeScript surfaces;
- GitHub workflows;
- CI scripts;
- generated docs/status artifacts;
- release metadata.

## Target ledger

The target non-Rust allowlist entries include:

```text
id
glob
kind
language
surface
classification
owner
reason
covered_by
created
review_after
```

Broad globs also require `broad_glob_reason`. Companion ledgers should cover
generated files, executable files, dependency surfaces, workflow behavior,
process execution, and network access.

The rollout map records the sequence: add the ledger first, add inventory and
proposal tooling next, then wire blocking checks into gate receipts only after
the allowlist is reviewable.

See [Rust 1.95 / 0.14.0 rollout map](ci/perl-lsp-rust-1.95-rollout.md).
