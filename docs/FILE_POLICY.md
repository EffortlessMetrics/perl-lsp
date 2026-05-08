# File policy

Rust and `xtask` are the default implementation surfaces for `perl-lsp`.
Non-Rust files are allowed when their role is explicit, reviewed, and covered by
an appropriate policy gate or companion ledger.

## Rust 1.95 rollout target

The Rust 1.95 / `0.14.0` rollout map is
[`docs/ci/perl-lsp-rust-1.95-rollout.md`](ci/perl-lsp-rust-1.95-rollout.md). File
policy work is staged after the MSRV, lint, and no-panic foundation:

1. Add the non-Rust ledger without enforcement.
2. Add inventory, proposal, and checker commands.
3. Add companion ledgers for generated, executable, dependency, workflow,
   process, and network surfaces.
4. Wire the checks into existing CI gate receipts.

Legitimate non-Rust surfaces include Perl fixtures and corpus files,
tree-sitter/native parser bindings, VS Code extension files, GitHub workflows,
CI scripts, generated status artifacts, and release metadata. The policy target is
not "no non-Rust"; it is "non-Rust by receipt".
