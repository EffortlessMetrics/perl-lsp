# Clippy Policy

This workspace uses the Effortless Metrics strict Clippy policy as a governed engineering surface, not a local taste file.

## Baseline

The root `Cargo.toml` owns the active workspace lint block. Every workspace member must inherit it with:

```toml
[lints]
workspace = true
```

The active profile starts the workspace panic-free ratchet: production code and tests may not introduce unchecked panic-family constructs such as `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, or `dbg!`. The broader standard profile, including `unreachable!`, parser-safe string and slice handling, silent-failure prevention, async lock hygiene, file/process footgun checks, and suppression governance, is tracked in `policy/clippy-lints.toml` as planned flips so follow-up PRs can promote each class with explicit debt.

## Policy ledger

`policy/clippy-lints.toml` is the machine-readable policy ledger. It records:

- the workspace MSRV;
- policy posture for panic-free tests and suppression style;
- active Rust and Clippy lints with class and reason; and
- planned Rust 1.93 hardening work plus Rust 1.94 and 1.95 lint flips before each promotion.

`policy/clippy-debt.toml` is the only place for temporary lint debt. Debt entries must include the lint, path, owner, reason, and expiry. Expired debt fails `cargo xtask check-lint-policy`.

## Suppressions

Do not use broad `#[allow(...)]` suppressions. If a narrow exception is unavoidable, use `#[expect(..., reason = "...")]` at the smallest possible scope and keep the reason reviewable. Test carveouts such as `allow-unwrap-in-tests = true` are forbidden.

## Local gate

Run the policy gate before sending lint-policy changes for review:

```bash
cargo xtask check-lint-policy
```

The gate verifies MSRV alignment, workspace lint inheritance, active/planned lint consistency, forbidden Clippy test carveouts, and non-expired debt metadata.
