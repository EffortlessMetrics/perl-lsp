# Semantic fixture scorecard (compiler-lite harness)

This harness seeds the semantic fixture surface for compiler-lite behavior, without requiring a
`perl-semantic-facts` crate.

## Run

```bash
cargo xtask semantic-scorecard
cargo xtask semantic-scorecard --json
```

## Source of truth

- Manifest: `crates/perl-workspace-index/tests/fixtures/semantic_scorecard/fixtures.json`
- Loader + deterministic checks: `xtask/src/tasks/semantic_scorecard.rs`

## Adding future semantic-fact scenarios

1. Add a fixture row to `fixtures` in sorted `id` order.
2. Add/update metric entries under `metrics`.
3. Keep unsupported metrics as `status: not_implemented` or `baseline_pending` with `value: null`.
4. Run `cargo test -p xtask` and `cargo xtask semantic-scorecard --json`.

This guarantees stable, deterministic scorecard output while semantic facts are incrementally
implemented.
