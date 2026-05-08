# No-panic policy

`perl-lsp` treats panic-family calls as policy debt because parser, LSP, DAP,
and CI-control code should report structured failure instead of aborting. The
active Clippy bans live in the workspace lint block and the broader policy plan
lives in `policy/clippy-lints.toml`.

## Rust 1.95 rollout target

The Rust 1.95 / `0.14.0` rollout target is an exact, counted, no-new-debt
no-panic gate. The rollout map is
[`docs/ci/perl-lsp-rust-1.95-rollout.md`](ci/perl-lsp-rust-1.95-rollout.md).

The intended sequence is:

1. Harden finding identity before any baseline reset.
2. Match by `path + family + selector_kind + selector_callee + snippet`.
3. Require allowlist entries to include the exact snippet and count.
4. Consume exact allowlist counts first.
5. Consume baseline counts second unless the gate is running in blocking mode.
6. Report anything left as new debt.

## Baseline discipline

The baseline PR is the only rollout step allowed to reset the no-panic baseline.
Baseline refreshes after that should drop disappeared findings only. New panic
family findings must be fixed, narrowly allowlisted with count and expiry, or
handled through an explicit baseline reset PR.
