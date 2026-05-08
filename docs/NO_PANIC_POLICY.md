# No-panic policy

`perl-lsp` treats panic-family calls as policy-controlled debt. The Rust 1.95 /
0.14.0 rollout does not reset any baseline in its documentation PR; it records
the target posture and reserves implementation for dedicated policy PRs.

## Current state

- Root Clippy hard bans already deny `panic`, `todo`, `unimplemented`,
  `unwrap_used`, `expect_used`, and `dbg_macro` through workspace lints.
- Additional panic-family lints are tracked in `policy/clippy-lints.toml`.
- The exact no-panic allowlist/baseline is missing or incomplete, so the
  rollout must harden identity before creating a no-new-debt baseline.
- `clippy.toml` still allows unwraps in tests; removing that carveout is a
  separate Clippy policy PR, not part of the rollout map PR.

## Target state

The target no-panic model is exact and counted:

```text
path + family + selector_kind + selector_callee + snippet + count
```

Matching order:

1. consume exact allowlist count slots;
2. then consume baseline count slots unless mode is `blocking`;
3. report anything left as new debt.

Allowlist entries should include the reviewed snippet and count. Baseline reset
is allowed only in the dedicated no-panic baseline PR after exact identity is in
place.

## Rollout link

See [Rust 1.95 / 0.14.0 rollout map](ci/perl-lsp-rust-1.95-rollout.md) for the
ordered PR ladder and validation gates.
