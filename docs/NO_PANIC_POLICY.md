# No-panic policy

`perl-lsp` treats unchecked panic paths as reliability debt. Parser, LSP, DAP,
workspace, release, and policy tooling should return structured errors or use
reviewed test helpers instead of collapsing `Result` or `Option` values.

## Current rollout status

The Rust 1.95 / 0.14.0 rollout targets an exact counted no-new-debt policy, but
that target is not active in this documentation PR. The rollout map lives in
[`docs/ci/perl-lsp-rust-1.95-rollout.md`](ci/perl-lsp-rust-1.95-rollout.md).

Current guardrails are split across:

- active panic-family Clippy bans in [`Cargo.toml`](../Cargo.toml), including
  `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, and
  `dbg_macro`;
- the governed lint ledger in
  [`policy/clippy-lints.toml`](../policy/clippy-lints.toml);
- the legacy Clippy test unwrap carveout in [`clippy.toml`](../clippy.toml),
  which is intentionally left for a dedicated follow-up PR.

## Target posture

The target no-panic lane is exact and counted:

1. Findings are keyed by path, family, selector kind, selector callee, snippet,
   and count.
2. Exact allowlist count slots are consumed first.
3. Baseline count slots are consumed second unless policy mode is blocking.
4. Anything left is reported as new debt.

Do not reset the no-panic baseline outside the dedicated baseline PR. A baseline
refresh may drop disappeared entries, but it must not absorb new panic-family debt
without an explicit reset.

## Test guidance

Tests should return `Result` or use repository helpers such as
`perl_tdd_support::must` and `perl_tdd_support::must_some`. The planned fallible
helper lane will add helper APIs for cases that need to convert `Option` or
`Result` values into `anyhow::Result` with context.
