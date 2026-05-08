# Clippy policy

`perl-lsp` treats Clippy as a governed engineering surface, not as a local taste file. The workspace policy is recorded in three places:

- `Cargo.toml` contains the active `[workspace.lints]` block inherited by member crates.
- `policy/clippy-lints.toml` is the machine-readable ledger for active, debt, tracked, and planned Rust-version lint flips.
- `policy/clippy-debt.toml` records temporary, expiring debt instead of weakening the global policy silently.

## Workspace posture

The policy applies to production code and tests. The current active Cargo lint block remains intentionally small; broader guardrail lints are tracked in `policy/clippy-lints.toml` until the relevant cleanup PRs can activate them without bundling behavior changes into the policy gate. Tests should return `Result` or use repository test helpers such as `perl_tdd_support::must` and `perl_tdd_support::must_some`.

The workspace still carries Clippy's legacy test unwrap carveout in `clippy.toml`. That carveout is recorded as expiring debt in `policy/clippy-debt.toml` so this control-plane PR can add governance without also rewriting unrelated tests.

The tracked lint set covers five guardrail families:

1. **Panic-free code**: no unchecked `Result`/`Option` collapse, panic macros, `todo!`, `unimplemented!`, or `unreachable!` paths.
2. **AST and UTF-8 safety**: parser and LSP boundary code must avoid unchecked string slicing, byte/character index confusion, and unchecked indexing.
3. **Silent-failure prevention**: ignored futures, ignored `must_use` values, discarded errors, and lossy line iteration are denied.
4. **Async, memory, numeric, and file/process footguns**: concurrency and parser correctness hazards are denied or warned according to the ledger.
5. **Suppression governance**: broad or unexplained suppressions are rejected. Prefer narrow `#[expect(..., reason = "...")]` receipts.

## Suppression style

Use `#[expect]` only when the lint is correct but the local exception is intentional and reviewed:

```rust
#[expect(
    clippy::indexing_slicing,
    reason = "Generated parser table access is bounded by table construction invariants."
)]
fn generated_table_lookup(table: &[usize], index: usize) -> usize {
    table[index]
}
```

Do not use a silent `#[allow]`. If a lint needs repo-wide temporary treatment, add a scoped entry to `policy/clippy-debt.toml` with `lint`, `path`, `owner`, `reason`, and `expires`.

## Planned Rust upgrades

The ledger tracks planned Rust 1.94 and 1.95 flips before the workspace MSRV moves. `cargo xtask check-lint-policy` verifies that planned lints are present in the ledger and not activated ahead of the MSRV bump.

The current workspace remains on the MSRV recorded in `Cargo.toml` and `policy/clippy-lints.toml`; the Rust 1.93 toolchain ratchet is intentionally left to a dedicated follow-up lane because toolchain changes affect CI, documentation, and release policy together.


## Rust 1.95 rollout map

The Rust 1.95 / `0.14.0` rollout is mapped in
[`docs/ci/perl-lsp-rust-1.95-rollout.md`](ci/perl-lsp-rust-1.95-rollout.md).
For that rollout, Clippy changes stay split across dedicated PRs:

- the compatibility spike runs Rust 1.95 without changing MSRV or lint policy;
- the MSRV/toolchain PR updates `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml`,
  and workflow toolchain references without removing the test unwrap carveout;
- the Rust compiler lint-floor PR activates tracked rustc lints;
- the Clippy ratchet PR measures and then activates clean or cheaply fixable
  Rust 1.94/1.95 lints;
- the test-carveout PR removes `allow-unwrap-in-tests = true` and adds a fallible
  helper path before broader test migration work.

The current mismatch is intentional and visible: `policy/clippy-lints.toml` still
allows test carveouts while `Cargo.toml` denies `unwrap_used` and `expect_used` at
the workspace level. Do not resolve that mismatch in the MSRV bump PR.

## Local check

Run the policy gate before changing lint configuration:

```bash
cargo xtask check-lint-policy
```

The gate checks lint inheritance, active Cargo lint levels, tracked lint metadata, planned upgrade ledger entries, and required debt metadata.
