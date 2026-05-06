# Clippy policy

`perl-lsp` treats Clippy as a governed engineering surface, not as a local taste file. The root `Cargo.toml` owns the active workspace lint baseline, while `policy/clippy-lints.toml` records active lints, staged platform-policy lints, and planned Rust 1.94 and 1.95 flips.

## Invariants

- The workspace MSRV is Rust 1.93 and must match `policy/clippy-lints.toml`.
- Workspace crates inherit the root lint policy with `[lints] workspace = true`.
- Production code and tests share the same panic-free baseline: no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, or `dbg!` carveouts.
- `clippy.toml` is reserved for repo-local thresholds and disallowed-method/type policy. It must not enable test carveouts such as `allow-unwrap-in-tests`.
- Suppressions should use narrow `#[expect(..., reason = "...")]` receipts rather than broad `#[allow(...)]` carveouts.

## Active lint families

The policy ledger covers active and staged lint families:

- panic-free production and tests;
- AST, parser, UTF-8, string slicing, and index safety;
- silent-failure prevention for ignored futures, must-use values, locks, `Result::ok`, and `map_err`;
- async and concurrency footguns;
- unsafe and memory-adjacent footguns;
- numeric correctness warnings and denies;
- file, process, and path hazards;
- API/trait correctness; and
- reviewability lints that reduce allocation noise and clarify control flow.

## Debt and future flips

Temporary exceptions belong in `policy/clippy-debt.toml` with `lint`, `path`, `owner`, `reason`, and `expires`. Staged lints are not yet active in `Cargo.toml`; they become active only after the debt ledger is retired or narrowed. Planned Rust 1.94 and 1.95 lint flips are tracked in `policy/clippy-lints.toml` before the MSRV bump so upgrades are explicit ratchets rather than surprise cleanup waves.

Run the policy gate with:

```bash
cargo xtask check-lint-policy
```
