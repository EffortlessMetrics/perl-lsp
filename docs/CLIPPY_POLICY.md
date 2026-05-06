# Clippy Policy

This workspace uses the Effortless Metrics strict Clippy policy as a governed engineering surface.
The policy is intentionally workspace-wide: production code, tests, examples, and xtask code all inherit the same panic-free defaults.

## Goals

- Forbid unchecked panic-family control flow (`unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, and `dbg!`).
- Prevent silent failure patterns such as ignored `Result`s, ignored futures, and discarded lock guards.
- Keep parser and protocol code safe around UTF-8 boundaries, byte indexes, string slicing, and unchecked indexing.
- Make suppression explicit with `#[expect(..., reason = "...")]` instead of broad or silent `#[allow(...)]` carveouts.
- Track future Rust 1.94 and 1.95 lint flips before the MSRV changes.

## Source of truth

The machine-readable lint ledger is [`policy/clippy-lints.toml`](../policy/clippy-lints.toml).
It records the active workspace lints, policy posture, and planned Rust-version ratchets.
The root [`Cargo.toml`](../Cargo.toml) must mirror every active lint in `[workspace.lints.rust]` and `[workspace.lints.clippy]`.

Temporary exceptions belong in [`policy/clippy-debt.toml`](../policy/clippy-debt.toml), not in `clippy.toml` test carveouts or crate-local lint weakening.
Debt entries must carry a lint, path, owner, reason, and expiry date.

## No test carveouts

Tests are part of the governed workspace surface. Do not add any of these `clippy.toml` options:

- `allow-unwrap-in-tests = true`
- `allow-expect-in-tests = true`
- `allow-panic-in-tests = true`
- `allow-indexing-slicing-in-tests = true`
- `allow-dbg-in-tests = true`

Prefer tests that return `Result` and use `?`, or use the repository's test helpers when an assertion needs richer diagnostics.

## Suppression style

Use narrow `#[expect(..., reason = "...")]` suppressions when a lint finding is intentionally retained.
The reason should explain why the local exception is safer than changing the code now.
Broad `#[allow(...)]` suppressions should be migrated into explicit debt or replaced with `#[expect]` as follow-up lint-policy PRs.

## Parser overlay

Because this workspace hosts Perl parser, lexer, semantic, LSP, and DAP code, the standard policy includes strict parser-safety rails:

- `clippy::string_slice`
- `clippy::indexing_slicing`
- `clippy::out_of_bounds_indexing`
- `clippy::char_indices_as_byte_indices`
- `clippy::sliced_string_as_bytes`
- `clippy::index_refutable_slice`

These lints protect UTF-8 and source-span boundaries where byte/character confusion can produce incorrect diagnostics or edits.

## Local gate

Run the policy gate with:

```bash
cargo xtask check-lint-policy
```

The gate verifies the MSRV ledger, workspace lint inheritance, active/planned lint consistency, absence of Clippy test carveouts, and required debt metadata.
